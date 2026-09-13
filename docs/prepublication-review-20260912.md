# Pre-publication review — 2026-09-12

The reviewed Android APK builds, installs and passes the checks below. This review fixed launcher defects and strengthened release checks; it did not change the native CPU engine, guest timers, scheduling or game pacing. Nothing was published as part of this review.

## Fixes

1. **Preserve existing progress during automatic startup-data import.** Launching an archive containing companion startup data could replace an older save folder that lacked the newer import marker. Automatic launch now retains existing progress. Identity/display settings and empty directories alone do not block a first-run import after resetting a save. Explicit replacement remains a separate user action. A regression test invokes the actual Android launcher with synthetic bundled data and verifies that unmarked earned progress survives.
2. **Make foreground state visible across threads.** The UI thread updates the foreground flag while the startup worker reads it to select the initial pause state. The flag is now volatile, removing the unsynchronized visibility assumption.
3. **Handle missing document-provider filenames.** Game import retains its fallback filename when a provider returns a null or empty display name.
4. **Repair diagnostics and lint.** API 36 documents `FRAME_TIMELINE_VSYNC_ID`, but its metric annotation omits the constant. The SDK-gated call now uses a narrowly scoped, documented lint workaround. Drawing trace sections close in a `finally` block. See the [Android API reference](https://developer.android.com/reference/android/view/FrameMetrics#FRAME_TIMELINE_VSYNC_ID).
5. **Finish small UI release details.** Added a native vector launcher icon, Korean keypad accessibility labels and a Korean emulated-identity hint. Control positions remain unchanged.
6. **Update release documentation and checks.** The README points to current compatibility inventories instead of the old six-game summary. Documentation distinguishes native checkpoints from historical AVD-only snapshots. The prerelease workflow now includes formatting, workspace/production-feature tests, compatibility tests, all standalone Java helper tests, release-file/history scanning and Android lint.

## Validation

| Check | Result |
| --- | --- |
| Rust workspace tests | 500 passed; zero failures |
| Production CPU feature configuration | 86 passed; zero failures; overlaps workspace coverage |
| Rust formatting / Clippy | Passed; Clippy retains existing warnings |
| Python compatibility, input-trace, Mac-checkpoint and provenance tests | 35 passed |
| Standalone Java import, save-reset and controller suites | All three passed |
| Web input tests | 16 passed |
| Android build, test APK and lint | Passed; lint: zero errors, seven warnings |
| Actual launcher startup-data preservation regression | Passed on the Mac-hosted Android environment |
| Rescue export and intentional-stop handling | Passed with synthetic diagnostic data |
| Audio timing conversion instrumentation | Passed |
| Native boot/pause/quick-save/quick-load/stop | Three rounds passed using isolated temporary saves |
| APK signature and 16 KB ZIP alignment | Passed |
| APK native library comparison | Byte-identical to the pre-review native library |
| APK assets | License notices and provenance only; no bundled game archives or saves |
| Release-file and HEAD-reachable history scanner | Zero findings |

The three lifecycle rounds reached Running state with paints after load. Mapped-region counts were 2941, 2947 and 2947; RSS was approximately 394, 396 and 396 MiB. This bounded check did not show continuing growth between the final two rounds, but is not a long-session leak test. User saves were not used for this exercise.

## Artifact

Reviewed APK SHA-256:

```text
15dd065c3cf02b8ecc01bdee290c00c2843dc0af6e0e5cb3d960c69a8c9049ec
```

The APK was installed in the existing Android test environment and the library screen checked. The current compatibility inventories retain the user's classifications; this review does not claim a new full playthrough of every game.

## Publication follow-up and limits

- **Use a persistent release signing key before establishing the public Android update line.** The existing workflow builds a debug-signed experimental APK. Fresh machines can generate different debug keys, preventing an ordinary in-place update. Version codes must also increase for subsequent releases. Preserve the signing identity deliberately; do not commit a private key or credentials. See [Android app signing](https://developer.android.com/studio/publish/app-signing).
- Seven non-blocking lint warnings remain: a profiling attribute on older Android versions, a Gradle patch update, an API-annotation recommendation, backup-rule guidance, keypad touch accessibility and two string-resource suggestions. The physical/touch keypad still warrants a dedicated accessibility pass before claiming full screen-reader support.
- This was a focused code and automated regression review, not proof that every execution path is correct. No fresh Fold 5 or Thor run, full-game endurance test, Windows build or Mac launcher build was performed in this stage.
- The release scanner checks non-ignored working files and history reachable from HEAD for excluded paths, personal home paths, known GitHub credential patterns and private keys. It is not an exhaustive secret detector or an audit of every Git reference. Ignored local artifacts are not release inputs.

Current inventory references: [Android environment](android-compatible-library-20260912.md), [LGT compatibility](lgt-compatibility-20260912.md), and [Dragon Road's in-game speed setting](dragon-road-speed-setting.md).
