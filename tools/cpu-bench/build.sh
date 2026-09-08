#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
output_dir=${1:?Usage: build.sh OUTPUT_DIRECTORY [off profiled throughput table pc_local] (requires Rust targets, NDK_HOME and wasm-opt)}
shift
if [ "$#" -eq 0 ]; then set -- off profiled throughput; fi
variants="$*"
for variant in "$@"; do
  case "$variant" in off|profiled|throughput|table|pc_local|blocks) ;; *) echo "Unknown variant: $variant" >&2; exit 1 ;; esac
done
: "${NDK_HOME:?Set NDK_HOME to the installed Android NDK}"
wasm_opt=${WASM_OPT:-wasm-opt}
case $(uname -s) in
  Darwin) ndk_host=darwin-x86_64 ;;
  Linux) ndk_host=linux-x86_64 ;;
  *) echo "This build script supports macOS and Linux hosts" >&2; exit 1 ;;
esac
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$NDK_HOME/toolchains/llvm/prebuilt/$ndk_host/bin/aarch64-linux-android24-clang"
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-"$repo_root/target"}
export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$repo_root=/src/wie"
mkdir -p "$output_dir/cpu-bench" "$output_dir/native"
output_dir=$(CDPATH= cd -- "$output_dir" && pwd)
cd "$repo_root"

for variant in $variants; do
  case "$variant" in
    profiled) set -- --features profiling ;;
    throughput) set -- --features throughput ;;
    table) set -- --features experimental-thumb-table ;;
    pc_local) set -- --features experimental-pc-local ;;
    blocks) set -- --features experimental-thumb-blocks ;;
    off) set -- ;;
  esac
  cargo build --locked --release -p wie-cpu-bench --lib --target wasm32-unknown-unknown "$@"
  "$wasm_opt" "$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/wie_cpu_bench.wasm" \
    -O --enable-bulk-memory --enable-nontrapping-float-to-int \
    -o "$output_dir/cpu-bench/$variant.wasm"
  cargo build --locked --release -p wie-cpu-bench --bin wie-cpu-bench --target aarch64-linux-android "$@"
  cp "$CARGO_TARGET_DIR/aarch64-linux-android/release/wie-cpu-bench" "$output_dir/native/wie-cpu-bench-$variant"
done
cp "$repo_root/tools/cpu-bench/index.html" "$repo_root/tools/cpu-bench/runner.js" "$output_dir/cpu-bench/"
printf '%s\n' "$variants" > "$output_dir/cpu-bench/variants.txt"
cp "$repo_root/LICENSE" "$output_dir/LICENSE"
printf 'Diagnostic assets: %s/cpu-bench\nAndroid executables: %s/native\n' "$output_dir" "$output_dir"
