use std::{cell::RefCell, hint::black_box};

use serde::Serialize;
use wie_core_arm::{Arm32CpuEngine, ArmEngine, ArmRegister, EngineRunResult, MemoryPermission};

pub const INSTRUCTIONS_PER_BATCH: u32 = 10_000;
const CODE: u32 = 0x10000;
const DATA: u32 = 0x20000;
const MEMORY_SIZE: usize = 0x20000;

#[derive(Clone, Copy, Debug, Serialize)]
pub enum Workload {
    #[serde(rename = "thumb_add")]
    ThumbAdd,
    #[serde(rename = "thumb_memory")]
    ThumbMemory,
    #[serde(rename = "arm_add")]
    ArmAdd,
}

impl Workload {
    pub fn from_id(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(Self::ThumbAdd),
            1 => Ok(Self::ThumbMemory),
            2 => Ok(Self::ArmAdd),
            _ => Err(format!("Unknown workload {id}")),
        }
    }

    fn code(self) -> &'static [u8] {
        match self {
            Self::ThumbAdd => &[0x01, 0x30, 0xfd, 0xe7],
            Self::ThumbMemory => &[0x11, 0x68, 0x01, 0x31, 0x11, 0x60, 0xfb, 0xe7],
            Self::ArmAdd => &[0x01, 0x00, 0x80, 0xe2, 0xfd, 0xff, 0xff, 0xea],
        }
    }

    fn cpsr(self) -> u32 {
        if matches!(self, Self::ArmAdd) { 0x10 } else { 0x30 }
    }
}

#[derive(Debug, Serialize)]
pub struct Validation {
    pub workload: Workload,
    pub instructions: u32,
    pub result: u32,
    pub registers_checked: usize,
    pub memory_bytes_checked: usize,
}

pub struct Benchmark {
    pub engine: Arm32CpuEngine,
    workload: Workload,
    instructions: u32,
    mode: u32,
}

impl Benchmark {
    pub fn new(workload: Workload) -> Result<Self, String> {
        let mut engine = Arm32CpuEngine::new();
        engine.mem_map(CODE, MEMORY_SIZE, MemoryPermission::ReadWriteExecute);
        engine.mem_write(CODE, workload.code()).map_err(|error| format!("{error:?}"))?;
        engine.reg_write(ArmRegister::Cpsr, workload.cpsr());
        engine.reg_write(ArmRegister::PC, CODE);
        engine.reg_write(ArmRegister::R2, DATA);
        Ok(Self {
            engine,
            workload,
            instructions: 0,
            mode: 0,
        })
    }

    pub fn configure(&mut self, mode: u32, interval: u32, seed: u32) -> Result<(), String> {
        if mode > 2 || interval == 0 || interval > (1 << 30) {
            return Err("Mode must be 0, 1 or 2; interval must be in 1..=2^30".into());
        }
        #[cfg(feature = "profiling")]
        {
            use wie_core_arm::cpu_profile::ProfileMode;
            let profile_mode = match mode {
                0 => ProfileMode::Off,
                1 => ProfileMode::Counts,
                _ => ProfileMode::Sampled,
            };
            let _ = profile_now();
            self.engine.set_profiling(profile_mode, interval, seed, profile_now);
            self.engine.reset_profiling();
        }
        #[cfg(not(feature = "profiling"))]
        {
            let _ = seed;
            if mode != 0 {
                return Err("This binary was compiled without profiling".into());
            }
        }
        #[cfg(feature = "throughput")]
        self.engine.reset_throughput();
        self.mode = mode;
        Ok(())
    }

    pub fn finish(&mut self) -> Result<serde_json::Value, String> {
        // Capture before validation, which itself reads guest registers and RAM.
        #[cfg(feature = "profiling")]
        let counters = {
            let snapshot = self.engine.profiling_snapshot();
            let observed = if self.mode == 0 { 0 } else { u64::from(self.instructions) };
            let expected_arm = if matches!(self.workload, Workload::ArmAdd) { observed } else { 0 };
            let cpu = &snapshot.cpu;
            let wrapper = &snapshot.wrapper;
            if cpu.instruction_attempts != observed
                || cpu.decoded_instructions != observed
                || cpu.retired_instructions != observed
                || cpu.executed_instructions != observed
                || cpu.arm_instructions != expected_arm
                || cpu.thumb_instructions != observed - expected_arm
                || cpu.condition_failed_instructions != 0
                || cpu.undefined_instructions != 0
                || cpu.exception_entries != 0
                || wrapper.run_calls != observed / u64::from(INSTRUCTIONS_PER_BATCH)
                || wrapper.count_exhaustions != wrapper.run_calls
                || wrapper.errors != 0
                || wrapper.svc_exits != 0
                || wrapper.host_read_bytes != 0
                || wrapper.host_write_bytes != 0
            {
                return Err("Profiling counters disagree with the fixed workload".into());
            }
            if self.mode < 2 && (cpu.sampled_instructions != 0 || cpu.clock_reads != 0 || wrapper.clock_reads != 0) {
                return Err("Profiling off/counts mode unexpectedly sampled the clock".into());
            }
            serde_json::to_value(snapshot).map_err(|error| error.to_string())?
        };
        #[cfg(not(feature = "profiling"))]
        let counters = serde_json::Value::Null;
        #[cfg(feature = "throughput")]
        let throughput = {
            let snapshot = self.engine.throughput_snapshot();
            let expected = u64::from(self.instructions);
            let expected_arm = if matches!(self.workload, Workload::ArmAdd) { expected } else { 0 };
            if snapshot.total_instructions != expected
                || snapshot.arm_instructions != expected_arm
                || snapshot.thumb_instructions != expected - expected_arm
                || snapshot.full_profiling_compiled != cfg!(feature = "profiling")
            {
                return Err("Throughput counters or compiled profiling features disagree with the fixed workload".into());
            }
            serde_json::to_value(snapshot).map_err(|error| error.to_string())?
        };
        #[cfg(not(feature = "throughput"))]
        let throughput = serde_json::Value::Null;
        let validation = self.validate()?;
        Ok(serde_json::json!({
            "validated": true, "validation": validation, "counters": counters,
            "throughput": throughput, "full_profiling_compiled": cfg!(feature = "profiling"),
            "experimental_thumb_table": cfg!(feature = "experimental-thumb-table"),
            "experimental_thumb_blocks": cfg!(feature = "experimental-thumb-blocks"),
            "experimental_pc_local": cfg!(feature = "experimental-pc-local"),
        }))
    }

    pub fn run_batches(&mut self, batches: u32) -> Result<(), String> {
        if batches == 0 {
            return Err("Batches must be positive".into());
        }
        let added = batches.checked_mul(INSTRUCTIONS_PER_BATCH).ok_or("Instruction count overflow")?;
        let total = self.instructions.checked_add(added).ok_or("Instruction count overflow")?;
        for _ in 0..batches {
            match black_box(self.engine.run(0x1ff00, INSTRUCTIONS_PER_BATCH)).map_err(|error| format!("{error:?}"))? {
                EngineRunResult::CountExhausted => {}
                _ => return Err("Workload stopped before its fixed instruction count".into()),
            }
        }
        self.instructions = total;
        Ok(())
    }

    pub fn validate(&mut self) -> Result<Validation, String> {
        let memory_workload = matches!(self.workload, Workload::ThumbMemory);
        let result = self.instructions / if memory_workload { 4 } else { 2 };
        let expected_registers = [
            (ArmRegister::R0, if memory_workload { 0 } else { result }),
            (ArmRegister::R1, if memory_workload { result } else { 0 }),
            (ArmRegister::R2, DATA),
            (ArmRegister::R3, 0),
            (ArmRegister::R4, 0),
            (ArmRegister::R5, 0),
            (ArmRegister::R6, 0),
            (ArmRegister::R7, 0),
            (ArmRegister::R8, 0),
            (ArmRegister::SB, 0),
            (ArmRegister::SL, 0),
            (ArmRegister::FP, 0),
            (ArmRegister::IP, 0),
            (ArmRegister::SP, 0),
            (ArmRegister::LR, 0),
            (ArmRegister::PC, CODE),
            (ArmRegister::Cpsr, self.workload.cpsr()),
        ];
        for (register, expected) in expected_registers {
            let actual = self.engine.reg_read(register);
            if actual != expected {
                return Err(format!("{register:?}: expected {expected:#x}, got {actual:#x}"));
            }
        }
        let mut expected_memory = vec![0; MEMORY_SIZE];
        expected_memory[..self.workload.code().len()].copy_from_slice(self.workload.code());
        if memory_workload {
            let offset = (DATA - CODE) as usize;
            expected_memory[offset..offset + 4].copy_from_slice(&result.to_le_bytes());
        }
        let mut memory = vec![0; MEMORY_SIZE];
        let read = self
            .engine
            .mem_read(CODE, MEMORY_SIZE, &mut memory)
            .map_err(|error| format!("{error:?}"))?;
        if read != MEMORY_SIZE || memory != expected_memory {
            return Err("Guest memory differs from the expected complete mapped region".into());
        }
        Ok(Validation {
            workload: self.workload,
            instructions: self.instructions,
            result,
            registers_checked: expected_registers.len(),
            memory_bytes_checked: MEMORY_SIZE,
        })
    }
}

#[cfg(all(feature = "profiling", target_arch = "wasm32"))]
#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "profile_now"]
    fn imported_profile_now() -> f64;
}

#[cfg(all(feature = "profiling", target_arch = "wasm32"))]
fn profile_now() -> u64 {
    // The diagnostic WebView supplies performance.now() * 1e6; guest time is untouched.
    unsafe { imported_profile_now() as u64 }
}

pub fn calibrate_clock(pairs: u32) -> Result<serde_json::Value, String> {
    if pairs == 0 || pairs > 1_000_000 {
        return Err("Clock calibration pairs must be in 1..=1000000".into());
    }
    #[cfg(not(feature = "profiling"))]
    {
        Err("Clock calibration requires the profiling build".into())
    }
    #[cfg(feature = "profiling")]
    {
        for _ in 0..100 {
            black_box(profile_now());
        }
        let mut deltas = Vec::with_capacity(pairs as usize);
        let mut regressions = 0;
        let loop_start = profile_now();
        for _ in 0..pairs {
            let start = black_box(profile_now());
            let end = black_box(profile_now());
            regressions += u32::from(end < start);
            deltas.push(end.saturating_sub(start));
        }
        let loop_end = profile_now();
        let loop_elapsed_ns = loop_end.saturating_sub(loop_start);
        deltas.sort_unstable();
        let percentile = |percent: usize| deltas[((deltas.len() - 1) * percent) / 100];
        Ok(serde_json::json!({
            "formatVersion": 1,
            "clock": if cfg!(target_arch = "wasm32") { "wasm_to_js_performance_now" } else { "native_instant" },
            "pairs": pairs,
            "loopPairs": pairs,
            "loopElapsedNs": loop_elapsed_ns,
            "meanLoopPairNs": loop_elapsed_ns as f64 / f64::from(pairs),
            "loopIncludesBookkeeping": true,
            "zeroPairs": deltas.iter().filter(|&&delta| delta == 0).count(),
            "regressions": regressions,
            "minNs": deltas[0],
            "p50Ns": percentile(50),
            "p95Ns": percentile(95),
            "p99Ns": percentile(99),
            "maxNs": deltas[deltas.len() - 1],
            "meanNs": deltas.iter().map(|&value| value as f64).sum::<f64>() / f64::from(pairs),
        }))
    }
}

#[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
fn profile_now() -> u64 {
    use std::{sync::OnceLock, time::Instant};
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_nanos() as u64
}

thread_local! {
    static BENCHMARK: RefCell<Option<Benchmark>> = const { RefCell::new(None) };
    static REPORT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn report_error(error: String) -> u32 {
    REPORT.with(|report| *report.borrow_mut() = serde_json::to_vec(&serde_json::json!({"error": error})).unwrap());
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn bench_setup(workload: u32) -> u32 {
    log::set_max_level(log::LevelFilter::Warn);
    match Workload::from_id(workload).and_then(Benchmark::new) {
        Ok(benchmark) => {
            BENCHMARK.with(|state| *state.borrow_mut() = Some(benchmark));
            REPORT.with(|report| report.borrow_mut().clear());
            0
        }
        Err(error) => report_error(error),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bench_run(batches: u32) -> u32 {
    BENCHMARK.with(|state| match state.borrow_mut().as_mut() {
        Some(benchmark) => benchmark.run_batches(batches).map_or_else(report_error, |()| 0),
        None => report_error("Call bench_setup first".into()),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn bench_configure(mode: u32, interval: u32, seed: u32) -> u32 {
    BENCHMARK.with(|state| match state.borrow_mut().as_mut() {
        Some(benchmark) => benchmark.configure(mode, interval, seed).map_or_else(report_error, |()| 0),
        None => report_error("Call bench_setup first".into()),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn bench_calibrate_clock(pairs: u32) -> u32 {
    match calibrate_clock(pairs) {
        Ok(calibration) => {
            REPORT.with(|report| *report.borrow_mut() = serde_json::to_vec(&calibration).unwrap());
            0
        }
        Err(error) => report_error(error),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bench_validate() -> u32 {
    BENCHMARK.with(|state| {
        let result = state
            .borrow_mut()
            .as_mut()
            .ok_or_else(|| "Call bench_setup first".into())
            .and_then(Benchmark::finish);
        match result {
            Ok(validation) => {
                REPORT.with(|report| *report.borrow_mut() = serde_json::to_vec(&validation).unwrap());
                0
            }
            Err(error) => report_error(error),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn bench_report_ptr() -> *const u8 {
    REPORT.with(|report| report.borrow().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn bench_report_len() -> usize {
    REPORT.with(|report| report.borrow().len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_workload_has_identical_results_across_batch_partitions() {
        for workload in [Workload::ThumbAdd, Workload::ThumbMemory, Workload::ArmAdd] {
            let mut one = Benchmark::new(workload).unwrap();
            let mut split = Benchmark::new(workload).unwrap();
            one.run_batches(3).unwrap();
            split.run_batches(1).unwrap();
            split.run_batches(2).unwrap();
            let one = one.validate().unwrap();
            let split = split.validate().unwrap();
            assert_eq!(one.instructions, 30_000);
            assert_eq!(one.instructions, split.instructions);
            assert_eq!(one.result, split.result);
            assert_eq!(one.registers_checked, 17);
            assert_eq!(one.memory_bytes_checked, 131_072);
        }
    }

    #[test]
    fn validation_detects_changes_outside_the_result_word() {
        let mut benchmark = Benchmark::new(Workload::ThumbMemory).unwrap();
        benchmark.run_batches(1).unwrap();
        benchmark.engine.mem_write(DATA + 16, &[1]).unwrap();
        assert!(benchmark.validate().is_err());
    }

    #[test]
    fn invalid_runs_do_not_execute_instructions() {
        let mut benchmark = Benchmark::new(Workload::ArmAdd).unwrap();
        assert!(benchmark.run_batches(0).is_err());
        assert!(benchmark.run_batches(u32::MAX).is_err());
        assert_eq!(benchmark.validate().unwrap().instructions, 0);
    }

    #[cfg(feature = "profiling")]
    #[test]
    fn profiling_modes_preserve_complete_guest_state_and_exact_instruction_counts() {
        for workload in [Workload::ThumbAdd, Workload::ThumbMemory, Workload::ArmAdd] {
            for mode in 0..=2 {
                let mut benchmark = Benchmark::new(workload).unwrap();
                benchmark.configure(mode, 8, 17).unwrap();
                benchmark.run_batches(3).unwrap();
                let report = benchmark.finish().unwrap();
                assert_eq!(report["validation"]["instructions"], 30_000);
                assert_eq!(report["counters"]["cpu"]["instruction_attempts"], if mode == 0 { 0 } else { 30_000 });
                assert_eq!(report["counters"]["wrapper"]["host_read_bytes"], 0);
                if mode == 2 {
                    assert!(report["counters"]["cpu"]["sampled_instructions"].as_u64().unwrap() > 0);
                }
            }
        }
    }

    #[cfg(feature = "profiling")]
    #[test]
    fn paired_clock_calibration_reports_the_measured_loop_separately() {
        let report = calibrate_clock(100).unwrap();
        assert_eq!(report["pairs"], 100);
        assert_eq!(report["loopPairs"], 100);
        assert_eq!(report["loopIncludesBookkeeping"], true);
        assert_eq!(report["regressions"], 0);
        assert!(report["loopElapsedNs"].as_u64().unwrap() >= report["maxNs"].as_u64().unwrap());
        assert!(calibrate_clock(0).is_err());
    }

    #[cfg(feature = "throughput")]
    #[test]
    fn throughput_counts_every_instruction_and_preserves_complete_guest_state() {
        for workload in [Workload::ThumbAdd, Workload::ThumbMemory, Workload::ArmAdd] {
            let mut benchmark = Benchmark::new(workload).unwrap();
            benchmark.configure(0, 256, 17).unwrap();
            benchmark.run_batches(1).unwrap();
            benchmark.run_batches(2).unwrap();
            let report = benchmark.finish().unwrap();
            assert_eq!(report["validated"], true);
            assert_eq!(report["throughput"]["total_instructions"], 30_000);
            assert_eq!(report["throughput"]["full_profiling_compiled"], cfg!(feature = "profiling"));
            benchmark.engine.reset_throughput();
            assert_eq!(benchmark.engine.throughput_snapshot().total_instructions, 0);
        }
    }
}
