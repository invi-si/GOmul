//! Native sampling workload only. No timing scores are emitted. Filter native
//! samples by engine function/inline frames; restore and validation are not CPU.
use std::{fs::File, io::BufReader};
use wie_core_arm::cpu_replay::transcript::{Replay, TIMING_ELIGIBLE, Transcript};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !TIMING_ELIGIBLE {
        return Err("Native sampling requires counters/tracing compiled out".into());
    }
    let args: Vec<String> = std::env::args().collect();
    let capture: Transcript = serde_json::from_reader(BufReader::new(File::open(
        args.get(1).ok_or("usage: transcript_native_profile CAPTURE [ROUNDS]")?,
    )?))?;
    let rounds = args.get(2).map(|s| s.parse()).transpose()?.unwrap_or(600usize);
    let mut replay = Replay::new(capture)?;
    for _ in 0..rounds {
        replay.reset();
        while replay.prepare_segment()? {
            replay.run_segment()?;
            replay.validate_segment()?;
        }
        replay.finish()?;
    }
    println!(
        "rounds={rounds} steps_per_round={} validated=true timing_eligible=false native_sampling_workload=true",
        replay.transcript().steps()
    );
    Ok(())
}
