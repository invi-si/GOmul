use std::{env, process::ExitCode, time::Instant};

use wie_cpu_bench::{Benchmark, INSTRUCTIONS_PER_BATCH, Workload, calibrate_clock};

fn argument(args: &[String], index: usize, default: u32) -> Result<u32, String> {
    args.get(index)
        .map_or(Ok(default), |value| value.parse().map_err(|_| format!("Invalid integer: {value}")))
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|arg| arg == "--calibrate") {
        if args.len() > 2 {
            return Err("Usage: wie-cpu-bench --calibrate [pairs=10000]".into());
        }
        println!("{}", calibrate_clock(argument(&args, 1, 10000)?)?);
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--help") || args.len() > 6 {
        return Err("Usage: wie-cpu-bench [workload 0..2] [batches=100] [mode 0..2=0] [warmup_batches=100] [interval=4096] [seed=1]".into());
    }
    let workload = Workload::from_id(argument(&args, 0, 0)?)?;
    let batches = argument(&args, 1, 100)?;
    let mode = argument(&args, 2, 0)?;
    let warmup_batches = argument(&args, 3, 100)?;
    let interval = argument(&args, 4, 4096)?;
    let seed = argument(&args, 5, 1)?;
    log::set_max_level(log::LevelFilter::Warn);
    if warmup_batches != 0 {
        let mut warmup = Benchmark::new(workload)?;
        warmup.configure(mode, interval, seed)?;
        warmup.run_batches(warmup_batches)?;
        warmup.finish()?;
    }
    let mut benchmark = Benchmark::new(workload)?;
    benchmark.configure(mode, interval, seed)?;
    let start = Instant::now();
    benchmark.run_batches(batches)?;
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let result = benchmark.finish()?;
    let report = serde_json::json!({
        "formatVersion": 1,
        "runtime": "native",
        "variant": if cfg!(feature = "profiling") { "profiled" } else if cfg!(feature = "throughput") { "throughput" } else if cfg!(feature = "experimental-thumb-table") { "table" } else if cfg!(feature = "experimental-pc-local") { "pc_local" } else { "off" },
        "mode": (["off", "counts", "sampled"][mode as usize]),
        "workload": workload,
        "batches": batches,
        "warmupBatches": warmup_batches,
        "interval": interval,
        "seed": seed,
        "elapsedMs": elapsed_ms,
        "guestInstructions": batches * INSTRUCTIONS_PER_BATCH,
        "validated": result["validated"],
        "validation": result["validation"],
        "counters": result["counters"],
        "throughput": result["throughput"],
        "full_profiling_compiled": result["full_profiling_compiled"],
        "experimental_thumb_table": result["experimental_thumb_table"],
        "experimental_pc_local": result["experimental_pc_local"],
    });
    println!("{report}");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
