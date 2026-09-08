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
cargo build --locked --release -p wie-android --target aarch64-linux-android --features experimental-thumb-inline,experimental-thumb-table
mkdir -p wie-android/android/app/src/main/jniLibs/arm64-v8a
cp target/aarch64-linux-android/release/libwie_android.so wie-android/android/app/src/main/jniLibs/arm64-v8a/
python3 wie-android/generate-notices.py
"$root/wie-android/android/gradlew" -p "$root/wie-android/android" :app:assembleDebug
