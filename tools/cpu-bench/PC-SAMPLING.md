# Native Android PC sampling

`android_pc_sample` statistically samples its own native AArch64 program counter with `SIGPROF` and `ITIMER_PROF`. It uses the same live engine and deterministic workloads as the ordinary benchmark. Guest instruction execution has no profiling counters, stage clocks, decoder changes or scheduling changes. This diagnoses native synthetic workloads; it cannot by itself establish the cost distribution of a WebAssembly game.

Build with only `pc-sampling`, using the Android NDK linker and a separate target/output directory. Keep debug information for exact source attribution:

```sh
CARGO_PROFILE_RELEASE_DEBUG=1 cargo build --locked --release \
  -p wie-cpu-bench --features pc-sampling --example android_pc_sample \
  --target aarch64-linux-android
```

Compile `examples/android_pc_layout.c` with the same NDK's `aarch64-linux-android24-clang -fsyntax-only` to independently verify the signal-context ABI. The Rust example also asserts these offsets at compile time. NDK AArch64 places `uc_mcontext` at byte 176 and its `pc` at byte 264, giving byte 440 from the signal context pointer. The Android AArch64 `ucontext_t` in libc 0.2.189 omits the kernel's sigmask padding, so the example supplies that outer layout explicitly. The first diagnostic binary used that incorrect binding and read x16; its version 1 samples are invalid for source attribution, including addresses that happen to resolve inside the executable. Version 2 reports `contextPcOffset: 440`. The symbolization helper rejects version 1.

The CLI accepts workload, measured batches, sampled flag, warmup batches and requested interval in microseconds:

```sh
android_pc_sample 0 30000 0 1000 1000
android_pc_sample 0 30000 1 1000 1000
```

One batch executes 10,000 guest instructions. Workloads 0–2 are Thumb add/branch, Thumb load/add/store/branch and ARM add/branch. Run the same executable with sampled 0 and 1, balance the order, and repeat each condition. Save the actual binary hash and raw JSON. Compare both elapsed wall time and process CPU time before using any sample distribution. Thermal state and competing foreground work matter. Android can quantize this CPU timer: report actual delivered samples per CPU second, rather than assuming the requested interval was honored. Periodic sampling of a tiny repeated loop may alias; repeat windows and intervals, and treat source lines as compiler attribution rather than exact machine-stage boundaries.

Warmup, memory setup, signal setup, buffer prefaulting, timer teardown and result serialization occur outside the measured execution window. The signal handler only reads the saved PC and updates a fixed, preallocated array of lock-free AArch64 atomics. It does not allocate, lock, read a clock, invoke libc or symbolize. The diagnostic rejects multiple threads and an already blocked or pending SIGPROF. A scope guard restores the previous timer, handler and mask. Every successful run checks all 17 architectural registers and every byte of the 128 KiB memory fixture. Invalid contexts, buffer drops, no samples and restoration failure are reported explicitly.

Symbolize only against the exact debug ELF verified on the device:

```sh
python3 tools/cpu-bench/symbolize_pc_samples.py \
  --binary /path/to/exact/android_pc_sample \
  --expected-sha256 HASH_VERIFIED_ON_DEVICE \
  --symbolizer /path/to/ndk/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-symbolizer \
  --output /path/to/attributed.json /path/to/sample.json /path/to/control.json
```

The helper checks architectural validation, sample totals, ASLR-relative address arithmetic and the executable's `run_measured` anchor. It attributes each executable sample to its innermost inline frame once; the full inline stack remains available for inspection. Percentages explicitly distinguish executable samples from all samples. Unknown or other-module PCs remain visible rather than silently disappearing from the denominator. No source stage clock percentages are mixed with the PC samples.
