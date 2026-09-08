//! Optional interpreter observations. The supplied clock measures host monotonic
//! nanoseconds; it must never advance the guest clock or execute guest work.
//!
//! Every stage has an inclusive time and a time excluding instrumented children.
//! `Instruction` contains all other stages. `Execute` includes memory, register,
//! and Thumb condition work; sum exclusive times, never inclusive times. Timing
//! includes observer overhead and is subject to the host clock's resolution.

pub type HostClock = fn() -> u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileMode {
    Off,
    Counts,
    Sampled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
#[repr(usize)]
pub enum Stage {
    Instruction,
    Fetch,
    Decode,
    Condition,
    Execute,
    DataRead,
    DataWrite,
    FlagsCpsr,
    BranchPc,
    ExceptionCheck,
    Exception,
}

impl Stage {
    pub const COUNT: usize = 11;
    pub const ALL: [Self; Self::COUNT] = [
        Self::Instruction,
        Self::Fetch,
        Self::Decode,
        Self::Condition,
        Self::Execute,
        Self::DataRead,
        Self::DataWrite,
        Self::FlagsCpsr,
        Self::BranchPc,
        Self::ExceptionCheck,
        Self::Exception,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Instruction => "instruction",
            Self::Fetch => "fetch",
            Self::Decode => "decode",
            Self::Condition => "condition",
            Self::Execute => "execute",
            Self::DataRead => "data_read",
            Self::DataWrite => "data_write",
            Self::FlagsCpsr => "flags_cpsr",
            Self::BranchPc => "branch_pc",
            Self::ExceptionCheck => "exception_check",
            Self::Exception => "exception",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct StageStats {
    /// Calls in all observed instructions, including unsampled instructions.
    pub events: u64,
    pub sampled_events: u64,
    /// Sampled spans whose inclusive duration exceeded zero clock ticks.
    pub nonzero_events: u64,
    pub inclusive_ns: u64,
    pub exclusive_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct ProfileSnapshot {
    pub mode: ProfileMode,
    /// Mean gap between sampled instructions. Actual gaps jitter to avoid
    /// repeatedly observing the same position in a tight guest loop.
    pub mean_interval: u32,
    pub seed: u32,
    pub instruction_attempts: u64,
    pub arm_instructions: u64,
    pub thumb_instructions: u64,
    /// Decoder recognized the instruction, regardless of its ARM predicate.
    pub decoded_instructions: u64,
    /// Cpu::step returned true; includes conditionally skipped instructions.
    pub retired_instructions: u64,
    /// Successfully retired after entering the instruction dispatch match.
    pub executed_instructions: u64,
    pub condition_failed_instructions: u64,
    pub undefined_instructions: u64,
    /// Exception entry attempts during Cpu::step (including SVC).
    pub exception_entries: u64,
    pub sampled_instructions: u64,
    pub clock_reads: u64,
    pub clock_regressions: u64,
    pub stack_overflows: u64,
    pub stages: [StageStats; Stage::COUNT],
}

impl ProfileSnapshot {
    fn empty(mode: ProfileMode, mean_interval: u32, seed: u32) -> Self {
        Self {
            mode,
            mean_interval,
            seed,
            instruction_attempts: 0,
            arm_instructions: 0,
            thumb_instructions: 0,
            decoded_instructions: 0,
            retired_instructions: 0,
            executed_instructions: 0,
            condition_failed_instructions: 0,
            undefined_instructions: 0,
            exception_entries: 0,
            sampled_instructions: 0,
            clock_reads: 0,
            clock_regressions: 0,
            stack_overflows: 0,
            stages: [StageStats::default(); Stage::COUNT],
        }
    }
}

#[derive(Clone, Copy)]
struct ActiveStage {
    stage: Stage,
    started: u64,
    children: u64,
}

const MAX_DEPTH: usize = 8;

#[derive(Clone, Copy)]
pub(crate) struct Profiler {
    snapshot: ProfileSnapshot,
    clock: HostClock,
    random: u32,
    until_sample: u32,
    in_instruction: bool,
    sampled: bool,
    instruction_token: bool,
    execute_token: bool,
    dispatched: bool,
    depth: usize,
    stack: [ActiveStage; MAX_DEPTH],
}

impl Default for Profiler {
    fn default() -> Self {
        Self {
            snapshot: ProfileSnapshot::empty(ProfileMode::Off, 1024, 1),
            clock: || 0,
            random: 1,
            until_sample: 0,
            in_instruction: false,
            sampled: false,
            instruction_token: false,
            execute_token: false,
            dispatched: false,
            depth: 0,
            stack: [ActiveStage {
                stage: Stage::Instruction,
                started: 0,
                children: 0,
            }; MAX_DEPTH],
        }
    }
}

impl Profiler {
    pub(crate) fn configure(
        &mut self,
        mode: ProfileMode,
        mean_interval: u32,
        seed: u32,
        clock: HostClock,
    ) {
        *self = Self::default();
        let interval = mean_interval.clamp(1, 1 << 30);
        self.snapshot = ProfileSnapshot::empty(mode, interval, seed);
        self.clock = clock;
        self.random = seed.max(1);
        self.until_sample = self.next_random() % interval;
    }

    pub(crate) fn reset(&mut self) {
        self.configure(
            self.snapshot.mode,
            self.snapshot.mean_interval,
            self.snapshot.seed,
            self.clock,
        );
    }

    pub(crate) fn snapshot(&self) -> ProfileSnapshot {
        self.snapshot
    }

    fn next_random(&mut self) -> u32 {
        let mut x = self.random;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.random = x;
        x
    }

    #[inline]
    pub(crate) fn begin_instruction(&mut self) {
        if self.snapshot.mode == ProfileMode::Off {
            return;
        }
        self.in_instruction = true;
        self.dispatched = false;
        self.snapshot.instruction_attempts = self.snapshot.instruction_attempts.wrapping_add(1);
        self.sampled = false;
        if self.snapshot.mode == ProfileMode::Sampled {
            if self.until_sample == 0 {
                self.sampled = true;
                self.snapshot.sampled_instructions =
                    self.snapshot.sampled_instructions.wrapping_add(1);
                self.until_sample = self.next_random() % (self.snapshot.mean_interval * 2 - 1);
            } else {
                self.until_sample -= 1;
            }
        }
        self.instruction_token = self.enter(Stage::Instruction);
    }

    #[inline]
    pub(crate) fn instruction_set(&mut self, thumb: bool) {
        if !self.in_instruction {
            return;
        }
        let count = if thumb {
            &mut self.snapshot.thumb_instructions
        } else {
            &mut self.snapshot.arm_instructions
        };
        *count = count.wrapping_add(1);
    }

    #[inline]
    pub(crate) fn decoded(&mut self, recognized: bool) {
        if self.in_instruction && recognized {
            self.snapshot.decoded_instructions = self.snapshot.decoded_instructions.wrapping_add(1);
        }
    }

    #[inline]
    pub(crate) fn condition_failed(&mut self) {
        if self.in_instruction {
            self.snapshot.condition_failed_instructions =
                self.snapshot.condition_failed_instructions.wrapping_add(1);
        }
    }

    #[inline]
    pub(crate) fn begin_execute(&mut self) {
        if !self.in_instruction {
            return;
        }
        self.dispatched = true;
        self.execute_token = self.enter(Stage::Execute);
    }

    #[inline]
    pub(crate) fn end_execute(&mut self) {
        self.leave(self.execute_token);
        self.execute_token = false;
    }

    #[inline]
    pub(crate) fn end_instruction(&mut self, retired: bool) {
        if !self.in_instruction {
            return;
        }
        if retired {
            self.snapshot.retired_instructions = self.snapshot.retired_instructions.wrapping_add(1);
            if self.dispatched {
                self.snapshot.executed_instructions =
                    self.snapshot.executed_instructions.wrapping_add(1);
            }
        } else {
            self.snapshot.undefined_instructions =
                self.snapshot.undefined_instructions.wrapping_add(1);
        }
        self.leave(self.instruction_token);
        self.instruction_token = false;
        self.in_instruction = false;
        self.sampled = false;
    }

    #[inline]
    pub(crate) fn exception(&mut self) {
        if self.in_instruction {
            self.snapshot.exception_entries = self.snapshot.exception_entries.wrapping_add(1);
        }
    }

    #[inline]
    pub(crate) fn register_write(&mut self, register: u8) -> bool {
        if !self.in_instruction {
            return false;
        }
        match register {
            crate::reg::PC => self.enter(Stage::BranchPc),
            crate::reg::CPSR | crate::reg::SPSR => self.enter(Stage::FlagsCpsr),
            _ => false,
        }
    }

    #[inline]
    pub(crate) fn enter(&mut self, stage: Stage) -> bool {
        if !self.in_instruction {
            return false;
        }
        let stats = &mut self.snapshot.stages[stage as usize];
        stats.events = stats.events.wrapping_add(1);
        if !self.sampled {
            return false;
        }
        self.enter_timed(stage)
    }

    // Keep rare timestamp/stack work out of each inlined instruction arm. The
    // clock boundaries and sampling decisions remain identical to enter/leave.
    #[cold]
    #[inline(never)]
    fn enter_timed(&mut self, stage: Stage) -> bool {
        if self.depth == MAX_DEPTH {
            self.snapshot.stack_overflows = self.snapshot.stack_overflows.wrapping_add(1);
            return false;
        }
        let stats = &mut self.snapshot.stages[stage as usize];
        stats.sampled_events = stats.sampled_events.wrapping_add(1);
        let started = self.read_clock();
        self.stack[self.depth] = ActiveStage {
            stage,
            started,
            children: 0,
        };
        self.depth += 1;
        true
    }

    #[inline]
    pub(crate) fn leave(&mut self, token: bool) {
        if !token {
            return;
        }
        self.leave_timed();
    }

    #[cold]
    #[inline(never)]
    fn leave_timed(&mut self) {
        let ended = self.read_clock();
        self.depth -= 1;
        let active = self.stack[self.depth];
        if ended < active.started {
            self.snapshot.clock_regressions = self.snapshot.clock_regressions.wrapping_add(1);
        }
        let elapsed = ended.saturating_sub(active.started);
        let stats = &mut self.snapshot.stages[active.stage as usize];
        if elapsed > 0 {
            stats.nonzero_events = stats.nonzero_events.wrapping_add(1);
        }
        stats.inclusive_ns = stats.inclusive_ns.wrapping_add(elapsed);
        stats.exclusive_ns = stats
            .exclusive_ns
            .wrapping_add(elapsed.saturating_sub(active.children));
        if self.depth > 0 {
            let parent = &mut self.stack[self.depth - 1];
            parent.children = parent.children.saturating_add(elapsed);
        }
    }

    #[inline]
    fn read_clock(&mut self) -> u64 {
        self.snapshot.clock_reads = self.snapshot.clock_reads.wrapping_add(1);
        (self.clock)()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{reg, Cpu, Memory, Mode};
    use std::cell::Cell;
    use std::collections::BTreeMap;

    thread_local! { static TICKS: Cell<u64> = const { Cell::new(0) }; }
    fn clock() -> u64 {
        TICKS.with(|t| {
            let old = t.get();
            t.set(old + 10);
            old
        })
    }
    fn panic_clock() -> u64 {
        panic!("clock called in untimed mode")
    }
    fn zero_clock() -> u64 {
        0
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestMemory {
        bytes: BTreeMap<u32, u8>,
        accesses: Vec<(bool, u32, u8)>,
    }
    impl TestMemory {
        fn new(program: &[u8]) -> Self {
            Self {
                bytes: program
                    .iter()
                    .enumerate()
                    .map(|(i, b)| (i as u32, *b))
                    .collect(),
                accesses: Vec::new(),
            }
        }
    }
    impl Memory for TestMemory {
        fn r8(&mut self, addr: u32) -> u8 {
            let value = *self.bytes.get(&addr).unwrap_or(&0);
            self.accesses.push((false, addr, value));
            value
        }
        fn r16(&mut self, addr: u32) -> u16 {
            u16::from_le_bytes([self.r8(addr), self.r8(addr + 1)])
        }
        fn r32(&mut self, addr: u32) -> u32 {
            self.r16(addr) as u32 | ((self.r16(addr + 2) as u32) << 16)
        }
        fn w8(&mut self, addr: u32, value: u8) {
            self.accesses.push((true, addr, value));
            self.bytes.insert(addr, value);
        }
        fn w16(&mut self, addr: u32, value: u16) {
            for (i, b) in value.to_le_bytes().iter().enumerate() {
                self.w8(addr + i as u32, *b);
            }
        }
        fn w32(&mut self, addr: u32, value: u32) {
            for (i, b) in value.to_le_bytes().iter().enumerate() {
                self.w8(addr + i as u32, *b);
            }
        }
    }

    fn cpu(thumb: bool, mode: ProfileMode, interval: u32) -> Cpu {
        let mut cpu = Cpu::new();
        cpu.reg_set(Mode::User, reg::CPSR, if thumb { 0x30 } else { 0x10 });
        cpu.reg_set(Mode::User, reg::SP, 0x200);
        cpu.set_profiling(
            mode,
            interval,
            31,
            if mode == ProfileMode::Sampled {
                clock
            } else {
                panic_clock
            },
        );
        cpu
    }

    /// Every upstream executable fixture must produce the exact same register
    /// banks, memory bytes, access order and step result after every instruction.
    #[test]
    fn all_fixtures_are_identical_in_off_counts_and_sampled_modes() {
        let fixtures: &[(bool, &[u8])] = &[
            (false, include_bytes!("../tests/data/emutest_arm0.bin")),
            (false, include_bytes!("../tests/data/emutest_arm1.bin")),
            (false, include_bytes!("../tests/data/emutest_arm10.bin")),
            (false, include_bytes!("../tests/data/emutest_arm2.bin")),
            (false, include_bytes!("../tests/data/emutest_arm3.bin")),
            (false, include_bytes!("../tests/data/emutest_arm4.bin")),
            (false, include_bytes!("../tests/data/emutest_arm5.bin")),
            (false, include_bytes!("../tests/data/emutest_arm6.bin")),
            (false, include_bytes!("../tests/data/emutest_arm7.bin")),
            (false, include_bytes!("../tests/data/emutest_arm8.bin")),
            (false, include_bytes!("../tests/data/emutest_arm9.bin")),
            (true, include_bytes!("../tests/data/emutest_thm0.bin")),
            (true, include_bytes!("../tests/data/emutest_thm1.bin")),
            (true, include_bytes!("../tests/data/emutest_thm10.bin")),
            (true, include_bytes!("../tests/data/emutest_thm11.bin")),
            (true, include_bytes!("../tests/data/emutest_thm2.bin")),
            (true, include_bytes!("../tests/data/emutest_thm3.bin")),
            (true, include_bytes!("../tests/data/emutest_thm4.bin")),
            (true, include_bytes!("../tests/data/emutest_thm5.bin")),
            (true, include_bytes!("../tests/data/emutest_thm6.bin")),
            (true, include_bytes!("../tests/data/emutest_thm7.bin")),
            (true, include_bytes!("../tests/data/emutest_thm8.bin")),
            (true, include_bytes!("../tests/data/emutest_thm9.bin")),
        ];
        for &(thumb, bytes) in fixtures {
            let mut cpus = [
                cpu(thumb, ProfileMode::Off, 1),
                cpu(thumb, ProfileMode::Counts, 1),
                cpu(thumb, ProfileMode::Sampled, 1),
                cpu(thumb, ProfileMode::Sampled, 256),
            ];
            let mut memories = [
                TestMemory::new(bytes),
                TestMemory::new(bytes),
                TestMemory::new(bytes),
                TestMemory::new(bytes),
            ];
            let mut terminated = false;
            for _ in 0..100_000 {
                let expected = cpus[0].step(&mut memories[0]);
                for i in 1..cpus.len() {
                    assert_eq!(expected, cpus[i].step(&mut memories[i]));
                    assert_eq!(cpus[0], cpus[i]);
                    assert_eq!(memories[0], memories[i]);
                    assert_eq!(cpus[i].profiling.depth, 0);
                }
                for memory in &mut memories {
                    memory.accesses.clear();
                }
                if !expected {
                    terminated = true;
                    break;
                }
            }
            assert!(terminated, "fixture failed to terminate");
            assert_eq!(cpus[0].profiling_snapshot().instruction_attempts, 0);
            assert_eq!(cpus[1].profiling_snapshot().clock_reads, 0);
            let full = cpus[2].profiling_snapshot();
            assert_eq!(full.instruction_attempts, full.sampled_instructions);
            assert_eq!(
                full.instruction_attempts,
                full.arm_instructions + full.thumb_instructions
            );
            assert_eq!(
                full.instruction_attempts,
                full.retired_instructions + full.undefined_instructions
            );
            assert_eq!(full.stack_overflows, 0);
            assert_eq!(full.clock_regressions, 0);
            assert_eq!(
                full.stages.iter().map(|s| s.exclusive_ns).sum::<u64>(),
                full.stages[Stage::Instruction as usize].inclusive_ns
            );
        }
    }

    #[test]
    fn predicates_and_undefined_are_not_counted_as_executed() {
        let mut cpu = cpu(false, ProfileMode::Sampled, 1);
        cpu.reg_set(Mode::User, reg::CPSR, 0x4000_0010); // Z set
        let mut memory = TestMemory::new(&[0x01, 0x00, 0xa0, 0x13, 0x00, 0x00, 0x00, 0xec]); // MOVNE; undefined coprocessor transfer
        assert!(cpu.step(&mut memory));
        assert_eq!(cpu.reg_get(Mode::User, 0), 0);
        let skipped = cpu.profiling_snapshot();
        assert_eq!(
            (
                skipped.instruction_attempts,
                skipped.decoded_instructions,
                skipped.retired_instructions,
                skipped.executed_instructions,
                skipped.condition_failed_instructions
            ),
            (1, 1, 1, 0, 1)
        );
        assert!(!cpu.step(&mut memory));
        let undefined = cpu.profiling_snapshot();
        assert_eq!(
            (
                undefined.instruction_attempts,
                undefined.decoded_instructions,
                undefined.retired_instructions,
                undefined.executed_instructions,
                undefined.undefined_instructions,
                undefined.exception_entries
            ),
            (2, 1, 1, 0, 1, 1)
        );
        assert_eq!(cpu.profiling.depth, 0);
    }

    #[test]
    fn arm_thumb_exchange_and_software_exception_keep_balanced_spans() {
        let mut memory = TestMemory::new(&[]);
        memory.w32(0, 0xe12f_ff10); // BX r0 => Thumb at 0x20
        memory.w16(0x20, 0xdf00); // SVC => ARM vector at 8
        memory.w32(8, 0xec00_0000); // undefined
        let mut cpu = cpu(false, ProfileMode::Sampled, 1);
        cpu.reg_set(Mode::User, 0, 0x21);
        assert!(cpu.step(&mut memory));
        assert!(cpu.thumb_mode());
        assert!(cpu.step(&mut memory));
        assert!(!cpu.thumb_mode());
        assert!(!cpu.step(&mut memory));
        let result = cpu.profiling_snapshot();
        assert_eq!(
            (
                result.arm_instructions,
                result.thumb_instructions,
                result.exception_entries
            ),
            (2, 1, 2)
        );
        assert_eq!(result.stages[Stage::Exception as usize].events, 2);
        assert_eq!(cpu.profiling.depth, 0);
    }

    #[test]
    fn nested_stage_exclusive_time_does_not_double_count() {
        TICKS.with(|t| t.set(0));
        let mut p = Profiler::default();
        p.configure(ProfileMode::Sampled, 1, 1, clock);
        p.begin_instruction();
        let outer = p.enter(Stage::Execute);
        let child = p.enter(Stage::DataRead);
        p.leave(child);
        p.leave(outer);
        p.end_instruction(true);
        let s = p.snapshot();
        assert_eq!(s.stages[Stage::Instruction as usize].inclusive_ns, 50);
        assert_eq!(s.stages[Stage::Instruction as usize].exclusive_ns, 20);
        assert_eq!(s.stages[Stage::Execute as usize].inclusive_ns, 30);
        assert_eq!(s.stages[Stage::Execute as usize].exclusive_ns, 20);
        assert_eq!(s.stages[Stage::DataRead as usize].inclusive_ns, 10);
        assert_eq!(s.stages.iter().map(|s| s.exclusive_ns).sum::<u64>(), 50);
        assert_eq!(s.clock_reads, 6);
    }

    fn positions(seed: u32) -> (Vec<usize>, ProfileSnapshot) {
        let mut p = Profiler::default();
        p.configure(ProfileMode::Sampled, 64, seed, zero_clock);
        let mut result = Vec::new();
        for i in 0..20_000 {
            p.begin_instruction();
            if p.sampled {
                result.push(i);
            }
            p.end_instruction(true);
        }
        (result, p.snapshot())
    }

    #[test]
    fn sampling_jitters_and_reset_repeats_the_selected_sequence() {
        let (a, snapshot) = positions(9);
        assert_eq!(a, positions(9).0);
        assert_ne!(a, positions(10).0);
        let phases: std::collections::BTreeSet<_> = a.iter().map(|x| x % 64).collect();
        assert!(
            phases.len() > 50,
            "sample positions alias a fixed guest loop"
        );
        assert!((250..380).contains(&a.len()));
        assert_eq!(
            snapshot.stages[Stage::Instruction as usize].nonzero_events,
            0
        );
        assert_eq!(
            snapshot.stages[Stage::Instruction as usize].sampled_events,
            a.len() as u64
        );
        let mut p = Profiler::default();
        p.configure(ProfileMode::Sampled, 1, 9, zero_clock);
        p.begin_instruction();
        p.end_instruction(true);
        let before = p.snapshot();
        p.reset();
        assert_eq!(p.snapshot().instruction_attempts, 0);
        p.begin_instruction();
        p.end_instruction(true);
        assert_eq!(before, p.snapshot());
    }

    #[test]
    fn low_resolution_and_regressing_clocks_are_reported_without_underflow() {
        fn regressing_clock() -> u64 {
            TICKS.with(|t| {
                let old = t.get();
                t.set(old.saturating_sub(1));
                old
            })
        }
        let mut p = Profiler::default();
        p.configure(ProfileMode::Sampled, 1, 1, zero_clock);
        p.begin_instruction();
        p.end_instruction(true);
        assert_eq!(p.snapshot().stages[0].nonzero_events, 0);
        assert_eq!(p.snapshot().stages[0].sampled_events, 1);
        TICKS.with(|t| t.set(10));
        p.configure(ProfileMode::Sampled, 1, 1, regressing_clock);
        p.begin_instruction();
        p.end_instruction(true);
        assert_eq!(p.snapshot().clock_regressions, 1);
        assert_eq!(p.snapshot().stages[0].inclusive_ns, 0);
    }

    #[test]
    fn stage_stack_is_bounded_and_overflow_does_not_unbalance_parent() {
        let mut p = Profiler::default();
        p.configure(ProfileMode::Sampled, 1, 1, clock);
        p.begin_instruction();
        let tokens: Vec<_> = (0..MAX_DEPTH).map(|_| p.enter(Stage::Execute)).collect();
        assert_eq!(p.snapshot().stack_overflows, 1);
        for token in tokens.into_iter().rev() {
            p.leave(token);
        }
        p.end_instruction(true);
        assert_eq!(p.depth, 0);
        assert_eq!(
            p.snapshot()
                .stages
                .iter()
                .map(|s| s.exclusive_ns)
                .sum::<u64>(),
            p.snapshot().stages[0].inclusive_ns
        );
    }
}
