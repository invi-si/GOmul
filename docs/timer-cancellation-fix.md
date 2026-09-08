# WIPI pending-timer cancellation candidate

Implements the observed missing contract: Unset invalidates every pending registration for its timer pointer. Registration is consumed synchronously immediately before guest callback entry; an already-entered callback is allowed to finish. Repeated Set and Def without Unset retain existing behavior.

Each Set allocates a guest-backed linked registration containing timer identity and a cancellation flag. The shared System adapter retains only the list head address. Guest WIPICTimer layout remains one word; cancellation neither adds fields to that ABI nor rewrites its callback address. Each pending callback closes over its registration address, so moving it through the MIDP deferred vector does not detach cancellation identity. At due delivery the adapter unlinks/frees the registration and checks its flag before guest entry. Cancelled registrations return false, suppressing the normal post-callback yield. Live timers retain the existing yield, due-time calculation and FIFO ordering.

Cancelled future events remain inert in the existing queue/deferred path until their due check releases them. This patch does not add queue sweeps, priority changes, timer caps, or Set replacement policy. Outstanding storage is proportional to unconsumed registrations. The guest list is traversed on cancellation and removal; measure before considering a more complex representation.

Tests cover pending cancellation, old/Unset/new with Def reuse, successor cancellation, in-flight completion, another timer remaining live, repeated Set/Def behavior, identity retained outside the backend queue, actual MIDP deferred-vector cancellation, and skipping cancelled due timers without an extra yield or reordering live events. Existing recurring-timer cooperative-yield tests remain in place.

This is a local diagnostic candidate, not a published release. A fresh boot census and matched input-ledger run are still required before claiming a gameplay latency improvement. Use normal startup rather than an old Quick Load replay for this diagnostic capture; no original app saves are reset.

Validation completed: WIPI-C 46 tests with tracing disabled/enabled; backend 52 disabled and 53 enabled; four targeted MIDP event-loop tests in both configurations; 83 ARM-core tests; Python census and ledger fixtures. Workspace Clippy completed with existing warnings. The two previously baseline-reproduced MIDP full-suite hangs were not rerun.

Candidate APK SHA-256: `d18787353be3614872f70fd75038e98e4e8d97bbe3fd28b1b94772fe676adce8`. Built and installed over isolated `local.wie.nativeapp.trace` on emulator-5554. Normal library startup opened and census enabled before manual navigation. Gameplay comparison remains pending user navigation.

## Post-fix AVD sample

The initial pull was identified as the old capture by its timestamps and contents; it was rejected. Explicit `--activity-single-top` was used for subsequent trace-control intents, and fresh save completion was verified. The original candidate startup trace was not recovered. Therefore the following census is steady-state evidence, not a complete startup cancellation audit.

Fresh census: 14.140 seconds, zero lost records, 154 observed Sets, 153 complete callbacks. Same pointer as baseline: 0x0150b570. Peak one queued plus one in flight (total two), versus baseline seven queued plus one in flight (total eight). No Unset calls occurred in this sample; zero post-cancel entries is consequently not a meaningful cancellation correctness gate here. The parser reports no unknown initial queue pop, but capture still began during gameplay and cannot establish startup ancestry.

Four controlled touchscreen directional holds, alternating right/left at approximately 1.65 seconds, produced eight press/release events in a separate 6.759-second full ledger. Zero lost records. Queue-enqueue-to-pop delay: median 5.255 ms, range 0.038–17.440 ms. All eight events entered at rank one, and none had another timer popped ahead of them. One overlapped the remainder of an already-running callback. Earlier tap/hold captures measured approximately 209–223 ms median queue waits, with six or more timers ahead. This supports removal of that specific backlog; it is a small sequential scene comparison, not a randomized or physical-phone benchmark.

73 guest paints / 6.759 seconds = 10.80 guest paints/sec; 59.20 million CPU step attempts/sec. Timer callback mean wall lifetime 25.93 ms, maximum 32.46 ms. The requested timer delay remains 60 ms. A single rearming loop now waits between callbacks instead of continuously servicing stale chains. Earlier 30–44 guest paints/sec are not a clean measure of correct useful game updates because duplicated chains were generating frames. No screen-response oracle or post-fix platform presentation trace was collected: guest queue delay is not end-to-end visible response latency. Character survival and exact visual scene equivalence were not independently checked.

Remaining: recover a verified clean-start census to exercise actual cancellation gates, then evaluate perceived responsiveness and remaining cadence on the Fold 5. Do not reduce timer delays or reintroduce duplicate work to raise the FPS counter. Recording is stopped; current game remains open.

## Verified clean-start validation and normal baseline

A new process started at the library. Recorder preflight marker 101 was verified in a fresh file, then recording restarted with marker 102 before manual normal game boot. The 85.047-second capture has zero lost records and no unknown initial queue entries. Seven Unset calls invalidated six pending registrations. All six have matching cancellation-skip events; zero entered guest callbacks and zero rearmed successors. This includes two registrations in the local deferred vector at cancellation. Thus the previously pending clean-start cancellation gate passes for this run.

Across startup, 546 registrations and 539 completed callbacks were observed. Raw pending population briefly peaked at three queued plus one in flight; raw counts include cancelled registrations awaiting disposal and are not a count of live duplicate chains. Unlike baseline, those cancelled registrations all terminate without guest execution.

A subsequent stationary normal-gameplay full trace lasted 14.063 seconds, zero lost records, zero physical input events. Guest paint rate 10.95/sec. Callback-start interval median 91.404 ms, p95 95.804, maximum 104.544; paint interval median 91.416 ms, p95 96.050, maximum 102.769. Callback lifetime median 26.039 ms, p95 28.380, maximum 35.968.

152 parent/successor cycles were explicitly linked: parent start to Set median 26.029 ms; requested timeout always 60 ms; Set to successor entry median 65.207 ms. Guest due-check lateness median 5 ms, maximum 15 ms; due-check to guest-entry trace median 0.009 ms, maximum 0.136 ms (host clock). These are separate distributions, not an assertion that their medians add exactly. The distinction between guest due-check and actual entry is retained rather than inventing an unrecorded guest-time read at entry.

Normal input latency cannot be estimated from this stationary segment because it has no inputs; the previous controlled-hold result remains separate. Skill-heavy comparison is pending manual skill activation. No CPU or timing changes were made. Artifacts and offline analysis script remain in ignored release-artifacts/timer-clean-start-avd-2026-09-09/.
