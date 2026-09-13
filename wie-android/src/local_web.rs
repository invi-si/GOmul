//! Local browser transport over private parent/child pipes; no listening socket.
use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::io::{BufRead, Write};

fn key(text: &str) -> anyhow::Result<KeyCode> {
    Ok(match text {
        "L" | "LSOFT" => KeyCode::LEFT_SOFT_KEY,
        "R" | "RSOFT" => KeyCode::RIGHT_SOFT_KEY,
        "UP" | "DOWN" | "LEFT" | "RIGHT" | "OK" | "CLR" | "CALL" | "HANGUP" | "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "*"
        | "#" => KeyCode::parse(text),
        _ => anyhow::bail!("Unknown phone key"),
    })
}
fn audio_json(command: AudioCommand) -> Value {
    match command {
        AudioCommand::Stop { handle } => json!({"stop": handle}),
        AudioCommand::Play { handle, sequence, repeat } => {
            let events: Vec<Value> = sequence
                .events
                .iter()
                .map(|event| match &event.data {
                    wie_backend::AudioEventData::Midi(data) => json!([event.time, "midi", data]),
                    wie_backend::AudioEventData::Wave {
                        channels,
                        sampling_rate,
                        samples,
                    } => json!([event.time, "wave", channels, sampling_rate, samples]),
                })
                .collect();
            json!({"handle":handle,"duration":sequence.duration,"repeat":repeat,"events":events})
        }
    }
}
fn command(value: &Value, state: &Shared, tx: &Sender<Command>) -> anyhow::Result<Value> {
    match value["op"].as_str().unwrap_or("") {
        "browser-storage" => {
            let (reply, result) = mpsc::channel();
            tx.send(Command::BrowserStorage(reply))?;
            result.recv()?.map_err(anyhow::Error::msg)?;
            Ok(json!({}))
        }
        "poll" => {
            let mut frame = state.frame.lock().unwrap_or_else(|e| e.into_inner());
            let pixels = if frame.dirty {
                frame.materialize();
                let rgba: Vec<u8> = frame
                    .pixels
                    .iter()
                    .flat_map(|pixel| {
                        let p = *pixel as u32;
                        [(p >> 16) as u8, (p >> 8) as u8, p as u8, 255]
                    })
                    .collect();
                frame.dirty = false;
                Some(STANDARD.encode(rgba))
            } else {
                None
            };
            let result = json!({"width":frame.width,"height":frame.height,"pixels":pixels,"paints":frame.paints,
                "korean":state.korean_input.load(std::sync::atomic::Ordering::Relaxed),
                "status":*state.status.lock().unwrap_or_else(|e| e.into_inner()),
                "audio":state.audio.lock().unwrap().drain(..).map(audio_json).collect::<Vec<_>>()});
            Ok(result)
        }
        "key" => {
            tx.send(Command::Key(
                key(value["key"].as_str().unwrap_or(""))?,
                value["down"].as_bool().unwrap_or(false),
                0,
            ))?;
            Ok(json!({}))
        }
        "pause" => {
            tx.send(Command::Pause(value["paused"].as_bool().unwrap_or(false)))?;
            Ok(json!({}))
        }
        "korean" => {
            tx.send(Command::TextInputMode(value["enabled"].as_bool().unwrap_or(false)))?;
            Ok(json!({}))
        }
        "speed" => {
            tx.send(Command::Speed(value["value"].as_u64().unwrap_or(1000).clamp(250, 2000) as u32))?;
            let mut queue = state.audio.lock().unwrap();
            queue.clear();
            queue.extend(state.active_audio.lock().unwrap().values().cloned());
            Ok(json!({}))
        }
        "checkpoint" => {
            let action = value["action"].as_str().unwrap_or("");
            anyhow::ensure!(
                ["save", "load", "recover", "save-startup", "load-startup"].contains(&action),
                "Unknown checkpoint action"
            );
            let (reply, result) = mpsc::channel();
            tx.send(Command::Checkpoint(action.into(), reply))?;
            Ok(json!({"message":result.recv()?.map_err(anyhow::Error::msg)?}))
        }
        "rescue" => Ok(json!({"path":manual_rescue(state)?.to_string_lossy()})),
        _ => anyhow::bail!("Unknown command"),
    }
}

pub fn serve(path: String, save: String) -> anyhow::Result<()> {
    let state = Arc::new(Shared::default());
    *state.status.lock().unwrap() = "Loading…".into();
    let (tx, rx) = mpsc::channel();
    let worker = state.clone();
    let thread = thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(path, save, worker.clone(), rx)));
        let failed = !matches!(&result, Ok(Ok(())));
        let message = match result {
            Ok(Ok(())) => "Stopped".to_owned(),
            Ok(Err(error)) => format!("Error: {error}"),
            Err(_) => "Error: emulator stopped unexpectedly".to_owned(),
        };
        if failed && !worker.stopping.load(std::sync::atomic::Ordering::Relaxed) {
            capture_rescue(&worker, &message);
        }
        *worker.status.lock().unwrap_or_else(|e| e.into_inner()) = message;
    });
    let mut output = std::io::stdout().lock();
    for line in std::io::stdin().lock().lines() {
        let result = line.map_err(anyhow::Error::from).and_then(|line| {
            anyhow::ensure!(line.len() < 65536, "Command too large");
            command(&serde_json::from_str::<Value>(&line)?, &state, &tx)
        });
        let result = match result {
            Ok(value) => value,
            Err(error) => json!({"error":error.to_string()}),
        };
        writeln!(output, "{}", result)?;
        output.flush()?;
    }
    state.stopping.store(true, std::sync::atomic::Ordering::Relaxed);
    let _ = tx.send(Command::Stop);
    let _ = thread.join();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_keys_without_panicking() {
        assert!(key("bad-key").is_err());
        assert!(matches!(key("LSOFT"), Ok(KeyCode::LEFT_SOFT_KEY)));
    }
}

// Called only by the emulator worker, between guest execution batches. No
// synthetic checkpoint, key release, pause, or timer event is introduced.
fn copy_storage(source: &std::path::Path, target: &std::path::Path, total: &mut u64) -> anyhow::Result<()> {
    if !source.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_storage(&entry.path(), &target.join(entry.file_name()), total)?;
        } else {
            anyhow::ensure!(kind.is_file(), "Unsupported storage link");
            *total += entry.metadata()?.len();
            anyhow::ensure!(*total <= 256 * 1024 * 1024, "Browser storage limit exceeded");
            std::fs::copy(entry.path(), target.join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub(super) fn snapshot_storage(slots: &checkpoint::Slots) -> anyhow::Result<()> {
    let root = slots.root.parent().unwrap().parent().unwrap().join("browser-export");
    if root.exists() {
        std::fs::remove_dir_all(&root)?;
    }
    let identity = slots.save.file_name().unwrap();
    let mut total = 0;
    copy_storage(&slots.save, &root.join("saves").join(identity), &mut total)?;
    for name in ["quick", "quick-old", "recovery", "recovery-old", "startup"] {
        copy_storage(&slots.root.join(name), &root.join("checkpoints").join(identity).join(name), &mut total)?;
    }
    Ok(())
}

#[cfg(test)]
mod browser_storage_tests {
    use super::*;
    #[test]
    fn snapshot_preserves_guest_data_and_separate_slots() -> anyhow::Result<()> {
        let root = std::env::temp_dir().join(format!(
            "gomul-storage-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos()
        ));
        let save = root.join("saves/game");
        std::fs::create_dir_all(save.join("empty"))?;
        std::fs::write(save.join("record"), b"earned")?;
        let slots = checkpoint::Slots::new(save.to_str().unwrap())?;
        for name in ["quick", "startup", "recovery"] {
            std::fs::create_dir_all(slots.root.join(name))?;
            std::fs::write(slots.root.join(name).join("trace"), name)?;
        }
        snapshot_storage(&slots)?;
        assert_eq!(std::fs::read(root.join("browser-export/saves/game/record"))?, b"earned");
        assert!(root.join("browser-export/saves/game/empty").is_dir());
        for name in ["quick", "startup", "recovery"] {
            assert_eq!(
                std::fs::read(root.join("browser-export/checkpoints/game").join(name).join("trace"))?,
                name.as_bytes()
            );
        }
        assert_eq!(std::fs::read(save.join("record"))?, b"earned");
        std::fs::remove_dir_all(root)?;
        Ok(())
    }
}
