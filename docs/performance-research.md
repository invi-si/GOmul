# Performance research and timer correctness

This page summarizes the investigation after GOmul 0.1.0. Detailed reports below
are chronological lab notes: earlier pending steps and hypotheses may be resolved
by later sections or reports. Published source includes reproducible tools and
synthetic tests; commercial-game transcripts, saves and raw traces stay private.

## Current conclusions

- **Fix retained:** WIPI pending timer cancellation. Previously canceled timer
  registrations could enter guest code and rearm, creating duplicate callback
  chains. Registrations are now tracked in guest-backed memory and consumed or
  skipped immediately before callback entry, including deferred registrations.
- **Measured effect:** a small sequential AVD comparison reduced input queue wait
  from roughly 209–223 ms to 5.3 ms. This is queue latency, not screen-response
  latency. Clean-start validation observed six canceled registrations, all skipped,
  with no callback entry or rearm from them.
- **Cadence:** approximately 10–11 guest paints/sec after the fix is an observation,
  not an original-phone accuracy claim. The game rearms after callback work with a
  requested 60 ms delay. Earlier 30–44 paints/sec included duplicated work; raising
  FPS is not the correctness objective.
- **CPU tooling:** deterministic callback transcripts verify segment step counts,
  exits, CPU/banked state, changed pages and complete final memory. Instrumented
  verification/attribution builds are separate from timing builds.
- **Rejected experiments:** existing Thumb blocks were 29–32% slower on the two
  captured workloads. The later register-indexing candidate showed median gains
  but failed the combined stability/repeatability gate and remains archived. Neither
  is promoted to production by this update.
- **Measurement environment:** macOS AVD host activity materially affected CPU
  timing. Explicit activation and recorded host state improve control, but every
  comparison must still check variance. AVD timings are not Fold 5 benchmarks.

No timer delays, refresh caps or CPU scheduling policy were adjusted to inflate
performance. Future CPU candidates must pass both deterministic correctness
fixtures and repeatable timing comparisons before live validation.

## Reading order

| Report | Question answered |
|---|---|
| [Input-to-presentation trace](input-trace-avd-report.md) | Where was the observed delay? |
| [Queue blocker ledger](queue-blocker-ledger-report.md) | What preceded physical input in the queue? |
| [Timer census](timer-census-avd-report.md) | Why were multiple update chains alive? |
| [Cancellation fix and clean-start validation](timer-cancellation-fix.md) | Are canceled registrations prevented from entering/rearming? |
| [Normal versus skills](timer-normal-vs-skill-avd-report.md) | What cadence survives after the fix? |
| [Callback CPU cost](callback-cost-avd-baseline.md) | How much legitimate guest work remains? |
| [Native callback attribution](native-callback-attribution-avd.md) | Which interpreter paths receive samples? |
| [Deterministic callback transcripts](callback-cpu-transcript.md) | How can CPU candidates be tested without replaying gameplay manually? |
| [Transcript attribution](transcript-cpu-attribution.md) | What do exact counts and repeated samples establish? |
| [AVD control stability](control-stability-avd.md) | Why did the same binary enter radically different timing regimes? |
| [Register candidate retest](unbanked-register-avd-retest.md) | Did the archived candidate pass under controlled host activation? |

## Tools and publication limits

[Diagnostic build/capture instructions](../tools/input-trace/README.md) cover
opt-in input tracing, timer census, queue reconstruction and callback accounting.
The normal Android package remains `local.wie.nativeapp`; `-PgomulTrace` selects
the isolated, development-signed, shell-profileable diagnostic package. Rust
`input-trace`, transcript capture/verification and exact-count features are opt-in.
No private game fixture is required for the included synthetic regression tests.

The CPU transcript and control-stability reports document the benchmark tools in
`tools/cpu-bench/`. Raw memory fixtures cannot be redistributed here; exact private
workload results are documented observations, not publicly downloadable fixtures.
Do not attach them to issues or pull requests. Existing copyright/license notices
and WIE attribution remain unchanged. This source update does not replace the
published 0.1.0 APK; that binary predates the timer fix.

## Source publication checks

Before publication, `cargo fmt` and `cargo clippy --workspace` completed (Clippy
retains warnings; this is not a warning-free claim). Default tests passed for
Android (10), backend (52), ARM core (83), utilities (3), and WIPI-C (46).
The exact-count ARM core configuration passed 93 tests; standalone CPU exact
counts passed 36 tests and one doctest. The trace-enabled package tests and four
focused MIDP event-queue tests also passed. Five Python analysis tests passed.

Android ARM64 capture-feature checking and Java/manifest compilation passed for
both the normal debug configuration and opt-in trace release configuration.
Manifest inspection confirmed the normal app ID/label and disabled shell
profiling, with separate trace identity/profileability when requested.
No APK was installed or released during source publication.

Two previously reproduced baseline MIDP full-suite hangs (timed alert without a
previous screen and ticker screen retention) remain documented limitations;
a full workspace test pass is not claimed. New source/report files were checked
for binary archives, personal home paths, known identity data, private keys and
common GitHub token formats. Raw artifacts remained ignored and were not staged.
