# Building GOmul

Clone this repository and install Rust stable (edition 2024), Python 3, Node.js 24+ for web builds, and the platform prerequisites below. Commands run at the repository root. Generated outputs and local configuration stay outside Git.

## Native Android APK (macOS or Linux)

Install JDK 21, Android SDK platform 36/build tools, and NDK 29 using Android's official tools. Review and accept their licences yourself. Set `JAVA_HOME`, `ANDROID_HOME`, and `NDK_HOME` to your installations; put Java and Android platform-tools on PATH.

```sh
rustup target add aarch64-linux-android
./wie-android/build.sh
```

Output: `wie-android/android/app/build/outputs/apk/debug/app-debug.apk`.
This is a development-signed experimental APK, not a production-signed store release. Native builds enable the tested experimental Thumb inline/table features; the ordinary CPU interpreter remains the fallback. No game is included.

For a physical device, install with `adb install -r` or open the APK on the phone. For an isolated test, create a new ARM64 API 36 Android Virtual Device in Android Studio, start it, and install the APK there. Do not reuse a device containing valuable saves for initial testing.

## Optional Mac AVD checkpoints

Install the APK on a running AVD, open GOmul once, then:

```sh
python3 tools/mac-checkpoints/install-bridge.py --avd YOUR_AVD_NAME
```

The installer locates `adb` from PATH (or `--adb`) and prints your local configuration path. It creates a private token locally, provisions only the selected AVD, and installs a macOS LaunchAgent. The helper supports one selected AVD per Mac user. It uses an ADB tunnel and needs no guest internet connection.

Quick Save and Quick Load appear beside CALL; hold Quick Load to undo. Slots are independent by archive hash. Loading restores the active game's captured memory and save files while preserving the current files of other games. Android OS state and the app version still come from the captured whole-device snapshot. Keep that AVD and helper available; emulator/app upgrades can invalidate snapshots.

The helper lives under `~/Library/Application Support/GOmul/`. Stop it with:

```sh
launchctl bootout "gui/$(id -u)/io.github.invi-si.gomul.checkpoints"
```

## Web and desktop source

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked
npm ci
npm run build:prod
```

For Tauri desktop builds install the platform prerequisites documented by Tauri, then `cargo install tauri-cli --locked` and run `cargo tauri build` from `wie-app`. These frontends are experimental and separate from the native Android UI. Windows binaries require validation on Windows before being advertised as supported releases.

## Checks

```sh
cargo fmt --all --check
cargo clippy --workspace
cargo test -p wie-wipi-c -p wie-core-arm -p wie-lgt -p wie-ktf
python3 -m unittest discover -s tools/mac-checkpoints -v
node --test wie-web/tests/game_input.test.mjs
python3 scripts/release-audit.py
```

Regenerate third-party notices after dependency changes with `python3 scripts/generate-rust-notices.py` and `python3 wie-android/generate-notices.py`.
