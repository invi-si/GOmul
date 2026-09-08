# AVD input-delay trace — 2026-09-09

The strongest finding is input waiting in the emulator's event queue, after the
native worker accepts it. The capture supports investigating queue servicing
before another broad CPU rewrite. It does not yet identify which queued event
or callback produces the backlog, or justify changing event order/timing.

## Build and measurement

Native arm64 Android 16 app, Mac-hosted AVD `emulator-5554`, package
`local.wie.nativeapp.trace`, non-debuggable/profileable release. Based on
`release/gomul` d255d1aa with uncommitted diagnostic changes. Features:
`input-trace,experimental-thumb-inline,experimental-thumb-table`; instruction-stage
profiling absent. Recording, guest clocks, CPU budgets, scheduler, renderer and
input order unchanged. APK SHA-256:
`5504b82cb8a30f14df2bf65cd1e659c37be8bacacb7fc0f8d29c1d190ff8dfe8`.

45.032 seconds in the user-selected crowded gameplay scene. The user started
operating near the end of the window: 17 complete physical down/up events,
plus one synthetic release at the capture edge. This is fewer than the planned
20–30 isolated probes. Several presses were held for roughly 0.5–1.3 seconds,
so generated repeats may contribute to queue load; individual repeats were not
traced. No claim of a controlled tap-only test or first-affected-frame latency.

## Results

| Interval | Median | Exploratory p95 | Maximum |
|---|---:|---:|---:|
| Android event → UI listener | 0.476 ms | 0.738 ms | 0.778 ms |
| Listener → native worker dequeue | 6.525 ms | 23.414 ms | 24.273 ms |
| Worker dequeue → emulator event-queue pop | 179.827 ms | 201.446 ms | 240.526 ms |
| Listener → event-queue pop | 191.749 ms | 218.082 ms | 244.180 ms |
| Image publication → Android draw | 7.412 ms | 16.001 ms | 24.616 ms |
| Image publication → AVD platform presentation | 34.022 ms | 43.881 ms | 73.069 ms |

The 17 keys entered behind **6–9 queued events**. Across their queue-wait
intervals, the worker was Running for 78.8%, runnable but unscheduled for 11.9%,
and sleeping for 9.3%. Retained task-2 polls covered 72.8% of those intervals.
The worker remains mostly busy while keys are queued; host scheduling alone
does not explain the delay. This does not establish whether the busy work
serves earlier queue items, pre-pop callbacks/timers, or unrelated tasks. Event
types ahead of the keys remain unidentified.

A single task poll reached **41.402 ms**, containing 5,498 CPU runs, 2,112,334
step attempts and 5,214 immediately completed SVC polls. Executor ticks reached
43.222 ms despite checking an 8 ms deadline between passes. Long SVC chains are
confirmed, but an individual chain does not explain the entire ~180 ms backlog.

Complete-tick task summaries attribute 32.573 s to CPU execution, 7.084 s to
active host polls, and 0.106 s to OTHER: approximately 81.9% / 17.8% / 0.3%.
These are **exclusive wall times**, including preemption. Aggregation removed
the individual CPU/host intervals, so they cannot be precisely intersected with
Perfetto scheduling. Do not relabel these as on-CPU percentages or OTHER as JVM.

Throughput: **154.59 million step attempts/sec**, **30.73 guest paints/sec**,
and **30.56 paint generations presented/sec** within the Android guest display
stack. This is neither Mac-panel FPS nor an optimization before/after result.
No emulator optimization was applied and input lag is not resolved.

## Trace validity and limits

The original verbose pilot overflowed and was rejected. The revised recorder
uses bounded thread-local aggregation; CPU/SVC details become per-task/tick
counters, and individual task intervals are retained at >=250 us. Both revised
captures lost zero native records; two edge spans are incomplete. Stationary
controls measured 28.35 paints/sec before, 28.20 with tracing, and 27.91 after.
The traced value lies between controls; this short sequence cannot prove a
precise overhead bound or rule out probe-specific observer effects. The full
capture also enables Perfetto and uses a changing gameplay workload.

Scheduling and FrameTimeline data were present with no reported event loss.
37 setup notices concern unavailable vendor GPU/display tracepoints; CPU
frequency data was unavailable. No thermal/frequency conclusions follow.

For 1,377 images, the draw marker's ancestry identifies the Choreographer token,
which exactly joins the app surface-frame token and then its display-frame
token. All joins were unique and non-dropped; 1,375 also had matching asynchronous
FrameMetrics callbacks. Presentation uses the **display timeline's endpoint**,
not the app slice or frame-commit callback. See the official
[FrameTimeline semantics](https://perfetto.dev/docs/data-sources/frametimeline).
Native monotonic and BOOTTIME synchronization offsets were zero at both ends.
This is presentation inside the AVD, not a photon measurement at the Mac screen.

Queue pop is earlier than actual guest callback exposure. No response oracle
was validated. Consequently input-to-visible-response remains unmeasured;
the separate median intervals must not be added and called measured latency.

## Validation and next step

137 targeted CPU/backend/utility tests passed with instrumentation compiled in
and out. Four aggregation tests cover nested exclusive attribution, short-poll
accounting, capture edges and invalid nesting. Workspace Clippy and Android
trace-feature Clippy completed with warnings; release packaging passed.

Next, add bounded queue-event type/age metadata and active callback duration
summaries around `EventQueue::get_next_event` and dispatch. Identify what occupies
the 6–9 slots and why servicing them spans multiple frames. Then optimize the
identified callback work while preserving event order, timers, repeats and
guest-visible effects. Do not introduce priority reordering, dropped repeats,
new yields or timer changes on the strength of this trace alone.

Private captures and summaries are retained in the ignored
`release-artifacts/input-trace-avd-2026-09-09/` directory. No game files,
snapshots or captures were published. This result applies to this AVD scene;
it does not establish Fold 5 performance or results for other games.
