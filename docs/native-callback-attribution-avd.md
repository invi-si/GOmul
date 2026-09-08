# Native source attribution during legitimate callbacks

Same optimized diagnostic APK and ELF verified by build ID 02b4c7a5d3002250fda2903e5b4750eabc8d1d63. Mac Android AVD, user-positioned stationary crowded scene. CPU, timer and scheduling source behavior unchanged. Native cpu-clock:u samples and full callback diagnostics ran together. No game controls were injected.

| Observation | 199 Hz requested | 313 Hz requested |
| --- | ---: | ---: |
| Native capture length | ~20 s | ~20 s |
| Recorder-reported native samples | 1,106 | 1,733 |
| Exported samples available to join | 1,106 | 1,732 |
| Native samples lost | 0 | 0 |
| Samples inside retained active timer polls, emulator library | 547 | 811 |
| Samples inside retained active timer polls, other modules | 96 | 162 |
| Outside retained active timer polls | 463 | 759 |
| Thumb match source location (thumb.rs:208), selected library samples | 86 (15.7%) | 113 (13.9%) |
| Inline memory_error path, selected library samples | 64 (11.7%) | 74 (9.1%) |

Both diagnostic CSVs have zero lost records. The second simpleperf text export contains one fewer sample than its recording summary; that one sample is unaccounted for and is not silently included in the classified denominator. Samples were joined by monotonic timestamp and thread ID to retained active polls, not merely callback wall lifetimes. The selected subset excludes short active polls (<250 us) whose individual timestamps are aggregated away. It does not represent every callback instruction. Selected other-module samples remain visible; sample percentages above use only selected emulator-library samples as denominator.

Register indexing and memory page access also repeatedly appear among leading source locations. `thumb.rs:208` is the instruction-kind match, but compiler line attribution/skid prevents interpreting every sample there as pure dispatch cost. Native PCs do not identify hot guest PCs, and these data cannot yet decide whether a small set of guest loops dominates.

## Prior experiment prevents a redundant prototype

The memory-error status is stored in RefCell<Option<u32>> and read after each emulated step. Source inspection suggests replacing it with a plain Option is semantically straightforward because writes have exclusive access. However, locating deterministic replay artifacts uncovered that the exact replacement was already benchmarked and rejected. Eight crowded-scene captures showed mixed median throughput changes, approximately -10.6% to +13.0%, without a dependable general gain. Prior gameplay comparisons also failed to establish benefit.

Do not revive this candidate merely because source samples land in that path. Sample share is not an estimate of removable overhead; generated code and register allocation can offset a source-level simplification. No new CPU patch was made and no previous rejected patch was restored.

## Next measurement

Collect a skill-active source-attribution window with this same build, then compare distributions. Native sampling plus callback diagnostics still requires an observer-cost control before accepting small performance gains. No optimization is accepted here. Any candidate must pass deterministic register/flags/memory and exit checks, preserve timer/event behavior, and improve both normal and skill legitimate workloads repeatably. A same-state whole-callback replay remains to be established; old SVC-bounded captures are secondary evidence.

Recording is stopped, the AVD adb daemon is restored to non-root, and the current game remains open. Raw data and exact ELF remain in ignored release-artifacts/native-callback-attribution-avd-2026-09-09/.

## Skill-active source sample

User repeatedly activated the visual skill during a 20-second 313 Hz native recording with simultaneous callback diagnostics. Recorder reports 1,958 samples and zero lost; report-sample exports 1,956, leaving two samples unaccounted for in export. Among exported samples: 748 fall in retained active timer polls in the emulator library, 156 in other modules, and 1,052 outside the retained active subset. Diagnostic CSV has zero lost records. All build-ID and clock checks pass.

Thumb match source location has 132/748 selected library samples (17.6%), versus 15.7% and 13.9% in the two normal samples. Memory-error inline path has 76/748 (10.2%), versus 11.7% and 9.1% normal. Register indexing, instruction fetch/page access, Thumb-mode checks and bit operations remain visible. No distinct skill-specific host API dominates this selected set.

125 of the 132 Thumb-match samples land at ELF address 0xb78bb8. Exact-build disassembly identifies `ldrh w16, [x14, x13, lsl #1]`: the dispatch-table entry load. Nearby instructions add the selected offset to the target base and branch indirectly. This is a stronger localization than the earlier whole-run hotspot, but sampling skid and dependencies still prevent treating these samples as a direct measurement of table-load latency. It does not establish that repeated decoding or a cache is the right solution.

Callback aggregates in this sampled session: 205 completed stages, mean CPU exclusive wall 24.844 ms, host wall 4.441 ms, 7.366 million steps, 8,131 CPU runs and 7,773 SVC exits; median CPU 20.129 ms, maximum 54.246 ms. The mixed normal/skill activity and different state make this an attribution window, not a replacement for the earlier matched-category cost baseline. Median/mean differences are not evidence of a code speedup; no execution code changed.

The attribution stage now has normal and skill evidence. Next work should be deterministic candidate evaluation, not another series of manually positioned baseline captures. Do not revive rejected fault-slot, CPSR-local, mode-inline or NOP-decoder edits without new evidence that addresses their prior failed benchmarks. The native source profile does not supply a guest-PC distribution, and same-state whole-callback replay remains a correctness/performance gate before accepting a live optimization.
