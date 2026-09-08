# Optional interpreter profiling

This directory vendors `arm32_cpu` 0.1.0 from crates.io, retaining its MIT license,
authors, sources and executable test fixtures. `UPSTREAM.sha256.json` records the
original package's file hashes. Local changes add observations only; instruction
implementations, decode order, guest memory and guest time are not optimized or
changed. The original `Cargo.toml.orig` describes the upstream package.

The `profiling` feature enables `Cpu::set_profiling(mode, mean_interval, seed,
clock)`, `reset_profiling()` and `profiling_snapshot()`. Without that feature,
profiling fields, hooks and types are removed at compile time. `Cpu` retains its
original register-only layout. With the feature enabled, architectural equality
and CPU serialization exclude the profiler; deserialization starts with profiling
Off. Public snapshots derive Serialize, using the upstream optional
`serde-serialize` dependencies. No heap allocation occurs in the profiler.

Modes:

* `Off`: no counters or host clock reads. Feature-enabled branch overhead may
  remain, so compare this separately with a build that omits the feature.
* `Counts`: event and instruction counters, without host clock reads.
* `Sampled`: the same counters plus stage timing on a sparse instruction subset.
  A deterministic xorshift sequence chooses gaps between 1 and
  `2 * mean_interval - 1`, avoiding a fixed phase in tight guest loops. The initial
  phase also varies by seed. Interval 1 samples every instruction; the interval
  is clamped to [1, 2^30]. Seed 0 uses the same sequence as seed 1. Reset preserves
  the selected clock, mode, interval and seed and restarts the sequence.

The injected `fn() -> u64` must report host monotonic nanoseconds. It must not
advance guest time, reenter the CPU, mutate guest state or panic. Profiling only
observes calls to `Cpu::step`; external register access and direct external
`Cpu::exception` calls are excluded. The clock is never used to choose execution
budgets, timer scheduling, instruction results or guest timestamps.

Counter interpretation:

* `instruction_attempts` counts `Cpu::step` calls, split into ARM and Thumb.
  A long Thumb branch consumes two halfwords in this existing interpreter but
  is one step and one instruction attempt, with two fetch events.
* `decoded_instructions` means the decoder recognized a pattern; an instruction
  may still fail during execution because of invalid operands.
* `retired_instructions` means `Cpu::step` returned true. This includes an ARM
  instruction skipped because its condition was false.
* `executed_instructions` means a successful step entered the dispatch match;
  conditionally skipped and failing instructions are excluded.
* `undefined_instructions` means `Cpu::step` returned false, including a
  recognized instruction rejected by its implementation. An unrecognized ARM
  instruction with a false condition preserves upstream skip behavior.
* `exception_entries` counts exception-entry attempts during a step, including
  software interrupts. Host-side SVC handling is outside the CPU and must be
  measured separately.

`Stage::ALL` and `Stage::name()` give the stable order/names of the snapshot's
`stages` array. Every entry counts all `events`, `sampled_events`, and sampled
spans whose inclusive duration is nonzero (`nonzero_events`). Timings are totals
over sampled events only, in nanoseconds:

| Stage | Scope |
| --- | --- |
| instruction | Entire step, including instrumented children and unclassified prelude |
| fetch | Aligned instruction memory read; includes a long Thumb branch's second halfword |
| decode | Functional instruction pattern lookup; excludes the optional logging-only duplicate decode |
| condition | ARM instruction condition or Thumb conditional branch predicate |
| execute | Dispatch match and instruction body, including nested stages |
| data_read / data_write | Guest data memory calls, including address/argument evaluation and alignment adapter |
| flags_cpsr | Explicit CPSR/SPSR reads/writes and packing new flags; arithmetic producing carry/overflow remains in execute |
| branch_pc | PC writes, including ordinary sequential advance, branch address calculation and loaded-PC assignment |
| exception_check | Checking whether the step requires an undefined-instruction exception |
| exception | Exception entry routine during a step, nested within execute or exception_check |

Each stage has `inclusive_ns` and `exclusive_ns`. Exclusive time subtracts nested
instrumented child spans. **Never add inclusive times across stages**: execute
contains data accesses and instruction contains everything. Summed exclusive
stage times equal total instruction inclusive time for complete samples with a
monotonic clock and no stack overflow. The bounded eight-level stack records
`stack_overflows` rather than allocating. Dynamic destination register writes
are classified as branch/status work only when their destination is PC/CPSR/SPSR.
General register lookup and arithmetic otherwise stay in execute.

The sampled timestamp/stack paths are outlined into non-inlined cold functions
to reduce duplication inside the instruction dispatch code. Event counters and
mode/sample checks remain inline. This preserves the sampling sequence and clock
boundaries; it does not establish low overhead without measurement.

Timing includes observer effects. In Android WebView, `performance.now()` can be
quantized to about 100 microseconds, much coarser than individual instructions.
A zero duration is not evidence of zero cost; a rare nonzero tick is not evidence
that one stage dominates. Record the observed clock resolution, clock-pair cost,
nonzero counts, total sample count and instrumented/uninstrumented throughput.
Use repeated physical-device runs and interval sweeps (for example 64, 256 and
1024). Report categories as unresolved when samples are too coarse or noisy. Do
not subtract a guessed clock cost or change device/browser clock flags to make a
result look precise. Regressing clock spans saturate at zero and increment
`clock_regressions`.

Validation commands (from the WIE workspace):

```
cargo test --manifest-path vendor/arm32_cpu/Cargo.toml --no-default-features
cargo test --manifest-path vendor/arm32_cpu/Cargo.toml --features profiling
```

Tests compare every upstream ARM/Thumb executable fixture in Off, Counts,
Sampled(1) and Sampled(256), including all architectural register banks, exact
memory bytes/access order and every step result. Additional tests cover
condition skips, undefined instructions, ARM/Thumb transitions, SVC exceptions,
stack bounds, deterministic sampling, nested timing accounting and coarse or
regressing clocks. Timing instrumentation is diagnostic evidence, not itself a
performance optimization.
