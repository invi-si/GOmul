//! Untimed, thread-local observations. No architectural state is stored here.
//! Enabled only around replay CPU runs, never around restore/validation.
use std::{cell::RefCell, collections::BTreeMap};

#[derive(Default, Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Snapshot {
    pub pcs: Vec<((bool, u32), u64)>,
    pub opcodes: Vec<((bool, u32), u64)>,
    pub classes: Vec<((bool, String), u64)>,
    pub register_reads: [u64; 18],
    /// IndexMut calls include read-modify-write operations; they are not literal
    /// host load/store counts. Explicit get/set and mode reads are also counted.
    pub register_writes: [u64; 18],
    pub mode_reads: u64,
    pub invalid_cpsr_old_reads: u64,
    /// fetch16, fetch32, data-read8/16/32, data-write8/16/32 (attempts).
    pub memory: [u64; 8],
    pub conditional_taken: u64,
    pub conditional_not_taken: u64,
    pub explicit_unconditional: u64,
}
#[derive(Default)]
struct Counts {
    active: bool,
    fetch: bool,
    snapshot: Snapshot,
    pcs: BTreeMap<(bool, u32), u64>,
    opcodes: BTreeMap<(bool, u32), u64>,
    classes: BTreeMap<(bool, u32), (String, u64)>,
}
thread_local! { static COUNTS: RefCell<Counts> = RefCell::new(Counts::default()); }
pub fn reset() {
    COUNTS.with(|c| *c.borrow_mut() = Counts::default());
}
pub fn enable(active: bool) {
    COUNTS.with(|c| {
        let mut c = c.borrow_mut();
        c.active = active;
        c.fetch = false;
    });
}
fn update(f: impl FnOnce(&mut Counts)) {
    COUNTS.with(|c| {
        let mut c = c.borrow_mut();
        if c.active {
            f(&mut c);
        }
    });
}
pub fn fetch(active: bool) {
    update(|c| c.fetch = active);
}
pub fn instruction(pc: u32, opcode: u32, thumb: bool, class: u32, name: impl std::fmt::Debug) {
    update(|c| {
        *c.pcs.entry((thumb, pc)).or_default() += 1;
        *c.opcodes.entry((thumb, opcode)).or_default() += 1;
        c.classes
            .entry((thumb, class))
            .or_insert_with(|| (format!("{name:?}"), 0))
            .1 += 1;
    });
}
pub fn register(reg: u8, write: bool) {
    update(|c| {
        if let Some(n) = if write {
            c.snapshot.register_writes.get_mut(reg as usize)
        } else {
            c.snapshot.register_reads.get_mut(reg as usize)
        } {
            *n += 1;
        }
    });
}
pub fn mode_read() {
    update(|c| c.snapshot.mode_reads += 1);
}
pub fn invalid_cpsr_read() {
    update(|c| c.snapshot.invalid_cpsr_old_reads += 1);
}
pub fn memory(width: usize, write: bool) {
    update(|c| {
        let index = if c.fetch && !write {
            if width == 2 {
                0
            } else {
                1
            }
        } else {
            2 + if write { 3 } else { 0 }
                + match width {
                    1 => 0,
                    2 => 1,
                    _ => 2,
                }
        };
        c.snapshot.memory[index] += 1;
    });
}
pub fn branch(conditional: bool, taken: bool) {
    update(|c| {
        if conditional {
            if taken {
                c.snapshot.conditional_taken += 1;
            } else {
                c.snapshot.conditional_not_taken += 1;
            }
        } else if taken {
            c.snapshot.explicit_unconditional += 1;
        }
    });
}
pub fn snapshot() -> Snapshot {
    COUNTS.with(|c| {
        let c = c.borrow();
        let mut s = c.snapshot.clone();
        s.pcs = c.pcs.iter().map(|(k, v)| (*k, *v)).collect();
        s.opcodes = c.opcodes.iter().map(|(k, v)| (*k, *v)).collect();
        s.classes = c
            .classes
            .iter()
            .map(|((thumb, _), (name, count))| ((*thumb, name.clone()), *count))
            .collect();
        s
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{reg, Cpu, ExampleMem, Mode};
    #[test]
    fn exact_loop_counts_and_disabled_host_reads_preserve_cpu() {
        let program = [0x02, 0x20, 0x01, 0x38, 0xfd, 0xd1]; // MOV 2; SUB 1; BNE SUB
        let mut observed = Cpu::new();
        observed.reg_set(Mode::User, reg::CPSR, 0x30);
        let mut oracle = observed;
        let mut a = ExampleMem::new_with_data(&program);
        let mut b = ExampleMem::new_with_data(&program);
        reset();
        for _ in 0..5 {
            enable(true);
            assert!(observed.step(&mut a));
            enable(false);
            assert!(oracle.step(&mut b));
        }
        assert_eq!(observed, oracle);
        let before = snapshot();
        assert_eq!(
            before.pcs,
            vec![((true, 0), 1), ((true, 2), 2), ((true, 4), 2)]
        );
        assert_eq!(before.conditional_taken, 1);
        assert_eq!(before.conditional_not_taken, 1);
        assert_eq!(before.register_writes[0], 3);
        assert_eq!(observed.reg_get(Mode::User, 0), 0);
        assert_eq!(snapshot(), before);
    }
}
