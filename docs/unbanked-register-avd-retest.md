# Archived register candidate: controlled AVD retest

## Decision

Do not promote to live validation. Both sweeps show lower median CPU cost on both
fixtures, but the full repeatability gate was not satisfied. Keep the candidate
archived and out of active source/production; do not repeat it without new evidence.
This is a failure to meet the acceptance criteria, not evidence of zero speedup.

The initial automated screen passed, but it does not test control stability:
normal control process medians had 14.36% CV. The confirmation control was much
steadier, but skill candidate/control process distributions overlapped, failing
the conservative automated screen. No measurements were discarded or corrected.

## Scope and method

Mac AArch64 Android AVD, emulator-5554, QEMU PID 10394. No physical Fold test.
Only the existing archived unbanked Index/IndexMut candidate was tested: bypass
mode/bank lookup for r0-r7, PC and CPSR; retain existing mapping for banked registers.
No CPU source, build flags, APK, timer, pacing or scheduling policy was changed.
The existing game was backgrounded with HOME and restored after benchmarking.

All four executables and both fixtures matched their archived SHA-256 manifest.
Frozen timing control SHA-256:
`1c7dee8deabccb6944de11d63c70d45289be4d07f57723e7ac181a951cbabcfc`.
Candidate SHA-256:
`4b816c217bb4d2d4fd059108bb1eecddc7867219371eafabdff53cae5d5cf3c3`.

Each sweep verified both fixtures with both verification executables before timing.
Each workload used ABBA twice, two warmups plus three measured rounds per process:
12 measured rounds per variant/workload/sweep, 96 measured rounds overall.
Host activation succeeded before all 40 verification/timing subprocesses; thread
state was recorded. Activation is not a guarantee of perfect host stationarity.
Timed executables reported timing_eligible=true, with profiling/count hooks absent.
Restore and validation were outside the CPU timers; per-segment clock overhead
was retained identically in both variants. No additional performance changes.

## Correctness

Both sweeps passed exact segment verification for control and candidate:

- Normal: 5,940,941 steps, 7,684 segments, 7,497 SVC exits.
- Skill: 9,072,858 steps, 8,519 segments, 8,012 SVC exits.
- Exact step counts, segment exits, complete CPU state including registers,
  CPSR/flags and banked state matched.
- Expected and actually dirtied pages matched before environmental restoration;
  every mapped page matched at completion.
- Every timed replay also passed CPU/exit and complete final-memory validation.

No source changes were made, so existing unit tests were not rerun. These are
fresh real-transcript correctness results, not a claim of a new full-suite run.

## CPU-only timing

Overall medians pool all 12 measured rounds per variant in each sweep. Control CV
is sample standard deviation / mean across its four process medians.

| Sweep | Fixture | Control ms | Candidate ms | CPU time reduction | Control process CV |
|---|---|---:|---:|---:|---:|
| Initial | normal | 28.611 | 26.279 | 8.15% | 14.36% |
| Initial | skill | 41.804 | 39.539 | 5.42% | 1.37% |
| Confirmation | normal | 28.725 | 26.071 | 9.24% | 1.31% |
| Confirmation | skill | 42.996 | 39.645 | 7.79% | 2.06% |

Individual process medians, in occurrence order within each variant:

| Sweep | Fixture | Variant | Process medians (ms) |
|---|---|---|---|
| Initial | normal | control | 37.280, 29.210, 28.223, 28.119 |
| Initial | normal | candidate | 27.319, 26.070, 26.202, 26.356 |
| Initial | skill | control | 42.834, 41.533, 41.880, 41.731 |
| Initial | skill | candidate | 39.524, 39.551, 41.137, 39.325 |
| Confirmation | normal | control | 29.006, 28.146, 28.605, 28.846 |
| Confirmation | normal | candidate | 27.258, 26.000, 26.010, 26.054 |
| Confirmation | skill | control | 43.588, 43.238, 41.565, 42.822 |
| Confirmation | skill | candidate | 39.687, 39.452, 41.712, 39.397 |

The confirmation skill candidate maximum process median (41.712 ms) exceeded
the control minimum (41.565 ms). Thus that sweep fails the stipulated
non-overlap screen despite its 7.79% overall median reduction. The first normal
control process median was 37.280 ms; all its raw samples remain included.
No gameplay FPS or input-latency benefit is established by these CPU-only results.

## Artifacts and next step

Private, Git-ignored raw outputs, hashes, activation records and summary files:
`release-artifacts/cpu-transcript/unbanked-activated-gate/` and
`release-artifacts/cpu-transcript/unbanked-activated-confirmation/`.
The archived patch/binaries remain in `release-artifacts/cpu-transcript/attribution/`.

The automated initial summary's eligible_for_live_validation=true is only its
threshold/non-overlap screen; it does not override the instability finding or
this combined decision. Leave production unchanged. A future CPU experiment
requires a separately evidenced candidate; do not combine this patch with another
change or repeat it merely to obtain a passing sweep.
