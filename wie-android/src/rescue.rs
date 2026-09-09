//! Failure evidence, separate from player checkpoints. No per-tick file I/O.
use crate::checkpoint::{self, Tape};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

pub struct Rescue {
    root: PathBuf,
    initial: std::collections::BTreeMap<PathBuf, Option<Vec<u8>>>,
    pub tape: Arc<Mutex<Tape>>,
    archive: Vec<u8>,
    captured: bool,
}
impl Rescue {
    pub fn new(slots: &checkpoint::Slots, tape: Arc<Mutex<Tape>>, archive: &[u8]) -> Self {
        Self {
            root: slots
                .root
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("rescues")
                .join(slots.save.file_name().unwrap()),
            initial: slots.initial.clone(),
            tape,
            archive: archive.to_vec(),
            captured: false,
        }
    }
    pub fn reset(&mut self, slots: &checkpoint::Slots, tape: Arc<Mutex<Tape>>) {
        self.initial = slots.initial.clone();
        self.tape = tape;
        self.captured = false;
    }
    pub fn capture(&mut self, error: &str, frame: &[u8], slots: &checkpoint::Slots) -> anyhow::Result<()> {
        if self.captured {
            return Ok(());
        }
        fs::create_dir_all(&self.root)?;
        let writing = self.root.join("writing");
        if writing.exists() {
            fs::remove_dir_all(&writing)?;
        }
        fs::create_dir(&writing)?;
        let tape = self.tape.lock().unwrap_or_else(|e| e.into_inner());
        checkpoint::write_tree(&writing.join("initial"), &self.initial)?;
        fs::write(writing.join("trace"), &tape.bytes)?;
        let safe_bytes = tape.safe_bytes();
        fs::write(writing.join("safe-trace"), &safe_bytes)?;
        fs::write(writing.join("frame"), frame)?;
        fs::write(writing.join("archive-id"), &self.archive)?;
        fs::write(writing.join("build-id"), option_env!("GOMUL_CHECKPOINT_BUILD").unwrap_or("unknown"))?;
        fs::write(writing.join("error.txt"), error)?;
        fs::write(
            writing.join("recording-status.txt"),
            tape.error.as_deref().unwrap_or("complete through observed failure"),
        )?;
        // Copy only a completed same-game, same-build local slot; never touch quick save.
        let quick = slots.slot("quick");
        let mut has_quick = false;
        if fs::read(quick.join("archive-id")).ok().as_deref() == Some(&self.archive)
            && fs::read(quick.join("build-id")).ok() == fs::read(writing.join("build-id")).ok()
        {
            let destination = writing.join("checkpoint");
            let copied = (|| -> anyhow::Result<()> {
                fs::create_dir(&destination)?;
                for part in ["initial", "expected"] {
                    checkpoint::write_tree(&destination.join(part), &checkpoint::tree(&quick.join(part))?)?;
                }
                for part in ["trace", "frame", "archive-id", "format", "build-id"] {
                    fs::copy(quick.join(part), destination.join(part))?;
                }
                Ok(())
            })();
            match copied {
                Ok(()) => has_quick = true,
                Err(error) => {
                    fs::remove_dir_all(&destination)?;
                    fs::write(writing.join("checkpoint-error.txt"), error.to_string())?;
                }
            }
        }
        let created = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
        fs::write(
            writing.join("report.txt"),
            format!(
                "GOmul Rescue v1\nCreated UTC milliseconds: {created}\nCheckpoint included: {has_quick}\nSafe boundary bytes: {}\n\ntrace includes the failing operation. safe-trace ends after the last completed tick.\nThese recordings require the matching game and emulator build. They are not RAM snapshots.\nOnly checkpoint/ is a normal validated-load checkpoint; safe-trace has no final-state oracle.\nMay contain private save data and game content. Share privately.\n",
                safe_bytes.len()
            ),
        )?;
        drop(tape);
        publish(&self.root)?;
        self.captured = true;
        Ok(())
    }
}
fn publish(root: &Path) -> anyhow::Result<()> {
    let latest = root.join("latest");
    let previous = root.join("previous");
    if !latest.exists() && previous.exists() {
        fs::rename(&previous, &latest)?;
    }
    if previous.exists() {
        fs::remove_dir_all(&previous)?;
    }
    if latest.exists() {
        fs::rename(&latest, &previous)?;
    }
    if let Err(error) = fs::rename(root.join("writing"), &latest) {
        if previous.exists() {
            let _ = fs::rename(&previous, &latest);
        }
        return Err(error.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn freezes_failure_without_touching_save_or_quick_slot() -> anyhow::Result<()> {
        let dir = std::env::temp_dir().join(format!(
            "gomul-rescue-test-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));
        let save = dir.join("saves/game");
        fs::create_dir_all(&save)?;
        fs::write(save.join("player"), b"before")?;
        let slots = checkpoint::Slots::new(save.to_str().unwrap())?;
        let quick = slots.root.join("quick");
        checkpoint::write_tree(&quick.join("initial"), &slots.initial)?;
        checkpoint::write_tree(&quick.join("expected"), &slots.initial)?;
        for (part, bytes) in [
            ("archive-id", b"hash".as_slice()),
            ("build-id", option_env!("GOMUL_CHECKPOINT_BUILD").unwrap_or("unknown").as_bytes()),
            ("format", b"test"),
            ("trace", b"old trace"),
            ("frame", b"old frame"),
        ] {
            fs::write(quick.join(part), bytes)?;
        }
        let tape = Arc::new(Mutex::new(Tape::default()));
        tape.lock().unwrap().tick();
        tape.lock().unwrap().mark_safe();
        let mut rescue = Rescue::new(&slots, tape.clone(), b"hash");
        fs::write(save.join("player"), b"after")?;
        // A panic while holding the tape must not prevent failure evidence capture.
        let cloned = tape.clone();
        let _ = std::thread::spawn(move || {
            let mut t = cloned.lock().unwrap();
            t.tick();
            panic!("test failure");
        })
        .join();
        rescue.capture("fatal test", b"frame", &slots)?;
        let report = rescue.root.join("latest");
        assert_eq!(fs::read(report.join("initial/player"))?, b"before");
        assert_eq!(fs::read(save.join("player"))?, b"after");
        assert_eq!(fs::read(report.join("checkpoint/trace"))?, b"old trace");
        assert_eq!(fs::read(quick.join("trace"))?, b"old trace");
        assert_eq!(fs::read(report.join("safe-trace"))?.len() * 2, fs::read(report.join("trace"))?.len());
        rescue.capture("second failure", b"different", &slots)?;
        assert_eq!(fs::read_to_string(report.join("error.txt"))?, "fatal test");
        fs::remove_file(quick.join("frame"))?;
        rescue.reset(&slots, tape.clone());
        rescue.capture("new failure with broken checkpoint", b"frame", &slots)?;
        assert!(report.join("checkpoint-error.txt").is_file());
        assert!(!report.join("checkpoint").exists());
        assert_eq!(fs::read_to_string(rescue.root.join("previous/error.txt"))?, "fatal test");
        fs::remove_dir_all(dir)?;
        Ok(())
    }
    #[test]
    fn interrupted_publish_retains_last_report() -> anyhow::Result<()> {
        let dir = std::env::temp_dir().join(format!(
            "gomul-rescue-publish-{}",
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));
        fs::create_dir_all(dir.join("latest"))?;
        fs::write(dir.join("latest/report.txt"), "old")?;
        assert!(publish(&dir).is_err());
        assert_eq!(fs::read_to_string(dir.join("latest/report.txt"))?, "old");
        fs::remove_dir_all(dir)?;
        Ok(())
    }
}
