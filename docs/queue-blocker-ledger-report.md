# Queue blocker ledger — Android AVD, 2026-09-09

**Due timer callbacks dominate the measured input queue wait.** Task 2 is
confirmed as `EventLoopRunner`. The backlog is not primarily repeat dispatch,
callSerially work, standalone repaint dispatch, or gaps between cheap pops.
This establishes where the wait occurs, not yet why the timer callbacks are
expensive or whether timer registration/cancellation is correct.

## Measurements

| Capture | Complete key events | Queue wait median / max | Timer share of per-key waits |
|---|---:|---:|---:|
| Short-tap portion, 40 s | 82 | 223.1 / 262.2 ms | 98.67% |
| Hold portion, 25 s | 6 | 209.2 / 284.1 ms | 98.84% |

These are backend enqueue→pop intervals, not input→visible-response latency.
The percentages sum individual key-wait ledgers; overlapping key windows can
count the same time, so these are not unique whole-capture CPU percentages.

The tap portion contained 42 timestamp-paired presses, median 102 ms and maximum
198 ms, with 41 down/up pairs completing backend dequeue inside the window.
It still generated 16 repeat events. The hold portion contained two intentional
holds of 1.65/1.68 s and one 1 ms press, generating 30 repeat events. The character
died around the first portion, and the hold capture used a changed/recovered
scene. These are **not a controlled same-state tap-versus-hold comparison**.
There is nevertheless direct timer-dominated queue-wait evidence in both.

A representative short-tap key entered at rank 7:

| Elapsed since enqueue | Rank |
|---:|---:|
| 0 ms | 7 |
| 23 ms | 6 |
| 55 ms | 5 |
| 93 ms | 4 |
| 123 ms | 3 |
| 156 ms | 2 |
| 192 ms | 1 |
| 230 ms | popped |

Six timer events were popped ahead of it. Timer callback lifetimes overlapped
223.2 ms of that key's 230.1 ms wait; timer yields accounted for 6.7 ms.
The final rank-1 wait occurred after a preceding timer was popped and before
its callback completed. This explains why counting FIFO positions alone was
insufficient.

## What ran

Short-tap capture: 1,214 timer pops, all already due; 41 KeyDown, 41 KeyUp,
16 KeyRepeat. 1,213 complete timer callbacks occupied 39.551 s of wall lifetime,
with 35.206 s of active future polls. Mean callback lifetime was about 32.6 ms;
maximum 55.5 ms. All 98 normal event dispatches together took only 25.3 ms.

Hold capture: 1,109 timer pops, all due; 3 KeyDown, 3 KeyUp, 30 KeyRepeat.
1,108 complete timer callbacks occupied 24.751 s of wall lifetime, with
22.242 s of active future polls. Mean callback lifetime was about 22.3 ms;
maximum 52.5 ms. All 36 normal dispatches together took 6.7 ms.

Neither capture observed callSerially callbacks, pre-pop serviceRepaints,
standalone Redraw events or Notify events. This does **not** rule out graphics
work inside the timer callbacks.

Every complete timer callback in both captures contained exactly **one new
timer enqueue and one image publication**. Timers were already late in guest
clock time: median 138 ms in the tap portion and 91 ms in the hold portion.
This is consistent with recurring game-update/render work occupying the event
consumer. Timer IDs identify each queue instance, but the trace does not yet
identify the guest timer object or callback address, so distinct timers cannot
be distinguished from repeated registration of the same timer object.

Guest paints/sec were 30.30 and 44.22 respectively. The changed scene prevents
interpreting that difference as a performance improvement. No production
optimization, timing change or lag fix was applied.

## Validity

Native recorder loss: zero in the pilot and both stimulus captures. Normal
getNextEvent/dispatch lifecycle association had zero ambiguous interval links;
edge-incomplete events remain unknown. Task-2 role was observed explicitly.
Detailed active-poll intervals below 250 us are omitted, but their counts,
total duration and maximum are retained per operation.

The stationary six-second controls measured 29.59 paints/sec before,
28.26 with tracing and 28.79 after. This is about 3.2% below the mean controls:
a precise <=2% overhead target was **not established**. Absolute timings remain
exploratory and potentially inflated. The strong structural finding is timer
callbacks occupying approximately 99% of individual queue waits. No new
Perfetto capture was taken; active-poll time includes OS preemption and is not
an on-CPU measurement. Async lifetime also includes suspension.

APK SHA-256:
`45861cb2e913f79df860098b6ee91f8e6ace7b3ee3454d493bf8ec53e7944f52`.
Package `local.wie.nativeapp.trace`, native arm64, non-debuggable release,
Android 16 Mac-hosted AVD. No inference about physical Fold 5 performance.
Private raw captures are in ignored `release-artifacts/queue-ledger-avd-2026-09-09/`.

## Recommended next step

Audit the native WIPI timer path before changing scheduling. In
`wie-wipi-c/src/api/kernel.rs`, `set_timer` schedules a closure that invokes a
guest callback; `unset_timer` is currently a stub. That is a concrete code gap,
but this capture does not prove this game calls it or that it causes the backlog.

Add timer-object address, callback address, requested timeout and set/unset
operations to the existing ledger. Determine whether the queued recurring
callbacks belong to distinct timer objects or stale/duplicate registrations,
and whether expensive time is useful update/render work or polling. Then fix
confirmed API semantics or optimize the identified work, with regression tests.
Do not drop timers, change their periods, prioritize input, or introduce arbitrary
yields merely to shorten these measurements. Input lag remains unresolved.
