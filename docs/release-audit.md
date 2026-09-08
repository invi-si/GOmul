# GOmul 0.1.0-alpha.1 release preparation

Prepared 2026-09-08 in a separate checkout, preserving the private development installation. Based on WIE revision 1ed87109, with the upstream history retained. This is an experimental native Android ARM64 release; Windows desktop builds have not been validated.

## Included and excluded

Included: emulator/launcher source, synthetic tests and fixtures, vendored dependency sources with licences, licensed font/soundfont resources, Gradle build tooling, setup scripts, build and compatibility documentation.

Excluded: commercial game packages, all nine Action Hero data files, personal emulated identity values, device snapshots, saves, memory/replay captures, connection tokens, local configuration, logs, build caches, APKs in source history, and personal home-directory paths. See data-bundle-review.md for the separate bundle decision.

The scripted source/history audit examined over 10,000 working files and reachable historical blobs. It found no matches for its excluded paths/file types, personal home paths, GitHub credentials, or private-key patterns. This is a defined automated check, not a guarantee against every possible secret format. The two upstream Hello World archives, synthetic CPU fixtures, and Gradle wrapper are explicit non-game exceptions. APK contents were also inspected for bundled game/save archives and personal home paths.

## Validation completed

- `cargo fmt --all` and `cargo clippy --workspace`: passed (existing dead-code warnings remain).
- WIPI-C, CPU, LGT and KTF tests: 167 passed, none failed.
- CPU suite with native release Thumb inline/table features: 83 passed, none failed.
- Python checkpoint/setup tests: 11 passed.
- Web input tests: 16 passed.
- Native ARM64 APK built from this clean checkout, including generated licence notices.
- Installed and launched on a new empty API 36 ARM64 AVD; GOmul displayed an empty game library and the silver UI.
- Test SDK/JDK/Rust toolchains and dependency caches were reused; this was a fresh checkout/output directory and fresh AVD, not a freshly installed host OS.

Private game observations are documented separately and are not a claim that this clean release was freshly tested through every full game. The Mac checkpoint helper remains tied to its selected local AVD. No portable save-state support is advertised.

## Publication boundaries

Release assets contain the development-signed experimental APK and its checksum. Desktop/web source is included but unvalidated Windows binaries are not published. Original upstream deployment/coverage workflows were removed from this fork to avoid inheriting unrelated services and release destinations.

The configurable phone-identity change is being submitted as a separate focused upstream change with defaults preserved and regression tests. Broader performance/compatibility work should be submitted in additional focused PRs after separate upstream review preparation, rather than as a single launcher-sized patch.
