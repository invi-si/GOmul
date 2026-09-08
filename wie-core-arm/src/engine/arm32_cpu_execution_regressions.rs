//! Observable instruction-fetch behavior shared by the baseline and decoder experiments.

use arm32_cpu::{Mode, reg};

use super::{Arm32CpuEngine, PAGE_SIZE};
use crate::engine::{ArmEngine, ArmRegister, EngineRunResult, MemoryPermission};

fn thumb_engine() -> Arm32CpuEngine {
    let mut engine = Arm32CpuEngine::new();
    engine.mem_map(0x10000, PAGE_SIZE, MemoryPermission::ReadWriteExecute);
    engine.reg_write(ArmRegister::Cpsr, 0x30);
    engine.reg_write(ArmRegister::PC, 0x10001);
    engine
}

fn one_step(engine: &mut Arm32CpuEngine, pc: u32) {
    engine.reg_write(ArmRegister::PC, pc | 1);
    assert!(matches!(engine.run(0x30000, 1).unwrap(), EngineRunResult::CountExhausted));
}

#[test]
fn thumb_word_load_preserves_unaligned_rotation_flags_and_pc() {
    let mut engine = thumb_engine();
    engine.mem_write(0x10000, &0x6811u16.to_le_bytes()).unwrap(); // LDR r1,[r2]
    engine.mem_write(0x1fffc, &0x12345678u32.to_le_bytes()).unwrap();
    for offset in 0..4 {
        engine.reg_write(ArmRegister::Cpsr, 0xa0000030);
        engine.reg_write(ArmRegister::R2, 0x1fffc + offset);
        one_step(&mut engine, 0x10000);
        assert_eq!(engine.reg_read(ArmRegister::R1), 0x12345678u32.rotate_right(offset * 8));
        assert_eq!(engine.reg_read(ArmRegister::R2), 0x1fffc + offset);
        assert_eq!(engine.reg_read(ArmRegister::Cpsr), 0xa0000030);
        assert_eq!(engine.reg_read(ArmRegister::PC), 0x10002);
    }
}

#[test]
fn host_and_guest_code_writes_are_visible_on_the_next_fetch() {
    let mut engine = thumb_engine();
    engine.mem_write(0x10000, &0x2001u16.to_le_bytes()).unwrap(); // MOV r0,1
    one_step(&mut engine, 0x10000);
    assert_eq!(engine.reg_read(ArmRegister::R0), 1);

    engine.mem_write(0x10000, &0x2102u16.to_le_bytes()).unwrap(); // MOV r1,2
    one_step(&mut engine, 0x10000);
    assert_eq!(engine.reg_read(ArmRegister::R0), 1);
    assert_eq!(engine.reg_read(ArmRegister::R1), 2);

    engine.mem_write(0x10020, &0x801au16.to_le_bytes()).unwrap(); // STRH r2,[r3]
    engine.reg_write(ArmRegister::R2, 0x2009); // MOV r0,9
    engine.reg_write(ArmRegister::R3, 0x10000);
    one_step(&mut engine, 0x10020);
    let mut bytes = [0; 2];
    engine.mem_read(0x10000, bytes.len(), &mut bytes).unwrap();
    assert_eq!(bytes, 0x2009u16.to_le_bytes());
    one_step(&mut engine, 0x10000);
    assert_eq!(engine.reg_read(ArmRegister::R0), 9);
    assert_eq!(engine.reg_read(ArmRegister::R1), 2);
}

#[test]
fn nop_requires_the_exact_low_byte_after_code_is_overwritten() {
    let mut engine = thumb_engine();
    engine.mem_write(0x10000, &0xbf00u16.to_le_bytes()).unwrap();
    engine.reg_write(ArmRegister::R0, 123);
    one_step(&mut engine, 0x10000);
    assert_eq!(engine.reg_read(ArmRegister::PC), 0x10002);
    engine.mem_write(0x10000, &0xbf01u16.to_le_bytes()).unwrap();
    engine.reg_write(ArmRegister::PC, 0x10001);
    assert!(matches!(engine.run(0x30000, 1), Err(wie_util::WieError::FatalError(message)) if message == "Undefined instruction"));
    assert_eq!(engine.reg_read(ArmRegister::PC), 4);
    assert_eq!(engine.cpu.reg_get(Mode::Undefined, reg::LR), 0x10002);
    assert_eq!(engine.cpu.reg_get(Mode::Undefined, reg::SPSR), 0x30);
    assert_eq!(engine.reg_read(ArmRegister::Cpsr), 0x9b);
    assert_eq!(engine.reg_read(ArmRegister::R0), 123);
}

#[test]
fn long_branch_fetches_the_current_second_halfword_across_pages() {
    let mut engine = thumb_engine();
    engine.mem_map(0x20000, PAGE_SIZE, MemoryPermission::ReadWriteExecute);
    engine.mem_write(0x1fffe, &0xf000u16.to_le_bytes()).unwrap();
    engine.mem_write(0x20000, &0xf800u16.to_le_bytes()).unwrap();
    one_step(&mut engine, 0x1fffe);
    assert_eq!(engine.reg_read(ArmRegister::PC), 0x20002);
    assert_eq!(engine.reg_read(ArmRegister::LR), 0x20003);

    engine.mem_write(0x20000, &0xf802u16.to_le_bytes()).unwrap();
    one_step(&mut engine, 0x1fffe);
    assert_eq!(engine.reg_read(ArmRegister::PC), 0x20006);
    assert_eq!(engine.reg_read(ArmRegister::LR), 0x20003);

    engine.mem_write(0x20000, &0x0000u16.to_le_bytes()).unwrap();
    engine.reg_write(ArmRegister::PC, 0x1ffff);
    assert!(matches!(engine.run(0x30000, 1), Err(wie_util::WieError::FatalError(message)) if message == "Undefined instruction"));
    assert_eq!(engine.reg_read(ArmRegister::PC), 4);
    assert_eq!(engine.cpu.reg_get(Mode::Undefined, reg::LR), 0x20000);
    assert_eq!(engine.reg_read(ArmRegister::LR), 0x20003);
}

#[test]
fn missing_long_branch_suffix_preserves_existing_error_precedence() {
    let mut engine = thumb_engine();
    engine.mem_write(0x1fffe, &0xf000u16.to_le_bytes()).unwrap();
    engine.reg_write(ArmRegister::PC, 0x1ffff);
    assert!(!engine.is_mapped(0x20000, 2));
    // The memory adapter returns zero on the missing suffix. The existing
    // interpreter reports the resulting undefined encoding before its fault latch.
    assert!(matches!(engine.run(0x30000, 1), Err(wie_util::WieError::FatalError(message)) if message == "Undefined instruction"));
    assert_eq!(engine.reg_read(ArmRegister::PC), 4);
    assert_eq!(engine.cpu.reg_get(Mode::Undefined, reg::LR), 0x20000);
}

#[test]
fn thumb_fetch_uses_the_raw_halfword_address_and_pc_advance() {
    let mut engine = thumb_engine();
    engine.mem_write(0x10000, &[0x00, 0x07, 0x20]).unwrap(); // MOV r0,7 at odd address 0x10001
    // Bypass the host entry-point normalization to exercise the CPU's existing
    // raw halfword fetch with an odd architectural PC. This preserves existing
    // behavior; the interpreter does not round halfword reads down.
    engine.cpu.reg_set(Mode::User, reg::PC, 0x10001);
    assert!(matches!(engine.run(0x30000, 1).unwrap(), EngineRunResult::CountExhausted));
    assert_eq!(engine.reg_read(ArmRegister::R0), 7);
    assert_eq!(engine.reg_read(ArmRegister::PC), 0x10003);
}
