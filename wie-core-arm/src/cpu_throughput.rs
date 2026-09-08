//! Minimal instruction throughput counters. The independent `cpu-throughput`
//! feature does not enable the vendor's stage instrumentation. No guest or host
//! clocks are read here; callers measure elapsed host time outside execution.
//!
//! Counts are attempted Cpu::step calls, including undefined/faulting steps.
//! A long Thumb branch remains one step in this interpreter. Host SVC work,
//! boundary checks and zero-budget exits do not count as instructions.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct CpuThroughputSnapshot {
    /// Steps executed through the optional Thumb block path.
    pub block_instructions: u64,
    pub arm_instructions: u64,
    pub thumb_instructions: u64,
    /// Derived from the ARM and Thumb totals when the snapshot is requested.
    pub total_instructions: u64,
    /// True when Cargo feature unification also enabled full stage profiling.
    /// Such a build is not a throughput-only overhead control, even in mode Off.
    pub full_profiling_compiled: bool,
}

#[derive(Default)]
pub(crate) struct ThroughputCounters {
    block: u64,
    arm: u64,
    thumb: u64,
}

impl ThroughputCounters {
    pub fn record_blocks(&mut self, count: u32) {
        self.block = self.block.wrapping_add(u64::from(count));
    }
    /// Flush run-local counts once on every return path, without a timestamp.
    pub fn record_run(&mut self, arm: u32, thumb: u32) {
        self.arm = self.arm.wrapping_add(u64::from(arm));
        self.thumb = self.thumb.wrapping_add(u64::from(thumb));
    }

    pub fn reset(&mut self) {
        self.block = 0;
        self.arm = 0;
        self.thumb = 0;
    }

    pub fn snapshot(&self) -> CpuThroughputSnapshot {
        CpuThroughputSnapshot {
            block_instructions: self.block,
            arm_instructions: self.arm,
            thumb_instructions: self.thumb,
            total_instructions: self.arm.wrapping_add(self.thumb),
            full_profiling_compiled: cfg!(feature = "cpu-profiling"),
        }
    }
}
