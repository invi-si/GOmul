use std::{fs::File, io::BufReader};
use wie_core_arm::cpu_replay::transcript::{Replay, Transcript, exact_counts};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("usage: transcript_counts CAPTURE")?;
    let capture: Transcript = serde_json::from_reader(BufReader::new(File::open(path)?))?;
    let mut replay = Replay::new(capture)?;
    let mut previous = None;
    for _ in 0..2 {
        replay.reset();
        exact_counts::reset();
        while replay.prepare_segment()? {
            replay.run_segment()?;
            replay.validate_segment()?;
        }
        replay.finish()?;
        let counts = exact_counts::snapshot();
        let steps: u64 = counts.pcs.iter().map(|(_, count)| count).sum();
        if steps != replay.transcript().steps() {
            return Err("instruction counts disagree with captured step count".into());
        }
        if let Some(previous) = previous.as_ref() {
            if previous != &counts {
                return Err("Repeated counters differ".into());
            }
        }
        previous = Some(counts);
    }
    println!(
        "{}",
        serde_json::json!({"validated":true,"identical_counts_twice":true,"timing_eligible":false,"steps":replay.transcript().steps(),"counts":previous.unwrap(),"runs":replay.transcript().run_distribution()})
    );
    Ok(())
}
