# Legitimate callback cost baseline — AVD

The user validated the timer-fix APK subjectively on the Fold 5 and explicitly chose the Mac Android environment for further profiling. Do not describe these measurements as physical Fold measurements or request reconnection as a prerequisite.

Candidate APK SHA-256: a24cf1845e87889506d89013db0da7edb45d7336cdbf8046ae4a23bc25c0c413. Only callback-scoped diagnostic aggregation changed; CPU execution, timer semantics, scheduling and refresh behavior were unchanged. Six Rust aggregation tests and one Python join/exclusion fixture passed; workspace Clippy and release build completed. Recording is compiled out without input-trace.

## Stationary crowded scene

23.309-second recording, zero lost records, 247 complete timer stages, no physical inputs. Per-callback means:

| Measurement | Mean |
| --- | ---: |
| Guest CPU exclusive active wall time | 21.321 ms |
| Host/SVC future exclusive active wall time | 4.293 ms |
| Total active poll wall time | 25.614 ms |
| Total callback lifetime | 28.455 ms |
| CPU step attempts | 6,185,062 |
| CPU runs | 7,966 |
| SVC exits | 7,750 |
| Host Ready polls | 7,750 |
| Host Pending polls | 0 |

Guest CPU accounts for about 83.24% of recorded active callback work. Median CPU time is 21.044 ms; maximum 31.802 ms. Median callback lifetime is 28.048 ms; maximum 43.004 ms. 79.43% of CPU runs contain at most 64 step attempts. This describes run frequency, not time spent: long runs may still dominate execution cost. Do not infer that SVC dispatch or short runs are the performance bottleneck solely from these counts.

All per-callback run histograms sum to CPU-run counts; CPU+host time never exceeds active poll time. Nested CPU work is subtracted from host intervals; task work during callback suspension is excluded. Host Pending count zero does not imply zero suspension: budget-exhaustion yields and other async boundaries remain. Wall time includes host preemption and diagnostic work; it is not on-CPU hardware time.

The recording observed 10.597 guest paints/sec. A subsequent stationary recording-off check gave 10.750 paints/sec across 28 complete one-second logging windows. The approximately 1.4% difference is only a rough observer-cost check: samples are sequential, game state was not replayed identically, and this is not a repeatability or sub-2% overhead guarantee. No optimization is accepted on this basis.

## Remaining gates

Next collect skill-heavy callbacks with the same instrumentation, then use low-overhead sampling to locate costs within legitimate guest execution. Before retaining any execution change, construct or select deterministic captures and verify CPU/memory results, SVC/event sequence, timer behavior and paint output against control. Aggregate counters do not prove equivalent guest work. Existing SVC-bounded deterministic CPU replay fixtures remain secondary evidence; no whole-callback state/checksum equivalence has yet been established.

The broad CPU ownership result justifies investigating the engine; it does not yet justify a specific dispatch/cache/flags implementation. No CPU optimization was made this turn. Raw captures are retained under ignored release-artifacts/callback-cost-avd-2026-09-09/.

## Skill callback baseline

28.044-second user-controlled skill session, 267 complete callbacks, zero lost records. Per-callback mean CPU active exclusive wall time 29.861 ms, host/SVC-future wall time 4.506 ms, active poll time 34.366 ms, total lifetime 38.765 ms. CPU share is approximately 86.9% of measured active work. Mean steps 8,537,579, CPU runs 7,951 and SVC exits 7,474; no Pending host polls. CPU median 28.595 ms and maximum 59.380 ms. Callback lifetime median 37.251 ms and maximum 78.704 ms.

Relative to normal, CPU work rises about 8.54 ms and host work only 0.21 ms, while steps rise about 38%. This supports guest execution as the next investigation target, rather than an increased SVC count. It does not identify an internal engine hotspot. Neither sample fixes identical state, and wall diagnostics have observer/preemption effects.

Input: 126 records, 125 matched worker-to-pop delays; lower-quantile p50 9.768 ms, p95 30.609 ms, maximum 63.248 ms. The unmatched input is excluded. Paint rate 9.556/sec remains observational and is not an optimization target. No CPU execution code changed.

## Native sampling feasibility

After stopping diagnostic recording, sampled the running AVD process with NDK simpleperf at requested 199 Hz, user-space cpu-clock, for 8 seconds. The initial unprivileged command was denied; restarting the AVD adb daemon as root enabled recording. 372 samples, zero lost. Process-wide attribution: 72.04% in Arm32CpuEngine::run on emulator worker TID 11019; 5.38% in Arm32CpuMemory::r32; 4.03% in Cpu::thumb_mode. Most CPU implementation is inlined into run, so these function totals do not identify fetch/decode/dispatch/flags costs. This was a short feasibility sample, not callback-filtered, not a skill-isolated interval and not an overhead-calibrated comparison. Percentages cover all sampled process threads; they must not be substituted for callback CPU/host percentages.

Next use exact-build native address/source attribution and callback interval correlation to resolve the inlined run hotspot, with a recording-off/sampling-off control. Do not infer that thumb_mode alone warrants reviving the previously rejected CPSR-local optimization. Keep candidate acceptance tied to identical deterministic guest work and repeatable cost improvements.
