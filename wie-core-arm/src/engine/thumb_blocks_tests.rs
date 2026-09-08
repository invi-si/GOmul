use super::super::Arm32CpuEngine;
use super::*;
use crate::engine::{ArmEngine, ArmRegister, EngineRunResult, MemoryPermission};

fn engine(code: &[u16], enabled: bool, flags: u32) -> Arm32CpuEngine {
    let mut e = Arm32CpuEngine::new();
    e.blocks.enabled = enabled;
    e.mem_map(0x10000, PAGE_SIZE * 2, MemoryPermission::ReadWriteExecute);
    e.mem_write(0x10000, &code.iter().flat_map(|i| i.to_le_bytes()).collect::<Vec<_>>())
        .unwrap();
    for i in 0..15 {
        e.cpu.reg_set(Mode::User, i, 0x20000 + u32::from(i) * 16);
    }
    e.reg_write(ArmRegister::Cpsr, flags | 0x30);
    e.reg_write(ArmRegister::PC, 0x10001);
    e
}
fn outcome(result: wie_util::Result<EngineRunResult>) -> alloc::string::String {
    match result {
        Ok(EngineRunResult::End) => "end".into(),
        Ok(EngineRunResult::CountExhausted) => "count".into(),
        Ok(EngineRunResult::Svc { category, lr, spsr }) => alloc::format!("svc {category} {lr} {spsr}"),
        Err(e) => alloc::format!("{e:?}"),
    }
}
fn compare(a: &mut Arm32CpuEngine, b: &mut Arm32CpuEngine, end: u32, count: u32) {
    assert_eq!(outcome(a.run(end, count)), outcome(b.run(end, count)));
    assert_eq!(a.cpu, b.cpu, "all architectural banks and CPSR/PC");
    assert_eq!(a.mem.pages, b.mem.pages, "all mapped memory and writes");
}
#[test]
fn arithmetic_flags_and_partial_fused_boundaries() {
    for flags in 0..16 {
        for value in [0, 1, 0x7fffffff, 0x80000000, u32::MAX] {
            for count in 0..12 {
                let code = [0x3001, 0x3801, 0x0041, 0x4281, 0xd000, 0x2207, 0xdf12];
                let mut a = engine(&code, true, flags << 28);
                let mut b = engine(&code, false, flags << 28);
                a.cpu.reg_set(Mode::User, 0, value);
                b.cpu.reg_set(Mode::User, 0, value);
                compare(&mut a, &mut b, 0x30000, count);
            }
        }
    }
}
#[test]
fn every_end_address_and_budget_in_pixel_loop() {
    // Indexed colour lookup, transparency test, conditional halfword write.
    let code = [
        0x9b1d, 0x5ceb, 0x3501, 0x469c, 0x9b31, 0x6a5a, 0x4663, 0x005b, 0x5a9a, 0x6833, 0x429a, 0xd000, 0x8002, 0x3901, 0x3002, 0x2900, 0xd1ee,
    ];
    for end in (0x10000..=0x10024).step_by(2) {
        for count in 0..40 {
            let mut a = engine(&code, true, 0);
            let mut b = engine(&code, false, 0);
            for e in [&mut a, &mut b] {
                e.cpu.reg_set(Mode::User, 0, 0x20500);
                e.cpu.reg_set(Mode::User, 1, 2);
                e.mem_write(0x200d0 + 0xc4, &0x20200u32.to_le_bytes()).unwrap();
                e.mem_write(0x20224, &0x20300u32.to_le_bytes()).unwrap();
                e.mem_write(0x20300, &7u16.to_le_bytes()).unwrap();
            }
            compare(&mut a, &mut b, end, count);
        }
    }
}
#[test]
fn fused_first_and_second_load_faults_and_alignment() {
    for sp in [0x20000, 0x20001, 0x2fffc, 0x30000] {
        for pointer in [0x20000u32, 0x20001, 0x2ffff, 0x30000] {
            let mut a = engine(&[0x9b00, 0x681a, 0x3001], true, 0xf0000000);
            let mut b = engine(&[0x9b00, 0x681a, 0x3001], false, 0xf0000000);
            for e in [&mut a, &mut b] {
                if sp < 0x30000 {
                    e.mem_write(sp & !3, &pointer.to_le_bytes()).unwrap();
                }
                e.cpu.reg_set(Mode::User, reg::SP, sp);
            }
            compare(&mut a, &mut b, 0x30000, 3);
        }
    }
}
#[test]
fn host_code_changes_and_partial_host_write_invalidate_guard() {
    let mut a = engine(&[0x2001, 0x2102, 0xe7fc], true, 0);
    let mut b = engine(&[0x2001, 0x2102, 0xe7fc], false, 0);
    compare(&mut a, &mut b, 0x30000, 6);
    for e in [&mut a, &mut b] {
        e.mem_write(0x10002, &0x2109u16.to_le_bytes()).unwrap();
    }
    compare(&mut a, &mut b, 0x30000, 6);
    for e in [&mut a, &mut b] {
        e.mem_write(0x2fffc, &[1, 2, 3, 4, 5]).unwrap_err();
        e.mem_write(0x10000, &0xbf01u16.to_le_bytes()).unwrap();
    }
    compare(&mut a, &mut b, 0x30000, 6);
}
#[test]
fn guest_write_to_next_instruction_and_store_fault() {
    for address in [0x10002, 0x30000] {
        let mut a = engine(&[0x8002, 0x2101, 0xe7fd], true, 0);
        let mut b = engine(&[0x8002, 0x2101, 0xe7fd], false, 0);
        for e in [&mut a, &mut b] {
            e.cpu.reg_set(Mode::User, 0, address);
            e.cpu.reg_set(Mode::User, 2, 0x2109);
        }
        compare(&mut a, &mut b, 0x30000, 9);
    }
}
#[test]
fn scheduler_sized_exit_and_host_register_changes() {
    let mut a = engine(&[0x3001, 0xe7fd], true, 0);
    let mut b = engine(&[0x3001, 0xe7fd], false, 0);
    compare(&mut a, &mut b, 0x30000, 10_000);
    for e in [&mut a, &mut b] {
        e.cpu.reg_set(Mode::User, 0, 0xffffffff);
    }
    compare(&mut a, &mut b, 0x30000, 10_000);
}
#[test]
fn unsupported_modes_fetch_faults_and_svc_are_oracle_exits() {
    for cpsr in [0x10, 0x30, 0x31, 0x32, 0x33, 0x37, 0x3b, 0x3f] {
        for pc in [0x10000, 0x10001, 0x1fffe, 0x30000] {
            let mut a = engine(&[0x2001, 0x2102, 0xdf03], true, 0);
            let mut b = engine(&[0x2001, 0x2102, 0xdf03], false, 0);
            for e in [&mut a, &mut b] {
                e.cpu.reg_set(Mode::User, reg::CPSR, cpsr);
                e.cpu.reg_set(Mode::User, reg::PC, pc);
            }
            compare(&mut a, &mut b, 0x40000, 4);
        }
    }
}

#[test]
fn previously_unsupported_code_becomes_supported() {
    let mut a = engine(&[0xbf00, 0x2001, 0xe7fc], true, 0);
    let mut b = engine(&[0xbf00, 0x2001, 0xe7fc], false, 0);
    compare(&mut a, &mut b, 0x30000, 3);
    for e in [&mut a, &mut b] {
        e.mem_write(0x10000, &0x2109u16.to_le_bytes()).unwrap();
    }
    compare(&mut a, &mut b, 0x30000, 3);
}

#[test]
fn interrupt_entry_after_each_partial_block_matches_oracle() {
    for count in 0..8 {
        for exception in [arm32_cpu::Exception::Interrupt, arm32_cpu::Exception::FastInterrupt] {
            let mut a = engine(&[0x3001, 0x4281, 0xd000, 0x2207, 0xe7fa], true, 0);
            let mut b = engine(&[0x3001, 0x4281, 0xd000, 0x2207, 0xe7fa], false, 0);
            compare(&mut a, &mut b, 0x30000, count);
            a.cpu.exception(exception);
            b.cpu.exception(exception);
            compare(&mut a, &mut b, 0x30000, 1);
        }
    }
}

#[cfg(all(feature = "cpu-throughput", not(feature = "cpu-profiling")))]
#[test]
fn block_steps_are_accounted_at_exact_scheduler_budget() {
    let mut e = engine(&[0x3001, 0xe7fd], true, 0);
    e.run(0x30000, 10_000).unwrap();
    let s = e.throughput_snapshot();
    assert_eq!(s.total_instructions, 10_000);
    assert_eq!(s.block_instructions, 10_000);
    e.reset_throughput();
    assert_eq!(e.throughput_snapshot().block_instructions, 0);
}

#[test]
fn every_guest_store_width_invalidates_warmed_code() {
    for store in [0x701au16, 0x801a, 0x601a] {
        let mut a = engine(&[0x2001, 0x2102, 0xe7fc], true, 0);
        let mut b = engine(&[0x2001, 0x2102, 0xe7fc], false, 0);
        compare(&mut a, &mut b, 0x30000, 6);
        for e in [&mut a, &mut b] {
            e.mem_write(0x10100, &store.to_le_bytes()).unwrap();
            e.cpu.reg_set(Mode::User, 2, 0x21072009);
            e.cpu.reg_set(Mode::User, 3, 0x10000);
            e.cpu.reg_set(Mode::User, reg::PC, 0x10100);
        }
        compare(&mut a, &mut b, 0x30000, 1);
        for e in [&mut a, &mut b] {
            e.cpu.reg_set(Mode::User, reg::PC, 0x10000);
        }
        compare(&mut a, &mut b, 0x30000, 6);
    }
}
