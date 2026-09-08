//! Optional host-side observations. These never supply or modify guest time.
//! CPU stage times are sampled inclusive/exclusive observations; wrapper and
//! SVC times have their own scope and must not be added to the CPU stage totals.

pub use arm32_cpu::profiling::{HostClock, ProfileMode, ProfileSnapshot, Stage, StageStats};

#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct BoundaryStats {
    pub events: u64,
    pub sampled_events: u64,
    pub nonzero_samples: u64,
    pub sampled_ns: u64,
}

#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct WrapperSnapshot {
    pub run_calls: u64,
    /// All sampled-mode run() calls are timed, including guest instruction work.
    pub run_inclusive_ns: u64,
    pub count_exhaustions: u64,
    pub function_returns: u64,
    pub svc_exits: u64,
    pub errors: u64,
    pub host_read_bytes: u64,
    pub host_write_bytes: u64,
    pub boundary_checks: BoundaryStats,
    pub host_reads: BoundaryStats,
    pub host_writes: BoundaryStats,
    pub clock_reads: u64,
    pub clock_regressions: u64,
}

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct EngineProfileSnapshot {
    pub cpu: ProfileSnapshot,
    pub wrapper: WrapperSnapshot,
}

#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct SvcSnapshot {
    pub calls: u64,
    pub polls: u64,
    /// Active Future::poll time, including nested guest execution and nested
    /// SVCs. Time awaiting a pending future is deliberately excluded.
    pub poll_inclusive_ns: u64,
    pub clock_regressions: u64,
}

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct CoreProfileSnapshot {
    pub engine: EngineProfileSnapshot,
    pub svc: SvcSnapshot,
}

#[derive(Clone, Copy)]
pub(crate) enum Boundary {
    Checks,
    HostRead,
    HostWrite,
}

pub(crate) struct WrapperProfiler {
    pub mode: ProfileMode,
    pub clock: HostClock,
    pub stats: WrapperSnapshot,
    interval: u32,
    seed: u32,
    random: u32,
    until_sample: [u32; 3],
}

impl Default for WrapperProfiler {
    fn default() -> Self {
        Self {
            mode: ProfileMode::Off,
            clock: || 0,
            stats: WrapperSnapshot::default(),
            interval: 1024,
            seed: 1,
            random: 1,
            until_sample: [0; 3],
        }
    }
}

impl WrapperProfiler {
    pub fn configure(&mut self, mode: ProfileMode, interval: u32, seed: u32, clock: HostClock) {
        *self = Self {
            mode,
            clock,
            interval: interval.clamp(1, 1 << 30),
            seed,
            random: (seed ^ 0xa5a5_19d3).max(1),
            ..Self::default()
        };
    }

    pub fn reset(&mut self) {
        self.configure(self.mode, self.interval, self.seed, self.clock);
    }

    pub fn now(&mut self) -> u64 {
        self.stats.clock_reads += 1;
        (self.clock)()
    }

    pub fn begin_run(&mut self) -> Option<u64> {
        if self.mode == ProfileMode::Off {
            return None;
        }
        self.stats.run_calls += 1;
        (self.mode == ProfileMode::Sampled).then(|| self.now())
    }

    pub fn end_run(&mut self, token: Option<u64>) {
        if let Some(started) = token {
            let ended = self.now();
            self.stats.clock_regressions += u64::from(ended < started);
            self.stats.run_inclusive_ns += ended.saturating_sub(started);
        }
    }

    fn stage(&mut self, stage: Boundary) -> &mut BoundaryStats {
        match stage {
            Boundary::Checks => &mut self.stats.boundary_checks,
            Boundary::HostRead => &mut self.stats.host_reads,
            Boundary::HostWrite => &mut self.stats.host_writes,
        }
    }

    pub fn begin(&mut self, stage: Boundary) -> Option<u64> {
        if self.mode == ProfileMode::Off {
            return None;
        }
        self.stage(stage).events += 1;
        if self.mode != ProfileMode::Sampled {
            return None;
        }
        let index = stage as usize;
        if self.until_sample[index] != 0 {
            self.until_sample[index] -= 1;
            return None;
        }
        self.random ^= self.random << 13;
        self.random ^= self.random >> 17;
        self.random ^= self.random << 5;
        self.until_sample[index] = self.random % (2 * self.interval - 1);
        self.stage(stage).sampled_events += 1;
        Some(self.now())
    }

    pub fn end(&mut self, stage: Boundary, token: Option<u64>) {
        if let Some(started) = token {
            let ended = self.now();
            self.stats.clock_regressions += u64::from(ended < started);
            let elapsed = ended.saturating_sub(started);
            let stats = self.stage(stage);
            stats.nonzero_samples += u64::from(elapsed != 0);
            stats.sampled_ns += elapsed;
        }
    }
}
