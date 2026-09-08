//! Diagnostic-only recording trigger and full-library replay benchmark exports.
use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use wie_core_arm::cpu_replay::{self, CapturedRun, Replay};

pub struct Controller {
    directory: PathBuf,
    checked: Instant,
    remaining: usize,
}
impl Controller {
    pub fn new(save: &str) -> Self {
        Self {
            directory: PathBuf::from(save),
            checked: Instant::now(),
            remaining: 0,
        }
    }
    pub fn poll(&mut self) {
        if let Some(capture) = cpu_replay::take_capture() {
            let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
            let path = self.directory.join(format!("cpu-replay-{stamp}.json"));
            let result = (|| -> anyhow::Result<()> {
                let file = fs::OpenOptions::new().write(true).create_new(true).open(&path)?;
                let mut writer = std::io::BufWriter::new(file);
                serde_json::to_writer(&mut writer, &capture)?;
                std::io::Write::flush(&mut writer)?;
                Ok(())
            })();
            let message = match result {
                Ok(()) => format!(
                    "Saved {}: {:?}, instructions={:?}, memory={} bytes\n",
                    path.display(),
                    capture.exit,
                    capture.instructions(),
                    capture.mapped_bytes()
                ),
                Err(error) => {
                    self.remaining = 0;
                    format!("Capture failed: {error}\n")
                }
            };
            let _ = fs::write(self.directory.join("cpu-replay-status.txt"), message);
        }
        if self.checked.elapsed() >= Duration::from_secs(1) {
            self.checked = Instant::now();
            let trigger = self.directory.join("cpu-replay.request");
            if let Ok(text) = fs::read_to_string(&trigger)
                && fs::remove_file(trigger).is_ok()
            {
                self.remaining = text.trim().parse::<usize>().unwrap_or(1).clamp(1, 8);
            }
        }
        if self.remaining > 0 && cpu_replay::request_capture() {
            self.remaining -= 1;
        }
    }
}
impl Drop for Controller {
    fn drop(&mut self) {
        cpu_replay::cancel_capture();
    }
}

thread_local! { static REPLAY: RefCell<Option<Replay>> = const { RefCell::new(None) }; }
fn load(path: &Path) -> anyhow::Result<Replay> {
    if fs::metadata(path)?.len() > 128 * 1024 * 1024 {
        anyhow::bail!("Capture exceeds 128 MiB");
    }
    let capture: CapturedRun = serde_json::from_reader(std::io::BufReader::new(fs::File::open(path)?))?;
    Replay::new(capture).map_err(|e| anyhow::anyhow!(e))
}
#[unsafe(no_mangle)]
pub extern "C" fn wie_replay_setup() -> u32 {
    match load(Path::new("/data/local/tmp/wie-cpu-replay.json")) {
        Ok(replay) => {
            REPLAY.with(|r| *r.borrow_mut() = Some(replay));
            0
        }
        Err(error) => {
            eprintln!("Replay setup: {error}");
            1
        }
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn wie_replay_reset() -> u32 {
    REPLAY.with(|r| match r.borrow_mut().as_mut() {
        Some(r) => {
            r.reset();
            0
        }
        None => 1,
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn wie_replay_run() -> u32 {
    REPLAY.with(|r| match r.borrow_mut().as_mut() {
        Some(r) => {
            r.run();
            0
        }
        None => 1,
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn wie_replay_validate() -> u32 {
    REPLAY.with(|r| match r.borrow().as_ref() {
        Some(r) => match r.validate() {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("Replay validation: {e}");
                2
            }
        },
        None => 1,
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn wie_replay_instructions() -> u32 {
    REPLAY.with(|r| r.borrow().as_ref().and_then(|r| r.instructions()).unwrap_or(0))
}
