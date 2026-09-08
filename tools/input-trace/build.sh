#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$root"
: "${ANDROID_HOME:?Set ANDROID_HOME}"
: "${NDK_HOME:?Set NDK_HOME}"
case "$(uname -s)" in
 Darwin) host=darwin-x86_64 ;;
 Linux) host=linux-x86_64 ;;
 *) exit 1 ;;
esac
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$NDK_HOME/toolchains/llvm/prebuilt/$host/bin/aarch64-linux-android26-clang"
features=input-trace,experimental-thumb-inline,experimental-thumb-table
case "${1:-}" in
 "") ;;
 --transcript) features="$features,cpu-transcript-capture" ;;
 *) echo "Usage: $0 [--transcript]" >&2; exit 2 ;;
esac
export GOMUL_CAPTURE_FEATURES="$features"
export GOMUL_CHECKPOINT_BUILD=$(python3 - <<'PY'
import hashlib,pathlib,subprocess,os
h=hashlib.sha256()
for n in sorted(set(subprocess.check_output(['git','ls-files','-co','--exclude-standard','-z']).split(b'\0'))):
 if n:
  p=pathlib.Path(n.decode())
  if p.suffix=='.rs' or p.name in ('Cargo.toml','Cargo.lock','rust-toolchain.toml'):h.update(n+b'\0'+p.read_bytes())
h.update(os.environ['GOMUL_CAPTURE_FEATURES'].encode());print(h.hexdigest())
PY
)
cargo rustc --locked --release --lib -p wie-android --target aarch64-linux-android --features "$features" -- -C link-arg=-Wl,--build-id=sha1
mkdir -p wie-android/android/app/src/main/jniLibs/arm64-v8a
cp "${CARGO_TARGET_DIR:-target}/aarch64-linux-android/release/libwie_android.so" wie-android/android/app/src/main/jniLibs/arm64-v8a/
python3 wie-android/generate-notices.py
wie-android/android/gradlew -p wie-android/android -PgomulTrace :app:assembleRelease
