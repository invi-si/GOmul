# CPU attribution using deterministic callback transcripts

This phase uses the saved normal (5,940,941 steps) and user-designated skill
(9,072,858 steps) fixtures on the Mac Android AVD. No Fold connection or manual
scene positioning is needed. Attribution builds are never timing authorities.
The original uninstrumented control binaries and their hashes are retained.

## Exact observations

`arm32_cpu/exact-counts`, forwarded by `wie-core-arm/cpu-exact-counts`, is an
untimed diagnostic feature. It records thread-local observations only while a
replay CPU segment is executing. Architectural CPU serialization/state is unchanged.
The feature enables transcript verification, including checks for unexpected
writes and exact step counts. `transcript_counts` executes each whole fixture
twice, verifies all segments, checks instruction totals against the capture, and
requires identical counter snapshots. Both real fixtures pass.

Counter definitions:

- Guest PC and raw opcode frequencies include ISA, instruction attempts and
  decoded classes. A Thumb long branch is one step with two fetch halfwords.
- Memory operations count adapter accesses after alignment handling, by width and
  fetch/data role. Faulting attempts are included; host restoration is excluded.
- Register reads/writes count explicit get/set and Index/IndexMut API calls.
  IndexMut includes read-modify-write access, so these are not host load/store
  instruction counts. Direct internal mode reads and invalid-CPSR old-value reads
  have separate counters. Core run-boundary bookkeeping is included.
- Taken/not-taken counts cover encoded conditional branches. Encoded unconditional
  B/BX/BL/BLX transfers have a separate count; computed transfers through ordinary
  PC-writing instructions are represented in opcode and PC-write counts, not
  silently treated as encoded branch instructions.
- CPU run lengths and exit reasons come from the verified transcript segments.
  Exact SVC records retain category, LR and SPSR; summaries group these by reason.

| Exact observation | Normal | Skill |
|---|---:|---:|
| Distinct guest PCs | 9,047 | 10,136 |
| Top 10 PC share | 11.20% | 8.54% |
| Register-access API calls / step | 9.92 | 9.81 |
| PC/CPSR share of register API calls | 80.66% | 81.32% |
| Conditional branches taken | 235,455 | 315,637 |
| Conditional branches not taken | 203,164 | 667,763 |
| Data reads, 8 / 16 / 32-bit | 127,569 / 203,696 / 1,486,030 | 290,010 / 274,976 / 2,061,203 |
| Data writes, 8 / 16 / 32-bit | 164 / 157,136 / 575,749 | 148 / 232,467 / 663,910 |

Normal's dominant Thumb classes are SpXfer, ImmOp, Shifted, AluOp and CondBranch.
Skill's are SpXfer, ImmOp, AluOp, CondBranch and Shifted. Both contain two ARM
instructions per SVC: SingleXferI and BranchEx. Thus the work is overwhelmingly
Thumb, but not exclusively so. High access counts are not themselves evidence of
removable time; native source/disassembly attribution is the next gate.

The independent counter loop regression test passes, as do the vendor's 35 tests,
92 ARM-core tests with counters enabled, and a new adapter test covering fetch vs
data widths and exclusion of host restoration. The additional adapter test was
run separately after the full suite. Full real transcript verification also passes
on Android. Clippy completes with warnings; no clean-warning claim is made.

## Native sampling

`transcript_native_profile` runs the same fixtures repeatedly with interpreter
counters, tracing and dirty verification compiled out. It emits no timing score.
It still validates segment CPU/exit and complete final memory every round.
The profiling executable has optimized code, debug information and an ELF build
ID. External simpleperf samples use `cpu-clock:u`; elapsed sampling time is not
used to judge a performance candidate.

`analyze-transcript-samples.py` checks the recorded ELF build ID and selects PCs
inside the exact `Arm32CpuEngine::run` symbol range. Restoration, validation,
loading, kernel work and out-of-line helpers outside that range are excluded
explicitly. Source leaf locations and inclusive inline functions are reported;
inclusive shares overlap, and sampling skid prevents treating a line share as a
precise removable cost. Raw traces, memory fixtures and guest-address reports stay
in the ignored `release-artifacts/cpu-transcript/attribution/` directory.

## Native results

Both recordings completed all requested replays and final validations. The normal
fixture ran 600 times; the skill fixture ran 200 times, at a requested 997 Hz.
Normal recorded/exported 211,953 samples; skill recorded/exported 81,328. Both
reported zero losses, with matching ELF build IDs and exact export totals.

| Native observations within `run()` | Normal | Skill |
|---|---:|---:|
| Selected samples | 86,254 | 41,497 |
| memory_error inline chain | 16.52% | 16.12% |
| RegFile Index reads | 6.15% | 6.16% |
| get_page inline chain | 5.60% | 5.86% |
| build_flags inline chain | 5.08% | 5.77% |
| RegFile explicit get | 3.56% | 3.91% |

These are overlapping inclusive sample shares, not a wall-time breakdown or
estimates of removable time. Normal and skill hit essentially the same common
machinery. The already-rejected fault-slot change was not retried merely because
memory_error remained hot; no CPSR-local, mode-inline, NOP or block-cache retry was
made.

Disassembly at the normal register-index site showed a bank-table load, an operand
index load, and an indexed register load; variable low-register operands also
retained a bounds check. Much of the hottest register-index site is the **banked
SP** access in SpXfer, so its complete sample share cannot be assigned to removable
unbanked-register work. Saved baseline source and the exact ELF remain alongside
the local artifacts to preserve source-line interpretation after edits.

## One generic candidate, then the gate

The candidate bypassed bank-table lookup for current-bank Index/IndexMut accesses
to r0–r7, PC and CPSR. These slots are identical across all valid ARM modes. It
retained the original path for banked registers, including SP and SPSR, and used
no unsafe access, new cache, guest-PC special case, timer or scheduling change.
It was compile-time experimental and never enabled in the gameplay APK.

It passed 36 vendor tests and 93 core tests with exact counters, including a new
mode-switch/banked-register regression. Both complete real transcripts passed
both verification binaries on Android before each timing sweep.

`transcript_gate.py` is now the reusable gate. It pushes specified binaries and
fixtures, validates **both workloads before any timing**, runs warmed ABBA/ABBA
rounds, saves artifact hashes and raw logs, and reports threshold and process-median
consistency separately. Its conservative live-validation screen requires at least
5% median improvement on both fixtures plus non-overlapping process medians.

| Sweep | Normal reduction | Skill reduction | Pass screen? |
|---|---:|---:|---|
| Initial | 8.60% | 7.18% | No: inconsistent process results |
| Confirmation | 6.49% | 4.74% | No: skill below threshold and inconsistent results |

Each sweep used the same frozen original control binary and 12 measured rounds
per variant/workload, with two warmups per process. All timed runs reported
instrumentation absent and passed CPU/exit and complete final-memory checks.
The prototype was **archived and removed from active source**, not promoted as an
optimization. Its patch and built binaries remain private under attribution/
for reproducibility; the bank-switch regression test remains in the suite.

## Timing limitation and next step

A material environment shift occurred: the exact unchanged control SHA-256
`1c7dee8deabccb6944de11d63c70d45289be4d07f57723e7ac181a951cbabcfc`
ran near 140–220 ms instead of the earlier 28–41 ms, with large variations even
within a process. Thus the earlier absolute times must not be mixed with these
sweeps. A freshly compiled control with all new hooks disabled differed from the
frozen control by +1.76% normal / +0.78% skill in an adjacent baseline check;
this does not explain the much larger environment shift.

Read-only host checks did not establish its cause. No guest or host CPU scheduling
policy, emulator cap, timer delay or pacing setting was changed to improve scores.
Positive medians are encouraging but insufficient to claim a repeatable win under
these conditions. The next step is to establish a stable AVD control distribution
under consistent host conditions before reconsidering this archived candidate or
another CPU patch. The saved fixtures eliminate further manual navigation for
that work. No gameplay FPS or input-latency improvement is claimed. No physical
Fold testing was performed or requested.

## Reproducing the tools

With the existing NDK linker configured:

```sh
cargo build --release -p wie-cpu-bench --example transcript_counts \
  --target aarch64-linux-android \
  --features exact-counts,experimental-thumb-inline,experimental-thumb-table

CARGO_PROFILE_RELEASE_DEBUG=2 cargo rustc --release -p wie-cpu-bench \
  --example transcript_native_profile --target aarch64-linux-android \
  --features replay,experimental-thumb-inline,experimental-thumb-table \
  -- -C link-arg=-Wl,--build-id=sha1
```

`transcript_counts CAPTURE` emits exact JSON counts after two identical verified
replays. `transcript_native_profile CAPTURE ROUNDS` is the external native-sampler
workload; it never emits a timing score. See `transcript_gate.py --help` for the
four binaries/two fixtures required by the permanent gate. Pause competing game
execution before sampling or timing. All artifacts remain local and ignored by Git.


## Follow-up: control regime stabilized

The control-only follow-up identified host application activity/priority as a
practical control variable. An activation request to the same macOS AVD process
restored approximately 28/41 ms immediately; explicit activation before every
subprocess then held process-median variation near 1.1% across six alternating
normal/skill pairs. The frozen binary was unchanged. See
[control-stability-avd.md](control-stability-avd.md) for exact results and limits.
The archived candidate was not retested during this follow-up. Future macOS AVD
runs of `transcript_gate.py` require `--avd-host-pid` so host activation and thread
state are explicit, rather than silently mixing regimes.

## Follow-up: archived register candidate retested

Two host-activated sweeps retained all samples and passed exact transcript
correctness. Median CPU reductions were 8.15%/5.42% and 9.24%/7.79%
(normal/skill), but initial normal control instability and confirmation skill
process overlap prevented promotion under the full acceptance criteria.
Candidate remains archived; no source or APK changes. See
[unbanked-register-avd-retest.md](unbanked-register-avd-retest.md) for all process
medians, limitations and the final decision.
