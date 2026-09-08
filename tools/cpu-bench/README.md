# Deterministic CPU diagnostics

This package depends on the live `wie-core-arm` engine and the workspace's interpreter dependency. It does not copy the engine, load game code, open game storage, or change production launchers. Builds select instrumentation-free execution (`off`), full optional stage instrumentation (`profiled`), or lightweight instruction totals (`throughput`); release logging is capped at WARN throughout.

The synthetic programs are the same three workloads used in the initial investigation:

| ID | Workload | Repeating instructions | Expected result after N instructions |
| --- | --- | --- | --- |
| 0 | `thumb_add` | Thumb ADD, branch | R0 = N / 2 |
| 1 | `thumb_memory` | Thumb load, ADD, store, branch | R1 and word at 0x20000 = N / 4 |
| 2 | `arm_add` | ARM ADD, branch | R0 = N / 2 |

One batch executes exactly 10,000 instructions. Each trial uses a fresh engine, maps 128 KiB, writes its program and initializes registers before starting the host clock. After the clock stops, validation checks R0–R15, CPSR, and every byte of mapped memory against the expected image, including unchanged code and zero padding. Profiling snapshots are taken before validation reads can affect boundary counters. A failed result is an error, never a timing sample.

## Build and check

From the repository root, install the `wasm32-unknown-unknown` and `aarch64-linux-android` Rust targets. Set `NDK_HOME` and make Binaryen's `wasm-opt` available, or set `WASM_OPT` to its executable. Use the same Rust release and Binaryen settings for both variants.

```sh
cargo test -p wie-cpu-bench
cargo test -p wie-cpu-bench --features profiling
cargo test -p wie-cpu-bench --features throughput
sh tools/cpu-bench/build.sh /path/to/diagnostic-output
```

Output includes `cpu-bench/{index.html,runner.js,variants.txt,off.wasm,profiled.wasm,throughput.wasm}` and three Android ARM64 native executables. Copy `cpu-bench` only into the separate profiling app's staged frontend; do not put it in `wie-web/public` or the normal launcher package. Retain the workspace and dependency license notices when distributing these binaries.

The optional Thumb decoder experiment is disabled by default. Build a separate pair with `sh tools/cpu-bench/build.sh /path/to/table-experiment off table`. These variants have no full profiling or throughput counters; the table build enables only `experimental-thumb-table`. Reports identify this with `experimental_thumb_table: true` for `table` and `false` for `off`. Both must report `full_profiling_compiled: false`, `counters: null` and `throughput: null`. Run the same fixed instruction counts using `variant: "off"` and `variant: "table"`, or native executables `wie-cpu-bench-off` and `wie-cpu-bench-table`. The `variants.txt` manifest limits the page's correctness checks to the files actually built. A separate full-game candidate can be built with `WIE_EXPERIMENTAL_THUMB_TABLE=1 WIE_CPU_PROFILING=0 WIE_CPU_THROUGHPUT=0 npm run build:prod`; leave the experimental variable unset for the control. Synthetic native results alone do not establish a WebView or game improvement.

The independent PC-local experiment reuses the already captured sequential program counter and is also disabled by default. Build it separately with `sh tools/cpu-bench/build.sh /path/to/pc-local-experiment off pc_local`; select `variant: "pc_local"` or `wie-cpu-bench-pc_local`. Its report must have `experimental_pc_local: true`, `experimental_thumb_table: false`, and no full profiling or throughput counters. Use the matching new `off` artifact from the same output directory. Do not combine the two experimental features when attributing a measured change to either one. `WIE_EXPERIMENTAL_PC_LOCAL=1` enables the corresponding optional game build; keep the table and diagnostic environment flags unset or `0` for that comparison.

With the existing Tauri Android project initialized and the Android/Rust build environment configured, the optional wrapper builds a separate debug shell around a staged frontend:

```sh
node tools/cpu-bench/build-apk.mjs /path/to/staged-frontend /path/to/WIE-CPU-Diagnostics.apk
```

The staged frontend must contain the `cpu-bench` output directory. The wrapper uses `tauri` on `PATH` (or `TAURI_BIN`) and the installed SDK at `ANDROID_HOME`. It enables a guarded Gradle script with `wieDiagnostic=true`, keeping the JNI namespace unchanged and assigning the application ID `local.wie.launcher.profile`. Tauri rewrites suffix assignments in its generated main Gradle file, so the guard lives in a separately applied script. The wrapper checks the final APK's package ID and debug flag before copying it to the requested output. It never installs the APK. Use an output path separate from normal launcher artifacts, and verify signing and device compatibility before installation.

The native invocation emits one JSON trial, with no emulator work in process setup or result serialization included in `elapsedMs`:

```sh
wie-cpu-bench-off WORKLOAD BATCHES MODE WARMUP_BATCHES INTERVAL SEED
wie-cpu-bench-profiled WORKLOAD BATCHES MODE WARMUP_BATCHES INTERVAL SEED
wie-cpu-bench-throughput WORKLOAD BATCHES 0 WARMUP_BATCHES INTERVAL SEED
```

Defaults are workload 0, 100 batches, mode 0, 100 warmup batches, sampling interval 4096, seed 1. Modes are 0 off, 1 counts, 2 sampled. The build without instrumentation accepts only mode 0. Select the physical device explicitly when pushing executables into `/data/local/tmp` and invoking them through `adb`; this script does not select, install on, or control a device.

The throughput build also uses mode 0: its instruction totals are always enabled and it has no stage sampling modes or host clock imports. Reports include `throughput` with exact ARM, Thumb and total instruction counts, and `full_profiling_compiled: false`. Reset happens before each trial, and totals are checked independently against the fixed program. Reject any throughput-only overhead sample reporting that full profiling was compiled in: Cargo feature unification can otherwise accidentally turn a lightweight control into the heavier build.

## WebView protocol

The diagnostic page at `/cpu-bench/index.html` exposes:

```js
await window.wieBench.runTrial({
  variant: "profiled", // "off", "profiled", "throughput", "table", or "pc_local"
  mode: "sampled",    // "off", "counts", "sampled"
  workload: "thumb_memory",
  batches: 100,
  warmupBatches: 100,
  interval: 4096,
  seed: 1,
});
```

The response contains `elapsedMs`, `guestInstructions`, full-state `validation`, and `counters`. Fetching, Wasm compilation, instantiation, memory setup, warmup and report generation happen outside the measured call. Native timing uses Rust's monotonic `Instant`; WebView timing uses `performance.now()`. Sampled-stage clocks use the same host monotonic clocks in nanoseconds, never the guest clock.

For lightweight totals, call the same API with `variant: "throughput", mode: "off"`. The response's `throughput.total_instructions` must equal `guestInstructions`, and both full-profiling flags must be false. Compare this variant with `off` using equal instruction counts before using it to report game instruction throughput.

Measure the actual clock path with `await window.wieBench.calibrateClock({ pairs: 10000 })` and `wie-cpu-bench-profiled --calibrate 10000`. These execute adjacent clock calls inside Rust; the Wasm version crosses into JavaScript for both calls. Reports include zero-pair counts, regressions and delta percentiles. `loopElapsedNs`, `loopPairs` and `meanLoopPairNs` measure the complete paired-call loop before sorting and serialization; this includes loop bookkeeping and vector writes. A high zero-pair fraction means short stages are below the clock's resolution; do not treat their zero timings as zero work or subtract calibration percentiles from individual stage samples.

Calibrate batches so measured blocks last roughly 0.5–1 second on the test device. Then keep batches fixed for all comparisons. Warm both variants and balance trial order: compare A/B/B/A for instrumentation-free versus compiled-in-but-off, then compiled-in-off versus counts, then compiled-in-off versus sampled. Use at least four observations per cell; keep instruction counts, sampling interval and seed equal in paired native/Wasm trials. Record device and WebView versions, thermal and battery state, and the actual order separately. Avoid building concurrently, enabling the debugger, or mixing JIT warmup with timing. Sampling overhead and noise must be measured before interpreting stage times as bottlenecks.

## Matching a Mac Node comparison

`node-runner.mjs` reuses the same raw Wasm files and `runner.js`, with local file loading replacing browser fetching. Supply a JSON plan containing `batches` keyed by the three workload names, `warmupBatches`, `interval`, `seed`, `pauseMs`, and `balancedOrder` (arrays using each condition index once). Default conditions are compile-off, profiled/off, profiled/counts and profiled/sampled. An optional `conditions` array can instead select `[{"variant":"off","mode":"off"},{"variant":"throughput","mode":"off"}]`, with balanced orders `[0,1]` and `[1,0]`. Copy the fixed batch plan from the device run instead of recalibrating it for the Mac.

```sh
node --no-liftoff --no-wasm-lazy-compilation tools/cpu-bench/node-runner.mjs \
  /path/to/diagnostic-output/cpu-bench /path/to/plan.json /path/to/mac-results.json
```

Check that the installed Node version supports these V8 flags. They force optimizing compilation for this separate host comparison; they do not describe the ordinary Android WebView's compilation policy. The report records Node/V8 versions, flags, host CPU, Wasm hashes, the exact plan, validation and every trial. Run it while builds and other CPU-heavy tasks are idle.

For the table pair, set conditions to `[{"variant":"off","mode":"off"},{"variant":"table","mode":"off"}]` and use balanced orders `[0,1]` and `[1,0]`. Clock calibration is omitted by default when no profiled condition is selected; it requires a separately built `profiled.wasm` and can be requested with `clockCalibration: true` in the plan. Artifact hashes always cover every selected variant.
