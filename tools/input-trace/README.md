# Input/task-poll diagnostic experiment

Opt-in diagnostics, originally developed from release/gomul d255d1aa.
Package: `local.wie.nativeapp.trace`. Keep the normal package and its data intact.
This is a diagnostic build, not a performance fix or a public release.

## Build

Use the Android SDK/NDK and Rust setup described in the launcher build instructions.
Build `wie-android` for `aarch64-linux-android` in release mode with features
`input-trace,experimental-thumb-inline,experimental-thumb-table`. Set the Android
NDK linker as in `wie-android/build.sh`; copy the resulting library into the
launcher's `jniLibs/arm64-v8a`, generate notices, and run `-PgomulTrace :app:assembleRelease` (or use `sh tools/input-trace/build.sh`).
The opt-in Gradle trace configuration is non-debuggable, shell-profileable,
and signed with the development key. It is for local diagnostics only.
Omitting `input-trace` compiles out the recorder/hooks and queue metadata.
Do not enable `cpu-profiling` or `arm32_cpu/profiling` through another feature.
CPU timing is per engine call; step attempts are derived from the existing
remaining budget, with the undefined-instruction attempt accounted separately.
No instruction-stage clocks or counters are inserted.

To bootstrap private data on an AVD, install the debug variant built with `-PgomulTrace`, import owned
games/data normally or seed it with run-as, then upgrade to the signed release
variant before measuring. Do not redistribute the seed or diagnostic captures.
Checkpoint compatibility checks remain intact; navigate manually when an older
checkpoint is incompatible. Existing recording and all guest timing are unchanged.

## Capture (device-local timestamps)

1. Open the game manually, finish loading/reconstruction, and reach the scene.
2. Establish that the symptom is present in this build. Record a stationary
   trace-disabled control using the existing guest-paint log.
3. Start Perfetto, streaming the config over stdin (AVD SELinux may deny
   reading configs from /data/local/tmp):
   `adb shell perfetto --txt -c - -o /data/misc/perfetto-traces/gomul-input.pftrace < tools/input-trace/capture.pbtxt`
4. Enable the native recorder without reopening the activity:
   `adb shell am start -f 0x20000000 -n local.wie.nativeapp.trace/local.wie.nativeapp.MainActivity --ez trace true`
5. First do a short pilot and check buffer loss/observer cost. If usable, collect
   20–30 isolated directional press/release probes in a 30–45 second window,
   with varied intervals and a visually identifiable response. Use Android touch
   input, not a direct JNI/channel helper. Mark ambiguous gameplay responses.
6. Disable via the same intent with `--ez trace false`. CSV serialization happens
   after recording stops, on the IO executor. Pull
   `/sdcard/Android/data/local.wie.nativeapp.trace/files/input-trace.csv` and the
   Perfetto file. Run `python3 tools/input-trace/analyze.py input-trace.csv`.
7. Repeat the stationary trace-disabled control. Reject runs with loss or material
   observer effects. A provisional 2% target does not establish zero overhead.

The bounded buffer holds one million 32-byte records (about 32 MB) and is reused.
Long captures may exceed capacity: any lost records invalidate the run. Shorten
or aggregate the capture after the pilot rather than interpreting partial data.
The recorder currently locks a shared preallocated buffer; this overhead must be
measured before accepting timing conclusions. It is not assumed negligible.

## Schema and interpretation

CSV: CLOCK_MONOTONIC ns, Linux TID, kind, phase (B/E/I), ID, value.
B/E spans use unique IDs. Begin values: task ID (4), requested budget (5), SVC
category (6), observed guest time (3; never use as telemetry wall time).
End values: Ready=1/Pending=0 for tasks/host polls; attempted steps for CPU;
executor reason 1=deadline, 2=sleepers before wakeup, 0=other/error.
Kinds: 1 tick, 2 executor, 3 pass, 4 task poll, 5 CPU run, 6 active host poll,
7 paint, 8 frame copy (including mutex wait), 10 input original event ns,
11 channel send span (input ID; down=1), 12 worker dequeue, 13 synchronous
emulator delivery, 14 event-queue pop, 15 image publication, 16 image pickup,
17 queue push, 18 CPU exit (0 return, 1 budget, 0x100000000+SVC category),
20 Choreographer (supplied timestamp as ID, actual entry as value),
21 bitmap update end, 22 draw begin (draw ID, paint ID), 23 frame commit
(draw ID, paint ID), 24 task/sleeper counts, 25 listener-entry ns,
26 event-to-press relationship, 27 platform frame ID/intended vsync,
28 platform frame ID/vsync, 29 platform frame ID/total duration,
30 platform frame ID/dropped metric callbacks, 31 bitmap update begin,
32 draw end, 99 clock synchronization (BOOTTIME as ID, MONOTONIC as value).
A synthetic key operation has original event time zero. Native repeat events
currently remain untagged; do not infer individual repeat latency.

Paint IDs and publication timestamps travel under the pixel mutex and remain
attached to the Java bitmap. onDraw adds an Android Trace section containing
its draw/paint IDs. FrameMetrics supplies the API 36 platform vsync ID, but the
analyzer does NOT assume a mapping to the closest draw. Verify that association
in the platform trace before following SurfaceFlinger's display timeline.
Reject ambiguous associations, including rotation during capture.

## Limits / pending scene validation

This first diagnostic stage tests uninterrupted polling and input-channel wait.
Queue pop is NOT actual guest callback exposure. No first-affected-frame oracle
or semantic game-state hook has been validated. No photon/presentation latency
is claimed from next-paint or frame-commit measurements. Commit is submission,
not display. The CPU counter measures attempts, not useful guest progress.

Exclusive task categories are GUEST_CPU, HOST_API, and OTHER. Nested re-entry is
subtracted; OTHER is not automatically JVM. These are wall-time categories until
intersected with Perfetto scheduling intervals. Genuine JVM/GC categories and
callback exposure require further validated markers if the initial trace needs
them. The current analyzer does not yet perform that platform intersection.

Source checks and correctness tests support the unchanged execution semantics;
they do not establish observer overhead, responsiveness, or improved performance.
An AVD result applies to the Mac Android emulator, not the Fold 5.

## Pilot correction: per-task aggregation

The first eight-second crowded-scene pilot overflowed: 8,555,760 records were
lost after the first million. Do not use that pilot for latency/performance
conclusions. Almost all records were CPU/SVC boundaries.

The revised recorder keeps those boundaries in a fixed, thread-local stack and
emits per-task totals, instead of acquiring the buffer mutex for every CPU/SVC
boundary. Only task polls >=250 us retain detailed B/E records and per-task
points 40–47. These points contain exclusive CPU ns, exclusive active-host ns,
OTHER ns, CPU-run count, attempted steps, SVC exits, Ready host polls and Pending
host polls. Complete ticks emit the corresponding totals at 50–57, total task
count at 58, and maximum task duration at 59. Short tasks contribute to every
aggregate despite not retaining an individual span. Pass/task-count points are
omitted. CPU/host spans and CPU exit points are consumed by the aggregator rather
than written individually. Unmatched capture-edge tasks/ticks are excluded.

CPU/SVC boundary clocks and instrumentation still have a cost. A new pilot and
trace-disabled controls remain required; aggregation does not establish low
overhead by itself. Invalid scope nesting increments the loss counter and
invalidates the recording, as does buffer overflow. Stack capacity is 64 active
nested CPU/host scopes. No changes were made to guest execution or scheduling.

## Queue blocker ledger (next capture)

`sh tools/input-trace/build.sh` builds the isolated release APK and supplies its
checkpoint build identity. Requires the SDK/NDK/Rust environment described above.
No new scheduler policy, event priority, repeat suppression or redraw coalescing.

Each enqueue has its own event ID: 60 type (1 Redraw, 2 Down, 3 Up, 4 Repeat,
5 Timer, 6 Notify); 61 related physical input ID or zero; 62 number already queued;
65 timer's guest deadline. 63 records pop/type; 64 post-pop queue length. Every
repeat has a distinct ID. Deferred timers retain their event ID on reinsertion,
so repeated enqueue/pop records represent scanning, not newly created timers.
90 records remaining physical keys' one-based rank after every pop. Between
changes, rank is constant. No per-instruction tracing is added.

Ledger spans explicitly include async suspension: 70 EventLoopRunner iteration,
71 getNextEvent, 72 pre-pop serviceRepaints, 73 callSerially batch, 74 individual
Runnable callback, 75 due timer callback, 76 timer yield, 77 empty-queue sleep,
78 dispatch body, 79 pre-pop cycle. Point 80: timer due boolean; 81: captured
callback count; 82: getNextEvent lifetime ID → returned backend event ID;
83: dispatch branch (1 key, 41 repaint, 1000 notify); 87: already-observed guest
clock at timer scan. No additional guest-clock reads.

67 maps iteration ID → executor task ID; 68 maps task ID → role 1 EventLoopRunner.
Other task roles remain unknown. Wrapper spans 89 getNextEvent and 88 dispatch
carry iteration IDs, allowing unambiguous get/dispatch/event association; the
analyzer counts ambiguous/capture-edge associations rather than guessing.

84 represents active future polls. The recorder aggregates count/total/max at
91/92/93 keyed to each operation's lifetime ID. Only polls >=250 us retain
individual spans. Lifetimes are not CPU costs; active polls still include OS
preemption. A fixed capacity of 64 active ledger operations/poll nesting bounds
recording state; overflow/invalid nesting invalidates the capture. Callback
class names are not inspected; each invocation has a diagnostic identity.

94 marks the stimulus phase, sent with an activity intent `--ei tracePhase 1`
(short taps) or 2 (long holds). Capture two separate short windows after a pilot:
about 20 brief taps, then several intentional holds. Check actual press duration
from original Android timestamps; do not assume every user action was a tap.
Keep repeat generation unchanged. Run `ledger.py capture.csv` for per-key rank
histories, preceding popped event types and a wall-time ledger. Inclusive stage
totals must not be summed. A partial/ambiguous trace requires inspection.

Validation: queue/future-poll tests pass with and without input-trace; timer
identity/repeat-order and nested aggregation regression tests pass. Two broader
MIDP tests (timed alert without previous screen; ticker screen retention) hang
both here and on unmodified d255d1aa; they are recorded as existing limitations,
not passing tests. No capture from this ledger build has been interpreted yet.

## Timer identity census

The census retains Def/Set/Unset and logical timer registration identity.
The current runtime includes the cancellation fix documented in
`docs/timer-cancellation-fix.md`; earlier census reports describe the pre-fix runtime. Guest WIPICTimer
layout is unchanged. Timer registration and deferred-event metadata compile to
zero-sized types when input-trace is omitted. No guest memory holds trace fields.

Enable narrow census mode with `--ez timerCensus true` in an activity intent,
then start the recorder with `--ez trace true` **while still in the library**.
Open the game normally and navigate to the scene. Avoid Quick Load for this
capture: registration origins must be observed from startup. Game's own save
loading is fine. Census mode filters out CPU, executor and display traces before
reading diagnostic clocks or locking the recording buffer. It keeps queue and
timer identity records; loss still invalidates the capture. Disable recording
normally, pull the CSV and run `timer_census.py capture.csv`.

Additional records:
- 110 backend event ID → Set registration ID (both enqueue and pop).
- 111 Def operation ID → timer pointer; 112 same ID → callback address.
- 113 Set registration ID → timer pointer; 114 → captured callback;
  115 → parameter; 116 → requested timeout; 117 → computed guest due time;
  118 → existing observed guest now; 125 → parent active callback registration.
- 119 Unset operation ID → timer pointer. Pending registrations are invalidated by the cancellation fix.
- 126 backend event ID → zero: cancellation skipped guest entry and the post-callback yield.
- 120 callback wall lifetime, value = registration ID.
- 121/122/123 repeat registration pointer/callback/parameter at callback entry,
  so a registration created before capture can be identified if it later runs.

Set operation IDs monotonically increase but share the global diagnostic ID
allocator, so gaps are expected. The enqueue registration scope covers only
synchronous context.set_timer; the callback parent scope covers one future poll
and is restored on Pending, Ready or error. Both rely on this adapter's single
emulation-worker execution, and never stay set across async suspension.

The census reports queued, popped/deferred and in-flight instances separately.
An executing callback plus its one queued successor is not itself evidence of
a duplicated pending timer. A pointer may be reused/redefined: inspect Def and
operation chronology before calling it one logical lifetime. Partial initial
queues are marked incomplete; they cannot establish where multiplicity began.
Unknown timer sources remain unknown. No WIPI replacement/cancellation contract
is inferred from the data alone.

Validation adds scoped-identity restoration on nested enqueue, Pending and error;
registration retention on timer reinsertion; and a reconstruction fixture that
distinguishes two pending registrations from an in-flight callback's successor.


The cancellation audit reports registrations invalidated before callback entry,
any subsequent guest entries, and any successors created by those registrations.
It does not count an already-entered callback as canceled. Physical queued counts
can briefly include canceled events waiting to be discarded at their due check;
these must not be confused with live callback chains.

## Callback cost baseline (post-cancellation)

Full trace mode now emits callback-owned aggregates keyed by the kind-75 timer
stage ID: 130 exclusive CPU ns, 131 exclusive host/SVC-future ns, 132 CPU runs,
133 step attempts, 134 SVC exits, 135 Ready host polls, 136 Pending host polls.
Kinds 140–147 count run lengths in bins 0, 1–16, 17–64, 65–256, 257–1024,
1025–4096, 4097–9999, and 10000+. Kind 92 remains total active callback poll ns.
`callback_cost.py capture.csv` joins these to registration and rearm metadata.

Ownership is scoped to an actively polled timer future and restored on every
poll exit, including Pending. The existing CPU/host nesting provides exclusive
buckets; no new per-instruction timing is added. Costs remain wall observations,
not on-CPU measurements. Measure observer overhead before accepting small gains.
Counters alone cannot prove identical guest work: deterministic replay, SVC
sequence and final CPU/memory/output validation remain required for candidates.

## Native source attribution within active callbacks

Build the diagnostic APK with `CARGO_PROFILE_RELEASE_DEBUG=1` and retain the exact
unstripped `libwie_android.so` before another build overwrites it. Debug metadata
is for source attribution; do not use a different build's addresses or infer that
symbol settings left machine code identical. Record `cpu-clock:u` native PCs with
NDK simpleperf alongside a full callback trace. Use the library build ID from
`simpleperf dump` and the corresponding ELF, not just a matching filename.

Export `simpleperf report-sample` text and `simpleperf dump`, then run
`native_callback_samples.py --samples TEXT --trace CSV --elf ELF --build-id ID
--perf-dump DUMP --readelf LLVM_READELF --symbolizer LLVM_SYMBOLIZER`.
The helper checks recorded/library build IDs, CLOCK_MONOTONIC, diagnostic record
loss, and native lost-record entries. Preserve the native recorder's zero-loss
summary too. It attributes each selected PC once to the innermost inline source
frame, retaining the full inline stack.

Only native samples inside the retained (>=250 us) active timer polls are selected.
Other samples, including short polls that were aggregated away, remain outside that
subset. Report its coverage and do not treat it as every callback instruction.
Sampling skid/compiler line tables are not exact per-stage measurements. Guest PC
and opcode distributions require a separate count/sampling mechanism; native PCs
do not identify guest addresses. Use sampling-disabled control windows before
accepting small performance differences.
