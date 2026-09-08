//! Validation and CPU-only timing of a callback transcript. Restore/delta/state
//! checks are deliberately outside timers. Use a separate verify build first.
use std::{
    fs::File,
    io::BufReader,
    time::{Duration, Instant},
};
use wie_core_arm::cpu_replay::transcript::{Replay, Transcript};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let capture: Transcript = serde_json::from_reader(BufReader::new(File::open(args.get(1).ok_or("usage: transcript CAPTURE [ROUNDS]")?)?))?;
    let mut replay = Replay::new(capture)?;
    let rounds = args.get(2).map(|s| s.parse()).transpose()?.unwrap_or(7usize);
    let instrumented = !wie_core_arm::cpu_replay::transcript::TIMING_ELIGIBLE;
    let clock_start = Instant::now();
    for _ in 0..replay.transcript().segments() {
        let start = Instant::now();
        std::hint::black_box(start.elapsed());
    }
    println!("empty_clock_pairs_ns={}", clock_start.elapsed().as_nanos());
    println!(
        "segments={} steps={} svc={} verification_instrumented={instrumented}",
        replay.transcript().segments(),
        replay.transcript().steps(),
        replay.transcript().svc_exits()
    );
    for round in 0..rounds + 2 {
        replay.reset();
        let mut cpu = Duration::ZERO;
        while replay.prepare_segment()? {
            let start = Instant::now();
            replay.run_segment()?;
            cpu += start.elapsed();
            replay.validate_segment()?;
        }
        replay.finish()?;
        if round >= 2 {
            println!(
                "round={} cpu_ns={} steps_per_sec={:.0} validated=true timing_eligible={}",
                round - 2,
                cpu.as_nanos(),
                replay.transcript().steps() as f64 / cpu.as_secs_f64(),
                !instrumented
            );
        }
    }
    Ok(())
}
