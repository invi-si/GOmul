# GOmul (고물)

A Korean feature-phone emulator based on [WIE](https://github.com/dlunch/wie) by Inseok Lee (dlunch) and contributors. GOmul adds native Android launcher work, compatibility fixes, CPU experiments, and optional Mac-hosted Android checkpoint controls.

**First release (0.1.0). Bring your own games.** No commercial games, game saves, authentication data bundles, or device snapshots are distributed here. The original MIT copyright and licence remain in [LICENSE](LICENSE). See [third-party notices](THIRD_PARTY_NOTICES.md).

## What runs where

- **Native Android ARM64:** game library, JAR/ZIP import, automatic companion-data setup, rotation, virtual controls and silver phone-style UI. Compatibility varies by game; this is not a complete WIPI implementation.
- **Android Virtual Device on macOS:** the same APK plus an optional local checkpoint helper. The optional helper uses whole-AVD Quick Save/Load and protected startup snapshots. Physical Android has a separate experimental [replay checkpoint implementation](docs/android-checkpoints.md), with slower loads and compatibility limits.
- **Web / Tauri desktop:** experimental source frontends inherited from WIE and extended in this fork. They do not have the native Android checkpoint integration. Windows release binaries have not been validated locally.

[Build instructions](docs/build.md) · [Local Action Hero setup](docs/action-hero-setup.md) · [Compatibility](docs/compatibility.md) · [Release audit](docs/release-audit.md)

## Development since 0.1.0

The source now includes WIPI timer cancellation correctness, opt-in input/timer diagnostics, deterministic CPU replay and regression tests. See the [performance research index](docs/performance-research.md) for findings, rejected experiments and measurement limits. The refreshed 0.1.0 APK (Android version code 7) includes the timer fix; see the release notes for checkpoint compatibility.

## Tested games

These observations come from private testing with user-supplied copies. Different carrier releases may behave differently.

| Game | Observed status |
| --- | --- |
| Reicarna / 레이카르나 | Playable; heavily exercised during optimization work. |
| Mini Game Paradise 1 / 미니게임천국1 | Reported working in manual testing. |
| Mini Game Paradise 2 / 미니게임천국2 | Reported working in manual testing. |
| Super Action Hero 3 / 슈퍼액션히어로3 (LGT) | Main menu reached with matching emulated phone identity and user-supplied data. Full gameplay not certified. |
| Gamevil 2010 Pro Baseball / 게임빌2010프로야구 | Fresh-save startup works; existing-save authentication problems remain. |
| NOM 3 / 놈3 | Boots; reported display flickering still needs confirmation. |

## Contributing

Report carrier/version, platform, reproduction steps, and whether a fresh save changes the result. Do not attach commercial archives, snapshots, personal phone identities, or memory dumps to public issues. Synthetic reproductions and code fixes are welcome.

General runtime fixes should be proposed upstream as focused changes with regression tests. GOmul-specific launcher and checkpoint work belongs in this fork. There is no affiliation with phone manufacturers, carriers, game publishers, or an endorsement by the WIE author.

Download the Android ARM64 APK from [GOmul 0.1.0 — First Release](https://github.com/invi-si/GOmul/releases/tag/v0.1.0). Standalone replay checkpoints remain experimental; see their [limits and validation](docs/android-checkpoints.md).
