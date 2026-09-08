//! cargo run -p wie-cpu-bench --release --features replay --example replay -- capture.json [iterations]
use std::{fs::File, io::BufReader, time::Instant};
use wie_core_arm::cpu_replay::{CapturedRun, Replay};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let path = args.get(1).ok_or("Expected capture.json [iterations]")?;
    let iterations: u32 = args.get(2).map(|s| s.parse()).transpose()?.unwrap_or(1000);
    if iterations == 0 {
        return Err("Iterations must be positive".into());
    }
    let capture: CapturedRun = serde_json::from_reader(BufReader::new(File::open(path)?))?;
    let mapped_bytes = capture.mapped_bytes();
    let mut replay = Replay::new(capture)?;
    for _ in 0..50 {
        replay.reset();
        replay.run();
        replay.validate()?;
    }
    let mut nanos = 0u128;
    for _ in 0..iterations {
        replay.reset();
        let start = Instant::now();
        replay.run();
        nanos += start.elapsed().as_nanos();
        replay.validate()?;
    }
    println!(
        "{}",
        serde_json::json!({"iterations": iterations, "timedNanoseconds": nanos,
        "instructionsPerRun": replay.instructions(), "mappedBytesCheckedEachRun": mapped_bytes,
        "millionInstructionsPerSecond": replay.instructions().map(|n| n as f64 * iterations as f64 * 1000.0 / nanos as f64),
        "validated": true})
    );
    Ok(())
}
