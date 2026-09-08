# arm32_cpu
[![crates.io](https://img.shields.io/crates/v/arm32_cpu.svg)](https://crates.io/crates/arm32_cpu)
[![docs.rs](https://docs.rs/arm32_cpu/badge.svg)](https://docs.rs/arm32_cpu)

An emulator for ARM32 CPUs with ARMv5-era ARM/Thumb support, written in Rust.

This project is a fork of [`armv4t_emu`](https://github.com/daniel5151/armv4t_emu),
extended to support ARMv5 instructions in addition to ARMv4T.

## Example

```rust
use arm32_cpu::{reg, Cpu, ExampleMem, Mode, Memory};

let prog = &[
    0x06, 0x00, 0xa0, 0xe3, //    mov r0, #6
    0x01, 0x10, 0xa0, 0xe3, //    mov r1, #1
    0x01, 0x10, 0x81, 0xe0, // l: add r1, r1, r1
    0x01, 0x00, 0x50, 0xe2, //    subs r0, #1
    0xfc, 0xff, 0xff, 0x1a, //    bne l
    0x01, 0x6c, 0xa0, 0xe3, //    mov r6, #0x100
    0x00, 0x10, 0x86, 0xe5, //    str r1, [r6]
    0xf7, 0xf0, 0xde, 0xad  // ; trigger undefined instr exception
];
let mut mem = ExampleMem::new_with_data(prog);
let mut cpu = Cpu::new();
cpu.reg_set(Mode::User, reg::PC, 0x00);
cpu.reg_set(Mode::User, reg::CPSR, 0x10);

while cpu.step(&mut mem) {}

assert_eq!(64, mem.r32(0x100));
```

## Optional Features

There are a couple of optional features that you may want to enable, providing various bits of functionality / debugging enhancements. These are disabled by default, as though they do bloat compile times.

Feature | Description
--------|-------------
`advanced_disasm` | Uses [`capstone`](https://github.com/capstone-rust/capstone-rs) to disassemble + log instructions. _Warning:_ Substantially increases compile times.*
`serde` | Adds `Serialize` and `Deserialize` derives on important types/structs.

\* Instead of debugging emulated code by creating a ad-hoc, application-specific debugger, consider using the [`gdbstub`](https://github.com/daniel5151/gdbstub) crate.

## Missing Features

At the moment, this crate's feature set is primarily motivated by whatever functionality its dependent projects require. It targets an ARMv5-era subset rather than full architecture coverage, and several pieces are still intentionally missing:

- Custom co-processor support (see [#3](https://github.com/daniel5151/armv4t_emu/issues/3))
- Big-endian support (see [#4](https://github.com/daniel5151/armv4t_emu/issues/4))
- Support for cycle-accurate emulation
    - This would be tricky to implement, as `arm32_cpu` is not an emulator for any particular ARM core.
    - Theoretically, this could be implemented by modifying the public API to accept some kind of platform-specific timing information.

## Local diagnostic feature

This vendored copy adds optional interpreter observations. See [PROFILING.md](PROFILING.md) for the API, stage definitions, sampling limitations and validation. Profiling is disabled by default; the original package file hashes are recorded in `UPSTREAM.sha256.json`.
