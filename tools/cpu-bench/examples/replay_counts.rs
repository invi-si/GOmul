//! Untimed instruction counts for saved runs, including early SVC/fault exits.
use std::{fs::File, io::BufReader};
use wie_core_arm::cpu_replay::{CapturedRun, Replay};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    for path in std::env::args().skip(1) {
        let capture: CapturedRun = serde_json::from_reader(BufReader::new(File::open(&path)?))?;
        let mut replay = Replay::new(capture)?;
        replay.reset();
        replay.run();
        replay.validate()?;
        let counts = replay.instruction_counts();
        replay.reset();
        replay.run();
        replay.validate()?;
        assert_eq!(counts, replay.instruction_counts());
        println!("{}", serde_json::json!({"file":path,"counts":counts,"validated":true}));
    }
    Ok(())
}
