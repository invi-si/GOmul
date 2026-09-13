#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
: "${ANDROID_HOME:?Set ANDROID_HOME}"
: "${NDK_HOME:?Set NDK_HOME}"
case "$(uname -s)" in
  Darwin) ndk_host=darwin-x86_64 ;;
  Linux) ndk_host=linux-x86_64 ;;
  *) echo "Build the native APK on macOS or Linux." >&2; exit 1 ;;
esac
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$NDK_HOME/toolchains/llvm/prebuilt/$ndk_host/bin/aarch64-linux-android26-clang"
cd "$root"
export GOMUL_CHECKPOINT_BUILD=$(python3 - <<'PYBUILD'
import hashlib, os, pathlib, subprocess
h = hashlib.sha256()
paths = subprocess.check_output(['git', 'ls-files', '-co', '--exclude-standard', '-z']).split(b'\0')
for name in sorted(set(paths)):
    if not name:
        continue
    p = pathlib.Path(name.decode())
    if p.suffix == '.rs' or p.name in ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml'):
        h.update(name + b'\0' + p.read_bytes())
h.update(b'arm64-release-thumb-inline-thumb-table-v1')
if os.environ.get('GOMUL_COMPATIBILITY_AUDIT') == '1':
    h.update(b'compatibility-audit-v1')
print(h.hexdigest())
PYBUILD
)
features=experimental-thumb-inline,experimental-thumb-table
if [ "${GOMUL_COMPATIBILITY_AUDIT:-0}" = 1 ]; then
    features="$features,compatibility-audit"
fi
cargo build --locked --release -p wie-android --target aarch64-linux-android --features "$features"
mkdir -p wie-android/android/app/src/main/jniLibs/arm64-v8a
cp "${CARGO_TARGET_DIR:-target}/aarch64-linux-android/release/libwie_android.so" wie-android/android/app/src/main/jniLibs/arm64-v8a/
python3 wie-android/generate-notices.py
if [ "${GOMUL_THOR:-0}" = 1 ]; then
    set -- -PgomulThor
else
    set --
fi
"$root/wie-android/android/gradlew" -p "$root/wie-android/android" "$@" :app:assembleDebug
