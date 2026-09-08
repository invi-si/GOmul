# Deterministic callback CPU transcripts

Status: both normal and skill gameplay transcripts captured and validated on the
AVD. The existing Thumb-block candidate regresses both workloads and is rejected
for activation. The reusable paired CPU workload set is complete. Use the Mac
Android AVD for this phase, as requested. These results must not be labelled as
Fold 5 measurements. No CPU optimization or timer/scheduling change is included.

## What is recorded

The diagnostic `cpu-transcript-capture` feature arms the next WIPI timer callback.
KTF/LGT hooks bracket the actual callback invocation, including its error return.
Only CPU runs under that callback's scoped identity become transcript segments.
Cancelled registrations still never enter the callback and cannot claim capture.

One complete mapped-memory base is retained at callback entry. Each segment has:

- Full CPU state before and after, including CPSR and banked registers.
- Original end address, instruction budget, actual step attempts, and exit/SVC/fault.
- Memory changes before the run and changes produced by the run.

Before-run changes are **environmental transitions**, not purported recordings of
individual SVC implementations. They include host/JVM operations, context switches,
and other tasks running while the callback is suspended. Those operations are not
executed during replay. Final environmental changes and callback return status are
also recorded. The final restored caller CPU state is a recorded transition; the
CPU-produced state at every segment exit is independently compared.

Capture-only dirty-page tracking covers CPU byte/halfword/word stores, host range
writes, and mapping. Deltas coalesce changed 64-byte lanes. The base is stored once;
changed-page shadows are shared across segments. No whole-heap copy is stored per
run. Oversized captures are discarded explicitly (100,000 runs / 128 MiB delta
payload). Capturing and serializing can cause a substantial pause: this is not a
low-overhead gameplay profiler, and its elapsed time is not a performance result.

## Validation and timing are separate builds

`cpu-replay` supports transcripts without capture or dirty tracking.
`cpu-transcript-verify` adds dirty tracking and exact step-count checks but does
not enable input tracing. Verification compares every segment's exit, full CPU,
expected changed pages, and all actually dirtied pages, catching unexpected writes
before a later environmental restore could hide them. Every replay build also
compares **every mapped page** before applying the final environmental transition.

The CLI separates prepare, execute, validate, and finish. Only execute is timed;
its small wrapper/exit bookkeeping and clock-call overhead remain included. It
prints a separate empty-clock-pair measurement; that number is not subtracted.
Instrumented binaries print `timing_eligible=false`, determined from the actual
core dependency's unified features. Timing binaries omit capture, verification,
throughput, profiling, and input tracing.

Replays reset guest state each round, perform two warmup rounds, and retain the
Thumb cache across rounds. Existing byte guards remain responsible for detecting
restored or modified code. Restoring environmental memory and validating expected
memory changes necessarily affects host cache conditions. This is a controlled
CPU comparison, not a direct measurement of live callback wall time or FPS.

## Build and capture

With the existing Android toolchain environment set:

```sh
sh tools/input-trace/build.sh --transcript
sh tools/cpu-bench/build-transcript-android.sh
```

The capture APK is the separate GOmul Trace application. The second command emits
`control-verify`, `blocks-verify`, `control`, and `blocks` under the ignored
`release-artifacts/cpu-transcript/` directory. Both use the currently retained
Thumb inline/table features. Only the blocks binaries enable the existing
experimental block path; the gameplay capture APK uses the control interpreter.

After navigating manually to the desired scene, create `cpu-transcript.request`
in the active game's save root (the same directory used by the existing
`cpu-replay.request` mechanism). This arms exactly one callback. Completion writes
`cpu-transcript-TIMESTAMP.json` and `cpu-transcript-status.txt` there. Record whether
this was ordinary gameplay or a skill action; accept only successful legitimate
callback captures. Do not assume they contain the earlier *average* 6.19M/8.54M
step totals: the capture reports its own exact work.

All captures contain private guest code, memory, and potentially save/identity
data. Keep them outside tracked source, never attach them to a public PR/release.

## Comparison gate

For each normal and skill fixture, run both verification binaries first:

```sh
./control-verify CAPTURE.json 1
./blocks-verify CAPTURE.json 1
```

Any segment/state/step mismatch rejects the candidate. Only then alternate warmed
control/blocks timing runs on the AVD using the exact same fixture files. Preserve
raw outputs, build features, binary and fixture hashes. Report run-to-run variation
and total CPU time for both workloads; do not retain a mixed or 1–2% result. A
repeatable improvement around 5% or greater on both is the gate for later live
validation, not a guarantee of a physical-device or gameplay benefit.

A fixture can also be replayed on the Mac with:

```sh
cargo run --release -p wie-cpu-bench --example transcript \
  --features replay,experimental-thumb-inline,experimental-thumb-table -- CAPTURE.json 7
```

## Correctness checks so far

- 83 default ARM-core tests and 92 replay-verification tests pass.
- 108 ARM-core tests pass with capture and the experimental block path enabled.
- 46 WIPI-C tests pass with callback capture hooks enabled.
- Six new transcript tests cover SVC/host/mapping transitions, flags and banked
  registers, budgets/branches/faults, self-modifying writes, ARM/Thumb transitions,
  IRQ/FIQ boundaries, unexpected writes/malformed deltas, callback ownership and
  async suspension/error completion. The existing block differential suite remains
  in place. There are six test functions; several exercise multiple cases.
- Both real transcripts pass control and block verification. No speedup is
  demonstrated; the block candidate is slower on both (details below).

The capture APK is installed in `emulator-5554` as GOmul Trace. All four
Android replay executables have been built and copied to
`/data/local/tmp/gomul-transcript/`. The control executable starts and reports
its usage correctly; both fixtures execute and validate successfully. Binary hashes are
retained locally in `release-artifacts/cpu-transcript/build-sha256.txt`.

## Normal callback result — 2026-09-09

Captured callback 1098 returned successfully: 7,684 segments, 5,940,941 exact step
attempts, 7,497 SVC exits, 2,047,936 bytes of delta payload. The JSON occupies
31,563,786 bytes. Both verification binaries passed all segment CPU/exit/step and
memory checks, plus full final memory comparison, across two warmup replays and
one reported replay.

The gameplay app was backgrounded using its existing pause behavior for timing.
Standalone AVD runs used order control/blocks/blocks/control, repeated twice.
Each process warmed twice, then measured three rounds: 12 measured rounds per
variant, four process medians each. All timed runs reported instrumentation absent
and passed the uninstrumented CPU/exit and full final-memory validation gates.

| AVD CPU replay | Control | Existing Thumb blocks |
|---|---:|---:|
| Median CPU time | 27.709 ms | 35.837 ms |
| Measured range | 27.410–29.120 ms | 35.394–46.328 ms |
| Process median range | 27.574–27.954 ms | 35.427–36.002 ms |
| Guest steps/sec | 214.4 million | 165.8 million |

The block candidate is **29.3% slower** by median CPU time. It fails the acceptance
gate on normal gameplay and remains disabled. Instrumented verification times
must not be used to judge performance: their extra tracking changes relative
costs significantly. These are CPU-transcript results, not live gameplay FPS or
Fold 5 measurements. The skill result below completes the paired workload set;
no new CPU patch is justified by this result alone.

Raw fixture, SHA-256, validation outputs, eight timing logs and summary are local
only under `release-artifacts/cpu-transcript/normal/`. The game session has been
brought back to the foreground for the user's skill setup.

## Skill callback and final decision — 2026-09-09

During the user's repeated-skill session, callback 3078 returned successfully:
8,519 CPU segments, 9,072,858 exact step attempts, 8,012 SVC exits, 2,327,872 bytes
of delta payload, and 33,079,771 serialized JSON bytes. This is 52.7% more guest
work than the captured normal callback. These are individual fixtures, not the
older multi-callback averages.

Both control and blocks passed every segment's full CPU, exit, step-count and
memory checks, plus the complete final mapped-memory comparison. The same AVD,
binaries, backgrounded gameplay app, two warmups per process, ABBA/ABBA order and
12 measured rounds per variant were used. Instrumented verification timings were
excluded from the performance comparison.

| AVD skill CPU replay | Control | Existing Thumb blocks |
|---|---:|---:|
| Median CPU time | 40.716 ms | 53.848 ms |
| Measured range | 40.390–41.046 ms | 52.848–54.838 ms |
| Process median range | 40.522–40.833 ms | 52.887–54.370 ms |
| Guest steps/sec | 222.8 million | 168.5 million |

The skill regression is **32.3%** by median CPU time, consistent across process
medians. Together with the **29.3% normal regression**, this clearly rejects the
existing block candidate under these controlled workloads. It remains default-off;
no CPU optimization was enabled and no timer or scheduling behavior was changed.
These results do not prove that all block execution approaches are unhelpful; they
show that this implementation is not a winner. No expansion of it is warranted
from the present evidence.

Live FPS and visible input latency were not measured in this stage. No claim is
made that gameplay lag is resolved, or that standalone replay timings equal live
callback wall times. The practical result is a deterministic correctness and
performance gate for future CPU changes, without repeated manual game navigation.
Keep the current interpreter; use both saved fixtures for any future candidate and
advance only a repeatable winner to live validation. Further manual gameplay
benchmarking of this losing candidate is unnecessary.

Skill raw data and its SHA-256 are local only under
`release-artifacts/cpu-transcript/skill/`. The game was returned to the foreground
after measurement and ADB was returned to non-root mode.
