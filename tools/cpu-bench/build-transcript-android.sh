#!/bin/sh
# Four separate binaries: correctness gate and uninstrumented timing, for both
# the current interpreter and the existing default-off Thumb block candidate.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$root"
: "${NDK_HOME:?Set NDK_HOME}"
case "$(uname -s)" in
 Darwin) host=darwin-x86_64 ;;
 Linux) host=linux-x86_64 ;;
 *) exit 1 ;;
esac
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$NDK_HOME/toolchains/llvm/prebuilt/$host/bin/aarch64-linux-android26-clang"
out="$root/release-artifacts/cpu-transcript"
mkdir -p "$out"
for variant in control-verify blocks-verify control blocks; do
 features=replay,experimental-thumb-inline,experimental-thumb-table
 case "$variant" in *verify) features="$features,transcript-verify" ;; esac
 case "$variant" in blocks*) features="$features,experimental-thumb-blocks" ;; esac
 cargo build --locked --release -p wie-cpu-bench --example transcript --target aarch64-linux-android --features "$features"
 cp "${CARGO_TARGET_DIR:-target}/aarch64-linux-android/release/examples/transcript" "$out/$variant"
 printf '%s\n' "$features" > "$out/$variant.features.txt"
done
