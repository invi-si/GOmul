# Post-cancellation normal versus skill gameplay — Android AVD

No CPU, timer, scheduling, refresh, or rendering code changed between samples. Both use candidate d18787353be3614872f70fd75038e98e4e8d97bbe3fd28b1b94772fe676adce8 on the Mac-hosted Android 16 AVD. User manually supplied the normal scene and skill activity. These sequential captures are not a randomized benchmark; the trace has no semantic marker proving the exact beginning/end of each visual effect.

| Measurement | Normal | Skill session |
| --- | ---: | ---: |
| Capture duration | 14.063 s | 28.175 s |
| Complete callback lifetimes | 153 | 278 |
| Lost records | 0 | 0 |
| Callback-start interval median | 91.40 ms | 99.84 ms |
| Callback-start interval p95 | 95.80 ms | 120.94 ms |
| Callback-start interval maximum | 104.54 ms | 135.62 ms |
| Paint interval median | 91.42 ms | 99.94 ms |
| Paint interval p95 | 96.05 ms | 121.69 ms |
| Paint interval maximum | 102.77 ms | 140.07 ms |
| Callback lifetime median | 26.04 ms | 34.86 ms |
| Callback lifetime maximum | 35.97 ms | 70.79 ms |
| Mean active callback poll wall time | 23.46 ms | 31.43 ms |
| Requested timeout | 60 ms | 60 ms |
| Set-to-successor-entry median | 65.21 ms | 65.25 ms |
| Guest due-check lateness median | 5 ms | 5 ms |
| Guest due-check lateness maximum | 15 ms | 14 ms |
| Guest paints per second | 10.95 | 9.87 |

The effect-session median update interval is about 9% longer. Callback execution costs increase while re-arm waiting and timer lateness remain nearly unchanged. The data support improved stability without the old stale timer queue, but do not support claiming that effects have no measurable cost or that cadence is identical.

The skill session contains 90 physical input records, with 88 matched queue deliveries. Queue enqueue-to-pop median is 8.18 ms and maximum 37.56 ms; listener-to-pop maximum 43.41 ms. Queue ranks are one or two, unlike the old six-plus-timer backlog. Two input records lack matched queue pops and are excluded, not counted as zero latency. Normal stationary trace has no inputs, so there is no paired normal-input comparison in these two files. These are queue delivery timings, not a verified first-visible-response latency.

For completed timer stages, mean total callback wall lifetime is 26.27/35.59 ms and mean active poll time is 23.46/31.43 ms (normal/skill). The remainder includes async suspension. Active wall time still includes host preemption: no new scheduler/Perfetto recording was taken. Across all recorded tasks, exclusive guest-CPU wall totals are 2966.46/7635.51 ms; host API wall totals 691.86/1309.14 ms. These totals cover different capture lengths and are not a callback-specific CPU/SVC/JVM decomposition; JVM work is not separately isolated by this instrumentation.

The requested cycle is explicitly linked by parent registration: callback entry → Set → successor entry. Normal parent-entry-to-Set median is 26.03 ms, skill 34.86 ms. Set-to-entry is about 65.2 ms in both, comprising the requested 60 ms plus about 5.2 ms additional host time. Guest lateness comes from the existing due-check clock read, with the subsequent host-time gap to callback entry measured separately (median about 0.009 ms). Do not sum distribution medians as if they were one measured cycle.

## Original-device reference search

Searches for the Korean title with gameplay, video, original-device, and frame-rate terms, plus the romanized title, did not locate usable original-handset timing measurements or verified footage. This is a search limitation, not evidence that no footage exists. A same-carrier/version handset recording with known video frame rate, visible continuous movement and skill use is still needed to assess original pacing. No throughput target is selected in its absence.

The developer's [2009 launch announcement](https://www.newswire.co.kr/newsRead.php?ected=&no=426258) establishes a 2009 release rollout; earlier conversational references to a 2007 release should not be treated as verified. The announcement supplies no update-rate measurement.

Clean-start timer cancellation has separately passed: six pending registrations invalidated, all six skipped, zero guest entries or rearmed successors, including deferred-vector registrations. Keep broad CPU optimization frozen pending timing-reference work; the next practical independent validation is the same workload on the physical Fold 5. The current AVD game is left open and recording is stopped.
