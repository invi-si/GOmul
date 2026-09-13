//! Failure and manual evidence, separate from player checkpoints. No per-tick file I/O.
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
        self.capture_to(&self.root, error, frame, slots, false)?;
        self.captured = true;
        Ok(())
    }
    pub fn capture_manual(&self, frame: &[u8], slots: &checkpoint::Slots) -> anyhow::Result<PathBuf> {
        let root = self.root.join("manual");
        self.capture_to(&root, "Manual rescue requested by user (no crash required)", frame, slots, true)?;
        Ok(root.join("latest"))
    }
    fn capture_to(&self, root: &Path, error: &str, frame: &[u8], slots: &checkpoint::Slots, manual: bool) -> anyhow::Result<()> {
        // Snapshot under the short recording lock, then release it before file I/O.
        // Manual capture must not wait indefinitely for a stuck worker's lock.
        let (bytes, safe_bytes, recording_error) = {
            let tape = if manual {
                match self.tape.try_lock() {
                    Ok(tape) => tape,
                    Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
                    Err(std::sync::TryLockError::WouldBlock) => anyhow::bail!("Session recording is busy; try again"),
                }
            } else {
                self.tape.lock().unwrap_or_else(|e| e.into_inner())
            };
            (tape.bytes.clone(), tape.safe_bytes(), tape.error.clone())
        };
        fs::create_dir_all(root)?;
        let writing = root.join("writing");
        if writing.exists() {
            fs::remove_dir_all(&writing)?;
        }
        fs::create_dir(&writing)?;
        checkpoint::write_tree(&writing.join("initial"), &self.initial)?;
        fs::write(writing.join("trace"), &bytes)?;
        fs::write(writing.join("safe-trace"), &safe_bytes)?;
        fs::write(writing.join("frame"), frame)?;
        fs::write(writing.join("archive-id"), &self.archive)?;
        fs::write(writing.join("build-id"), option_env!("GOMUL_CHECKPOINT_BUILD").unwrap_or("unknown"))?;
        fs::write(writing.join("error.txt"), error)?;
        fs::write(
            writing.join("recording-status.txt"),
            recording_error.as_deref().unwrap_or(if manual {
                "complete through manual capture; current tick may be unfinished"
            } else {
                "complete through observed failure"
            }),
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
                "GOmul Rescue v1\nCreated UTC milliseconds: {created}\nCheckpoint included: {has_quick}\nSafe boundary bytes: {}\n\ntrace ends at the observed capture point and may include an unfinished operation. safe-trace ends after the last completed tick.\nThese recordings require the matching game and emulator build. They are not RAM snapshots.\nOnly checkpoint/ is a normal validated-load checkpoint; safe-trace has no final-state oracle.\nMay contain private save data and game content. Share privately.\n",
                safe_bytes.len()
            ),
        )?;
        publish(root)?;
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

#[cfg(test)]
mod manual_tests {
    use super::*;
    #[test]
    fn manual_captures_are_repeatable_and_do_not_consume_automatic_rescue() -> anyhow::Result<()> {
        let dir = std::env::temp_dir().join(format!(
            "gomul-manual-rescue-{}",
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));
        let save = dir.join("saves/game-a");
        fs::create_dir_all(&save)?;
        fs::write(save.join("player"), b"initial")?;
        let slots = checkpoint::Slots::new(save.to_str().unwrap())?;
        let tape = Arc::new(Mutex::new(Tape::default()));
        tape.lock().unwrap().tick();
        tape.lock().unwrap().mark_safe();
        tape.lock().unwrap().tick(); // Simulate an unfinished guest tick.
        let mut rescue = Rescue::new(&slots, tape.clone(), b"game-a-hash");
        fs::create_dir_all(slots.slot("quick"))?;
        fs::write(slots.slot("quick").join("trace"), b"do not overwrite")?;
        fs::write(save.join("player"), b"current")?;
        let path = rescue.capture_manual(b"screen-one", &slots)?;
        assert!(!rescue.captured);
        assert_eq!(fs::read(path.join("frame"))?, b"screen-one");
        assert_eq!(fs::read(path.join("initial/player"))?, b"initial");
        assert!(fs::metadata(path.join("safe-trace"))?.len() < fs::metadata(path.join("trace"))?.len());
        assert!(fs::read_to_string(path.join("error.txt"))?.contains("Manual"));
        {
            let _busy = tape.lock().unwrap();
            assert!(rescue.capture_manual(b"locked", &slots).is_err());
        }
        assert_eq!(fs::read(path.join("frame"))?, b"screen-one");
        rescue.capture_manual(b"screen-two", &slots)?;
        assert_eq!(fs::read(path.join("frame"))?, b"screen-two");
        assert_eq!(fs::read(path.parent().unwrap().join("previous/frame"))?, b"screen-one");
        rescue.capture("actual failure", b"failed-screen", &slots)?;
        assert_eq!(fs::read(rescue.root.join("latest/frame"))?, b"failed-screen");
        assert_eq!(fs::read(path.join("frame"))?, b"screen-two");
        assert_eq!(fs::read(save.join("player"))?, b"current");
        assert_eq!(fs::read(slots.slot("quick").join("trace"))?, b"do not overwrite");
        let other_save = dir.join("saves/game-b");
        fs::create_dir_all(&other_save)?;
        let other_slots = checkpoint::Slots::new(other_save.to_str().unwrap())?;
        let other = Rescue::new(&other_slots, tape, b"game-b-hash");
        let other_path = other.capture_manual(b"other-screen", &other_slots)?;
        assert_ne!(path, other_path);
        assert_eq!(fs::read(path.join("archive-id"))?, b"game-a-hash");
        assert_eq!(fs::read(other_path.join("archive-id"))?, b"game-b-hash");
        fs::remove_dir_all(dir)?;
        Ok(())
    }
}
