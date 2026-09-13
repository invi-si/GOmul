//! Experimental session reconstruction. Records host nondeterminism; never serializes Rust futures.
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use wie_backend::{Event, KeyCode};

const LIMIT: usize = 64 * 1024 * 1024;
const BUILD: Option<&str> = option_env!("GOMUL_CHECKPOINT_BUILD");
const MAGIC: &[u8] = b"GOmul replay alpha4 v1\0";
const KEYS: [KeyCode; 24] = [
    KeyCode::UP,
    KeyCode::DOWN,
    KeyCode::LEFT,
    KeyCode::RIGHT,
    KeyCode::OK,
    KeyCode::LEFT_SOFT_KEY,
    KeyCode::RIGHT_SOFT_KEY,
    KeyCode::CLEAR,
    KeyCode::CALL,
    KeyCode::HANGUP,
    KeyCode::VOLUME_UP,
    KeyCode::VOLUME_DOWN,
    KeyCode::NUM0,
    KeyCode::NUM1,
    KeyCode::NUM2,
    KeyCode::NUM3,
    KeyCode::NUM4,
    KeyCode::NUM5,
    KeyCode::NUM6,
    KeyCode::NUM7,
    KeyCode::NUM8,
    KeyCode::NUM9,
    KeyCode::HASH,
    KeyCode::STAR,
];

pub enum Step {
    Event(Event),
    Tick,
    ConsumeRedraw,
}

#[derive(Default)]
pub struct Tape {
    // Presentation mirror reconstructed from the recorded input stream.
    pub korean_input: bool,
    pub bytes: Vec<u8>,
    cursor: Option<usize>,
    repeated: u32,
    last_clock: u64,
    clock_record: Option<usize>,
    pub error: Option<String>,
    offset: i128,
    speed_clock: Option<(u64, u64, u32)>,
    safe_end: usize,
    safe_clock: Option<(usize, [u8; 4])>,
}
impl Tape {
    /// Keep a tiny bookmark; clock run-length compression can still extend the
    /// last record, so retain its count as it was at the successful boundary.
    pub fn mark_safe(&mut self) {
        if self.error.is_none() && self.cursor.is_none() {
            self.safe_end = self.bytes.len();
            self.safe_clock = self.clock_record.map(|pos| (pos + 9, self.bytes[pos + 9..pos + 13].try_into().unwrap()));
        }
    }
    pub fn safe_bytes(&self) -> Vec<u8> {
        let mut bytes = self.bytes[..self.safe_end].to_vec();
        if let Some((pos, count)) = self.safe_clock {
            bytes[pos..pos + 4].copy_from_slice(&count);
        }
        bytes
    }
    pub fn replay(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            cursor: Some(0),
            ..Self::default()
        }
    }
    pub fn check(&self) -> anyhow::Result<()> {
        if let Some(e) = &self.error {
            anyhow::bail!("{e}");
        }
        Ok(())
    }
    fn fail(&mut self) {
        self.error = Some("Checkpoint replay diverged; current session retained".into());
    }
    fn append(&mut self, bytes: &[u8]) {
        self.clock_record = None;
        if self.error.is_some() {
            return;
        }
        if self.bytes.len() + bytes.len() > LIMIT {
            self.error = Some("Session recording limit reached (64 MiB). Previous checkpoint is still available.".into());
        } else {
            self.bytes.extend_from_slice(bytes);
        }
    }
    fn take(&mut self, len: usize) -> Option<&[u8]> {
        let pos = self.cursor?;
        if pos.checked_add(len).is_none_or(|end| end > self.bytes.len()) {
            self.fail();
            return None;
        }
        self.cursor = Some(pos + len);
        Some(&self.bytes[pos..pos + len])
    }
    fn expect(&mut self, tag: u8) -> bool {
        if self.repeated != 0 {
            self.fail();
            return false;
        }
        if self.take(1) == Some(&[tag][..]) {
            true
        } else {
            self.fail();
            false
        }
    }
    fn word(&mut self) -> u64 {
        self.take(8).map(|v| u64::from_le_bytes(v.try_into().unwrap())).unwrap_or(0)
    }
    fn live_wall(&self, host: u64) -> u64 {
        match self.speed_clock {
            Some((anchor, value, rate)) => value.saturating_add(host.saturating_sub(anchor).saturating_mul(rate as u64) / 1000),
            None => host,
        }
    }
    pub fn set_speed(&mut self, rate: u32) {
        let host = wall();
        self.speed_clock = Some((host, self.live_wall(host), rate.clamp(250, 3000)));
    }
    pub fn now(&mut self) -> u64 {
        if self.cursor.is_some() {
            if self.error.is_some() {
                self.last_clock = self.last_clock.saturating_add(100);
                return self.last_clock;
            }
            if self.repeated != 0 {
                self.repeated -= 1;
                return self.last_clock;
            }
            if !self.expect(1) {
                return self.last_clock;
            }
            self.last_clock = self.word();
            self.repeated = self
                .take(4)
                .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
                .unwrap_or(1)
                .saturating_sub(1);
            return self.last_clock;
        }
        let value = (self.live_wall(wall()) as i128 + self.offset).max(0) as u64;
        if let Some(pos) = self.clock_record {
            if self.last_clock == value {
                let count = u32::from_le_bytes(self.bytes[pos + 9..pos + 13].try_into().unwrap());
                if count < u32::MAX {
                    self.bytes[pos + 9..pos + 13].copy_from_slice(&(count + 1).to_le_bytes());
                    return value;
                }
            }
        }
        self.last_clock = value;
        let pos = self.bytes.len();
        let mut data = vec![1];
        data.extend(value.to_le_bytes());
        data.extend(1u32.to_le_bytes());
        self.append(&data);
        if self.error.is_none() {
            self.clock_record = Some(pos);
        }
        value
    }
    pub fn order(&mut self, tasks: &mut [usize]) -> wie_util::Result<()> {
        if self.cursor.is_some() {
            if self.expect(2) {
                let count = self.word() as usize;
                if count != tasks.len() {
                    self.fail();
                } else {
                    let mut remaining = tasks.to_vec();
                    for task in tasks {
                        let id = self.word() as usize;
                        if let Some(pos) = remaining.iter().position(|&v| v == id) {
                            remaining.swap_remove(pos);
                            *task = id;
                        } else {
                            self.fail();
                            break;
                        }
                    }
                }
            }
            if let Some(e) = &self.error {
                return Err(wie_util::WieError::FatalError(e.clone()));
            }
        } else {
            if self.error.is_some() {
                return Ok(());
            }
            let mut data = vec![2];
            data.extend((tasks.len() as u64).to_le_bytes());
            for &id in tasks.iter() {
                data.extend((id as u64).to_le_bytes());
            }
            self.append(&data);
        }
        Ok(())
    }
    pub fn event(&mut self, event: &Event) {
        if let Event::TextInputMode(korean) = event {
            self.korean_input = *korean;
        }
        let (kind, key) = match event {
            Event::Redraw => (0, 0),
            Event::TextInputMode(korean) => (4, u8::from(*korean)),
            Event::Keydown(k) => (1, key(*k)),
            Event::Keyup(k) => (2, key(*k)),
            Event::Keyrepeat(k) => (3, key(*k)),
            _ => unreachable!(),
        };
        self.append(&[4, kind, key]);
    }
    pub fn redraw(&mut self) {
        self.append(&[5]);
    }
    pub fn tick(&mut self) {
        self.append(&[3]);
    }
    pub fn next(&mut self) -> anyhow::Result<Step> {
        self.check()?;
        let tag = self.take(1).and_then(|x| x.first().copied());
        match tag {
            Some(3) => Ok(Step::Tick),
            Some(5) => Ok(Step::ConsumeRedraw),
            Some(4) => {
                let bytes = self.take(2).ok_or_else(|| anyhow::anyhow!("Truncated checkpoint event"))?;
                if bytes[0] == 4 {
                    anyhow::ensure!(bytes[1] <= 1, "Invalid text input mode");
                    let korean = bytes[1] != 0;
                    self.korean_input = korean;
                    return Ok(Step::Event(Event::TextInputMode(korean)));
                }
                let code = *KEYS.get(bytes[1] as usize).ok_or_else(|| anyhow::anyhow!("Invalid checkpoint key"))?;
                Ok(Step::Event(match bytes[0] {
                    0 => Event::Redraw,
                    1 => Event::Keydown(code),
                    2 => Event::Keyup(code),
                    3 => Event::Keyrepeat(code),
                    _ => anyhow::bail!("Invalid checkpoint event"),
                }))
            }
            _ => anyhow::bail!("Checkpoint event boundary mismatch"),
        }
    }
    pub fn done(&self) -> bool {
        self.cursor == Some(self.bytes.len()) && self.repeated == 0
    }
    pub fn resume(&mut self) -> anyhow::Result<()> {
        self.check()?;
        anyhow::ensure!(self.done(), "Unconsumed checkpoint recording");
        self.cursor = None;
        self.offset = self.last_clock as i128 - wall() as i128;
        self.clock_record = None;
        Ok(())
    }
}
fn wall() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}
fn key(k: KeyCode) -> u8 {
    KEYS.iter().position(|&x| x == k).unwrap() as u8
}

pub fn tree(path: &Path) -> anyhow::Result<BTreeMap<PathBuf, Option<Vec<u8>>>> {
    fn visit(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>, total: &mut usize) -> anyhow::Result<()> {
        if !dir.exists() {
            return Ok(());
        }
        for e in fs::read_dir(dir)? {
            let e = e?;
            let ty = e.file_type()?;
            if ty.is_dir() {
                out.insert(e.path().strip_prefix(root)?.to_owned(), None);
                visit(root, &e.path(), out, total)?;
            } else if ty.is_file() {
                *total += e.metadata()?.len() as usize;
                anyhow::ensure!(*total <= LIMIT, "Game data exceeds checkpoint limit");
                out.insert(e.path().strip_prefix(root)?.to_owned(), Some(fs::read(e.path())?));
            } else {
                anyhow::bail!("Unsupported link in game data");
            }
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    visit(path, path, &mut out, &mut 0)?;
    Ok(out)
}
pub fn write_tree(path: &Path, data: &BTreeMap<PathBuf, Option<Vec<u8>>>) -> anyhow::Result<()> {
    fs::create_dir_all(path)?;
    for (name, bytes) in data {
        let p = path.join(name);
        if let Some(bytes) = bytes {
            fs::create_dir_all(p.parent().unwrap())?;
            fs::write(p, bytes)?;
        } else {
            fs::create_dir_all(p)?;
        }
    }
    Ok(())
}
pub struct Slots {
    pub root: PathBuf,
    pub save: PathBuf,
    pub initial: BTreeMap<PathBuf, Option<Vec<u8>>>,
}
impl Slots {
    pub fn new(save: &str) -> anyhow::Result<Self> {
        let save = PathBuf::from(save);
        let root = save
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("checkpoints")
            .join(save.file_name().unwrap());
        fs::create_dir_all(&root)?;
        let mut slots = Self {
            root,
            save,
            initial: BTreeMap::new(),
        };
        slots.rollback()?;
        slots.initial = tree(&slots.save)?;
        Ok(slots)
    }
    pub fn store(&self, name: &str, tape: &Tape, frame: &[u8], archive: &[u8]) -> anyhow::Result<()> {
        tape.check()?;
        let build = BUILD.ok_or_else(|| anyhow::anyhow!("Checkpoint build identity missing; build using wie-android/build.sh"))?;
        let temp = self.root.join("writing");
        if temp.exists() {
            fs::remove_dir_all(&temp)?;
        }
        write_tree(&temp.join("initial"), &self.initial)?;
        write_tree(&temp.join("expected"), &tree(&self.save)?)?;
        fs::write(temp.join("trace"), &tape.bytes)?;
        fs::write(temp.join("frame"), frame)?;
        fs::write(temp.join("archive-id"), archive)?;
        fs::write(temp.join("format"), MAGIC)?;
        fs::write(temp.join("build-id"), build)?;
        // Commit with a backup so interruption cannot destroy the last completed slot.
        let slot = self.root.join(name);
        let old = self.root.join(format!("{name}-old"));
        if !slot.exists() && old.exists() {
            fs::rename(&old, &slot)?;
        }
        if old.exists() {
            fs::remove_dir_all(&old)?;
        }
        if slot.exists() {
            fs::rename(&slot, &old)?;
        }
        if let Err(e) = fs::rename(&temp, &slot) {
            if old.exists() {
                let _ = fs::rename(&old, &slot);
            }
            return Err(e.into());
        }
        Ok(())
    }
    pub fn slot(&self, name: &str) -> PathBuf {
        let slot = self.root.join(name);
        if slot.exists() { slot } else { self.root.join(format!("{name}-old")) }
    }
    pub fn rollback(&self) -> anyhow::Result<()> {
        let backup = self.root.join("rollback");
        if backup.exists() {
            if self.save.exists() {
                fs::remove_dir_all(&self.save)?;
            }
            fs::rename(backup, &self.save)?;
        }
        Ok(())
    }
    pub fn begin_load(
        &self,
        name: &str,
        archive: &[u8],
    ) -> anyhow::Result<(Tape, BTreeMap<PathBuf, Option<Vec<u8>>>, Vec<u8>, BTreeMap<PathBuf, Option<Vec<u8>>>)> {
        let diagnostic = cfg!(feature = "rescue-replay") && matches!(name, "rescue-capture" | "rescue-verify");
        let slot = self.slot(if diagnostic { "quick" } else { name });
        anyhow::ensure!(
            diagnostic || Some(fs::read_to_string(slot.join("build-id"))?.as_str()) == BUILD,
            "Checkpoint needs the APK build that created it"
        );
        anyhow::ensure!(
            fs::read(slot.join("format"))? == MAGIC,
            "Checkpoint needs the APK version that created it"
        );
        anyhow::ensure!(fs::read(slot.join("archive-id"))? == archive, "Checkpoint belongs to a different game");
        anyhow::ensure!(fs::metadata(slot.join("trace"))?.len() <= LIMIT as u64, "Checkpoint is too large");
        let tape = Tape::replay(fs::read(slot.join("trace"))?);
        let initial = tree(&slot.join("initial"))?;
        let expected = tree(&slot.join("expected"))?;
        let frame = fs::read(slot.join("frame"))?;
        if !self.save.exists() {
            fs::create_dir_all(&self.save)?;
        }
        fs::rename(&self.save, self.root.join("rollback"))?;
        if let Err(e) = write_tree(&self.save, &initial) {
            self.rollback()?;
            return Err(e);
        }
        Ok((tape, initial, frame, expected))
    }
    pub fn commit_load(&self) -> anyhow::Result<()> {
        let committed = self.root.join("committed-old-data");
        if committed.exists() {
            fs::remove_dir_all(&committed)?;
        }
        fs::rename(self.root.join("rollback"), &committed)?;
        // Renaming is the commit point. Cleanup failure must never roll back a partial tree.
        let _ = fs::remove_dir_all(committed);
        Ok(())
    }
}

#[cfg(test)]
mod rescue_boundary_tests {
    use super::*;
    #[test]
    fn text_input_mode_round_trips_in_order_with_keys() {
        let mut tape = Tape::default();
        tape.event(&Event::TextInputMode(true));
        tape.event(&Event::Keydown(KeyCode::NUM4));
        tape.event(&Event::Keyup(KeyCode::NUM4));
        tape.event(&Event::TextInputMode(false));
        let mut replay = Tape::replay(tape.bytes.clone());
        assert!(matches!(replay.next().unwrap(), Step::Event(Event::TextInputMode(true))));
        assert!(matches!(replay.next().unwrap(), Step::Event(Event::Keydown(KeyCode::NUM4))));
        assert!(matches!(replay.next().unwrap(), Step::Event(Event::Keyup(KeyCode::NUM4))));
        assert!(matches!(replay.next().unwrap(), Step::Event(Event::TextInputMode(false))));
        assert!(replay.done());
        assert!(Tape::replay(vec![4, 4, 2]).next().is_err());
    }
    #[test]
    fn safe_prefix_survives_clock_compression_and_failed_tick() {
        let mut tape = Tape::default();
        tape.bytes.push(1);
        tape.bytes.extend(42u64.to_le_bytes());
        tape.bytes.extend(1u32.to_le_bytes());
        tape.clock_record = Some(0);
        tape.mark_safe();
        let before = tape.bytes.clone();
        // The next clock read may increment this already-present record.
        tape.bytes[9..13].copy_from_slice(&2u32.to_le_bytes());
        tape.tick();
        assert_eq!(tape.safe_bytes(), before);
        assert_eq!(u32::from_le_bytes(tape.bytes[9..13].try_into().unwrap()), 2);
        tape.error = Some("recording exhausted".into());
        tape.mark_safe();
        assert_eq!(tape.safe_bytes(), before);
    }
    #[test]
    fn recording_and_replay_are_unchanged_by_boundary_bookmarks() {
        let mut tape = Tape::default();
        tape.event(&Event::Keydown(KeyCode::OK));
        tape.tick();
        let bytes = tape.bytes.clone();
        tape.mark_safe();
        assert_eq!(tape.bytes, bytes);
        let mut replay = Tape::replay(tape.safe_bytes());
        assert!(matches!(replay.next().unwrap(), Step::Event(Event::Keydown(KeyCode::OK))));
        assert!(matches!(replay.next().unwrap(), Step::Tick));
        assert!(replay.done());
    }
}

#[cfg(test)]
mod rescue_build_gate_tests {
    use super::*;
    #[test]
    fn regular_quick_load_always_checks_build_and_preserves_saves() {
        let root = std::env::temp_dir().join(format!("gomul-rescue-gate-{}-{}", std::process::id(), wall()));
        let save = root.join("saves/test");
        fs::create_dir_all(&save).unwrap();
        fs::write(save.join("keep"), b"original").unwrap();
        let slots = Slots::new(save.to_str().unwrap()).unwrap();
        fs::create_dir_all(slots.root.join("quick")).unwrap();
        fs::write(slots.root.join("quick/build-id"), b"unrelated-build").unwrap();
        let error = slots.begin_load("quick", b"archive").err().unwrap().to_string();
        assert!(error.contains("APK build"));
        assert_eq!(fs::read(save.join("keep")).unwrap(), b"original");
        assert!(!slots.root.join("rollback").exists());
        fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod speed_tests {
    use super::Tape;
    #[test]
    fn speed_scales_elapsed_clock_without_changing_anchor() {
        let mut tape = Tape::default();
        assert_eq!(tape.live_wall(5000), 5000);
        for (rate, expected) in [(250, 5250), (1000, 6000), (2000, 7000), (3000, 8000)] {
            tape.speed_clock = Some((1000, 5000, rate));
            assert_eq!(tape.live_wall(1000), 5000);
            assert_eq!(tape.live_wall(2000), expected);
            assert_eq!(tape.live_wall(999), 5000);
        }
    }
    #[test]
    fn speed_setting_does_not_add_replay_events() {
        let mut tape = Tape::default();
        tape.set_speed(2000);
        assert!(tape.bytes.is_empty());
        let value = tape.now();
        let mut replay = Tape::replay(tape.bytes);
        replay.set_speed(250);
        assert_eq!(replay.now(), value);
        assert!(replay.done());
    }
}

// Keep the original quick-save directory as slot 1; recovery/startup remain separate.
pub fn quick_action(action: &str) -> (&str, &str) {
    match action {
        "save:1" => ("save", "quick"),
        "load:1" => ("load", "quick"),
        "save:2" => ("save", "quick-2"),
        "load:2" => ("load", "quick-2"),
        "save:3" => ("save", "quick-3"),
        "load:3" => ("load", "quick-3"),
        _ => (action, "quick"),
    }
}

#[cfg(test)]
mod slot_tests {
    use super::*;
    #[test]
    fn numbered_actions_preserve_legacy_and_special_slots() {
        for (number, directory) in [(1, "quick"), (2, "quick-2"), (3, "quick-3")] {
            for action in ["save", "load"] {
                assert_eq!(quick_action(&format!("{action}:{number}")), (action, directory));
            }
        }
        for action in ["save", "load", "recover", "load-startup", "rescue-verify", "save:4", "load:../quick"] {
            assert_eq!(quick_action(action), (action, "quick"));
        }
    }
    #[test]
    fn numbered_slots_and_games_do_not_overwrite_each_other() {
        if BUILD.is_none() {
            return;
        }
        let root = std::env::temp_dir().join(format!("gomul-three-slots-{}-{}", std::process::id(), wall()));
        for game in ["a", "b"] {
            let save = root.join("saves").join(game);
            fs::create_dir_all(&save).unwrap();
            let slots = Slots::new(save.to_str().unwrap()).unwrap();
            for name in ["quick", "quick-2", "quick-3"] {
                slots
                    .store(name, &Tape::default(), format!("{game}-{name}").as_bytes(), b"archive")
                    .unwrap();
            }
            slots.store("quick-2", &Tape::default(), b"replacement", b"archive").unwrap();
            assert_eq!(fs::read(slots.slot("quick").join("frame")).unwrap(), format!("{game}-quick").as_bytes());
            assert_eq!(
                fs::read(slots.slot("quick-3").join("frame")).unwrap(),
                format!("{game}-quick-3").as_bytes()
            );
            assert_eq!(fs::read(slots.slot("quick-2").join("frame")).unwrap(), b"replacement");
        }
        fs::remove_dir_all(root).unwrap();
    }
}
