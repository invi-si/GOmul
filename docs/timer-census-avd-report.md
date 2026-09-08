# WIPI timer identity census — Android AVD, 2026-09-09

The trace strongly supports stale timer registrations as the mechanism behind the input queue backlog. The game calls `MC_knlUnsetTimer` while a registration is pending, but the current implementation is a success-returning stub. That old registration subsequently executes and rearms itself alongside a newer registration of the same timer.

## Capture and counts

Native diagnostic APK on the Mac-hosted Android 16 AVD, package `local.wie.nativeapp.trace`. Capture began before normal game boot, with no unknown initial queue entries and zero lost records. The user navigated to gameplay and left the character still for the final interval. No production timer, scheduling, input, or rendering behavior was changed.

- One timer pointer: `0x0150b570`; one callback: `0x00048f31`; parameter always zero.
- 1,508 Set operations, 1,501 complete callback lifetimes, eight Def and seven Unset operations.
- Requested timeout: 60 ms for 1,506 registrations; 500 ms for two.
- Peak same-pointer population: seven queued registrations, one in flight, eight total. A separate instant had six deferred registrations. These per-state maxima are not additive.
- Input #132 entered behind six timer events, all belonging to that same pointer/callback, plus the preceding key event. One timer callback was already in flight.

One executing callback plus its one successor is normal possible overlap. Seven pending registrations are a different finding.

## First directly observed stale chain

Times below are relative to the first Set operation, using host monotonic timestamps:

| Offset | Operation |
| --- | --- |
| 0 ms | Set registration 65056, timeout 60 ms, no existing registration |
| 52.985 ms | Unset 76775, with registration 65056 still queued |
| 52.986 ms | Def the same timer pointer and callback |
| 52.987 ms | Set registration 76781 while the old registration remains queued |
| 59.673 ms | Old registration 65056 begins its callback, after Unset |
| 70.146 ms | Old callback schedules successor 98159; the newer chain is also outstanding |

Later Unset operations occur with progressively more outstanding registrations, including six queued at the last Unset. The cancellation API gap allows earlier chains to survive and replenish themselves. The precise independent replacement semantics of Def and repeated Set still need an authoritative contract; this trace alone does not establish them.

## Re-arm timing

Across replacement Sets associated with complete callbacks, the median time remaining after Set is 0.003208 ms; the maximum is 7.40825 ms. Both are below the minimum requested timeout of 60 ms. The proposed mechanism of rearming early and then executing longer than the successor timeout is not supported by this capture.

## Relation to earlier input traces

Earlier queue-ledger captures measured approximately 209–223 ms median key queue waits, with about 99% of those wait windows overlapping timer callback lifetimes. This census identifies the multiple timer instances as one timer object, and directly observes an old chain surviving cancellation. This is stronger evidence for a correctness fix than another interpreter optimization.

No before/after improvement is claimed: cancellation has not been implemented, gameplay response was not measured after a fix, and these are AVD results rather than physical Fold 5 results. Instrumentation overhead for this census has not been separately quantified; use the structural event sequence, not its timing as a production performance benchmark.

## Validation and next step

Instrumentation tests passed with tracing enabled and disabled: backend 53/52, util 4/3, WIPI-C 41/41; two targeted MIDP event tests and the Python census/ledger checks passed. Workspace Clippy and release APK build completed. Two unrelated full-suite MIDP hangs were independently reproduced on the untouched baseline; a clean full-suite pass is not claimed.

Next: establish WIPI cancellation and timer lifetime semantics, then implement generic cancellation covering queued and deferred registrations and safe in-flight/re-arm behavior. Preserve guest-backed authoritative state and avoid game-specific workarounds. Add lifecycle regression tests before repeating this census and comparing input latency in the same scene. Do not prioritize keys, change timer periods, or impose an arbitrary one-timer cap to hide the backlog.

Raw CSV and parsed JSON are retained locally under ignored `release-artifacts/timer-census-avd-2026-09-09/`; they are not publication artifacts.
