//! Native Android adapter. Guest CPU, memory and WIPI services live on one Rust
//! worker; JNI carries input, complete frames and audio commands only.
mod audio;
mod checkpoint;
#[cfg(all(feature = "input-trace", target_os = "android"))]
mod input_trace;
#[cfg(any(test, all(feature = "input-trace", target_os = "android")))]
mod trace_aggregate;
use wie_util::input_trace as trace;
#[cfg(feature = "native-cpu-bench")]
mod cpu_bench;
#[cfg(feature = "cpu-replay")]
mod cpu_replay;
#[path = "../../src/database.rs"]
mod database;
#[path = "../../src/filesystem.rs"]
mod filesystem;
mod loader;

use jni::{
    JNIEnv,
    objects::{JClass, JIntArray, JLongArray, JString},
    sys::{jboolean, jbyteArray, jint, jlong, jstring},
};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{
        Arc, Mutex, OnceLock,
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant as HostInstant},
};
use wie_backend::{AudioCommand, AudioSink, Event, Filesystem, Font, Instant, KeyCode, Platform, Screen, canvas::Image};

#[derive(Default)]
struct Shared {
    frame: Mutex<Frame>,
    audio: Mutex<VecDeque<AudioCommand>>,
    status: Mutex<String>,
    exiting: std::sync::atomic::AtomicBool,
    stopping: std::sync::atomic::AtomicBool,
    replaying: std::sync::atomic::AtomicBool,
    active_audio: Mutex<HashMap<u32, AudioCommand>>,
}
#[derive(Clone)]
struct Frame {
    width: u32,
    height: u32,
    pixels: Vec<i32>,
    dirty: bool,
    redraw: bool,
    paints: u64,
    published_ns: u64,
}
impl Default for Frame {
    fn default() -> Self {
        Self {
            width: 240,
            height: 320,
            pixels: vec![0xff000000u32 as i32; 240 * 320],
            dirty: true,
            redraw: true,
            paints: 0,
            published_ns: 0,
        }
    }
}
#[derive(Clone)]
struct Output(Arc<Shared>);
impl Screen for Output {
    fn resize(&self, width: u32, height: u32) -> wie_util::Result<()> {
        if width == 0 || height == 0 || width > 1024 || height > 1024 {
            return Err(wie_util::WieError::FatalError("Unsupported display size".into()));
        }
        let mut f = self.0.frame.lock().unwrap();
        f.width = width;
        f.height = height;
        f.pixels.resize((width * height) as usize, 0);
        f.dirty = true;
        Ok(())
    }
    fn request_redraw(&self) -> wie_util::Result<()> {
        self.0.frame.lock().unwrap().redraw = true;
        Ok(())
    }
    fn paint(&self, image: &dyn Image) {
        let _span = trace::span(trace::PAINT, 0);
        let pixels: Vec<i32> = image
            .colors()
            .iter()
            .map(|c| (0xff000000u32 | (u32::from(c.r) << 16) | (u32::from(c.g) << 8) | u32::from(c.b)) as i32)
            .collect();
        #[cfg(feature = "frame-diagnostics")]
        tracing::debug!(target: "wie_frames", white_pixels = pixels.iter().filter(|&&p| p == -1).count(), pixels = pixels.len(), "present");
        let mut f = self.0.frame.lock().unwrap();
        f.width = image.width();
        f.height = image.height();
        f.pixels = pixels;
        f.dirty = true;
        f.paints += 1;
        #[cfg(all(feature = "input-trace", target_os = "android"))]
        {
            f.published_ns = input_trace::now();
        }
        trace::event(trace::PUBLISH, b'I', f.paints, f.published_ns);
    }
    fn width(&self) -> u32 {
        self.0.frame.lock().unwrap().width
    }
    fn height(&self) -> u32 {
        self.0.frame.lock().unwrap().height
    }
}
impl AudioSink for Output {
    fn send(&self, c: AudioCommand) {
        match &c {
            AudioCommand::Play { handle, .. } => {
                self.0.active_audio.lock().unwrap().insert(*handle, c.clone());
            }
            AudioCommand::Stop { handle } => {
                self.0.active_audio.lock().unwrap().remove(handle);
            }
        }
        if !self.0.replaying.load(std::sync::atomic::Ordering::Relaxed) {
            self.0.audio.lock().unwrap().push_back(c);
        }
    }
}
struct AndroidPlatform {
    phone_number: Option<String>,
    tape: Arc<Mutex<checkpoint::Tape>>,
    output: Output,
    font: Font,
    db: database::DatabaseRepository,
    fs: filesystem::CliFilesystem,
}
impl Platform for AndroidPlatform {
    fn phone_number(&self) -> Option<&str> {
        self.phone_number.as_deref()
    }
    fn font(&self) -> &Font {
        &self.font
    }
    fn screen(&self) -> &dyn Screen {
        &self.output
    }
    fn now(&self) -> Instant {
        Instant::from_epoch_millis(self.tape.lock().unwrap().now())
    }
    fn task_order(&self, tasks: &mut [usize]) -> wie_util::Result<()> {
        self.tape.lock().unwrap().order(tasks)
    }
    fn database_repository(&self) -> &dyn wie_backend::DatabaseRepository {
        &self.db
    }
    fn filesystem(&self) -> &dyn Filesystem {
        &self.fs
    }
    fn audio_sink(&self) -> Box<dyn AudioSink> {
        Box::new(self.output.clone())
    }
    fn write_stdout(&self, _: &[u8]) {}
    fn write_stderr(&self, b: &[u8]) {
        eprintln!("{}", String::from_utf8_lossy(b));
    }
    fn exit(&self) {
        self.output.0.exiting.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    fn vibrate(&self, _: u64, _: u8) {}
}
enum Command {
    Key(KeyCode, bool, u64),
    Pause(bool),
    Stop,
    Checkpoint(String, Sender<std::result::Result<String, String>>),
}
struct Session {
    shared: Arc<Shared>,
    tx: Sender<Command>,
    thread: JoinHandle<()>,
}
static SESSION: OnceLock<Mutex<Option<Session>>> = OnceLock::new();
fn session() -> &'static Mutex<Option<Session>> {
    SESSION.get_or_init(|| Mutex::new(None))
}
fn stop() {
    let old = session().lock().unwrap().take();
    if let Some(old) = old {
        old.shared.stopping.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = old.tx.send(Command::Stop);
        let _ = old.thread.join();
    }
}
fn run(path: String, save: String, shared: Arc<Shared>, rx: Receiver<Command>) -> anyhow::Result<()> {
    #[cfg(feature = "frame-diagnostics")]
    let _frame_trace = {
        std::fs::create_dir_all(&save)?;
        let file = std::fs::File::create(PathBuf::from(&save).join("frames.log"))?;
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(Mutex::new(file))
            .with_env_filter("warn,wie_frames=debug,wie_midp::classes::javax::microedition::lcdui::display=debug")
            .finish();
        tracing::subscriber::set_default(subscriber)
    };
    #[cfg(feature = "cpu-replay")]
    let mut cpu_capture = cpu_replay::Controller::new(&save);
    let mut slots = checkpoint::Slots::new(&save)?;
    let archive_id = Sha256::digest(std::fs::read(&path)?).to_vec();
    let mut tape = Arc::new(Mutex::new(checkpoint::Tape::default()));
    let mut emulator = create_emulator(&path, &save, shared.clone(), tape.clone())?;
    *shared.status.lock().unwrap() = "Running".into();
    let mut held = HashMap::new();
    let mut paused = false;
    let mut halted = false;
    loop {
        let command = rx.recv_timeout(Duration::from_millis(if paused || halted { 100 } else { 1 }));
        let commands = command.into_iter().chain(rx.try_iter());
        for command in commands {
            match command {
                Command::Stop => return Ok(()),
                Command::Checkpoint(action, reply) => {
                    for (key, _) in held.drain() {
                        deliver(&mut *emulator, &tape, Event::Keyup(key));
                    }
                    let result = (|| -> anyhow::Result<String> {
                        if action == "save" {
                            anyhow::ensure!(!halted, "Cannot save a stopped game; Quick Load remains available");
                            slots.store("quick", &tape.lock().unwrap(), &frame_bytes(&shared.frame.lock().unwrap()), &archive_id)?;
                            return Ok("Quick Save created on this device (experimental).".into());
                        }
                        anyhow::ensure!(action == "load" || action == "recover", "Unknown checkpoint action");
                        let name = if action == "recover" { "recovery" } else { "quick" };
                        // The current runtime remains alive until reconstruction succeeds.
                        let recovery = action == "load" && !halted && tape.lock().unwrap().error.is_none();
                        if recovery {
                            slots.store(
                                "recovery",
                                &tape.lock().unwrap(),
                                &frame_bytes(&shared.frame.lock().unwrap()),
                                &archive_id,
                            )?;
                        }
                        let restored = restore(&path, &save, &shared, &slots, name, &archive_id)?;
                        emulator = restored.0;
                        tape = restored.1;
                        slots.initial = restored.2;
                        halted = false;
                        Ok(if action == "load" && !recovery {
                            "Quick Load complete. No undo was created for the stopped/expired session."
                        } else {
                            "Quick Load complete. Active audio restarts from its beginning."
                        }
                        .into())
                    })()
                    .map_err(|e| e.to_string());
                    if result.is_err() && slots.root.join("rollback").exists() {
                        halted = true;
                    }
                    if action != "save" {
                        let mut queue = shared.audio.lock().unwrap();
                        queue.clear();
                        queue.extend(shared.active_audio.lock().unwrap().values().cloned());
                    }
                    *shared.status.lock().unwrap() = if halted { "Game stopped; Quick Load available" } else { "Running" }.into();
                    let _ = reply.send(result);
                }
                Command::Pause(value) => {
                    paused = value;
                    for (key, _) in held.drain() {
                        deliver(&mut *emulator, &tape, Event::Keyup(key));
                    }
                }
                Command::Key(key, true, id) if !paused && !halted => {
                    trace::event(trace::DEQUEUE, b'I', id, 1);
                    trace::set_input(id);
                    if let std::collections::hash_map::Entry::Vacant(e) = held.entry(key) {
                        e.insert(HostInstant::now());
                        trace::event(trace::DELIVERY, b'I', id, 1);
                        deliver(&mut *emulator, &tape, Event::Keydown(key));
                    }
                    trace::set_input(0);
                }
                Command::Key(key, false, id) => {
                    trace::event(trace::DEQUEUE, b'I', id, 0);
                    trace::set_input(id);
                    if held.remove(&key).is_some() {
                        trace::event(trace::DELIVERY, b'I', id, 0);
                        deliver(&mut *emulator, &tape, Event::Keyup(key));
                    }
                    trace::set_input(0);
                }
                _ => {}
            }
        }
        if paused || halted {
            continue;
        }
        for (&key, time) in &mut held {
            if time.elapsed() > Duration::from_millis(100) {
                *time = HostInstant::now();
                deliver(&mut *emulator, &tape, Event::Keyrepeat(key));
            }
        }
        tape.lock().unwrap().redraw();
        let redraw = {
            let mut f = shared.frame.lock().unwrap();
            std::mem::take(&mut f.redraw)
        };
        if redraw {
            deliver(&mut *emulator, &tape, Event::Redraw);
        }
        tape.lock().unwrap().tick();
        let tick_result = {
            let _span = trace::span(trace::TICK, 0);
            emulator.tick()
        };
        if let Err(error) = tick_result {
            halted = true;
            *shared.status.lock().unwrap() = format!("Error: {error}. Quick Load available.");
            continue;
        }
        #[cfg(feature = "cpu-replay")]
        cpu_capture.poll();
        if shared.exiting.load(std::sync::atomic::Ordering::Relaxed) {
            halted = true;
            *shared.status.lock().unwrap() = "Game stopped; Quick Load available".into();
        }
    }
}
fn create_emulator(
    path: &str,
    save: &str,
    shared: Arc<Shared>,
    tape: Arc<Mutex<checkpoint::Tape>>,
) -> anyhow::Result<Box<dyn wie_backend::Emulator>> {
    let phone_number = match std::fs::read_to_string(PathBuf::from(&save).join("phone-number.txt")) {
        Ok(value) => {
            let value = value.trim();
            anyhow::ensure!(
                value.len() == 11 && value.bytes().all(|c| c.is_ascii_digit()),
                "Phone identity must contain 11 digits"
            );
            Some(value.to_owned())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let platform = AndroidPlatform {
        phone_number,
        tape,
        output: Output(shared.clone()),
        font: Font::try_from_static(include_bytes!("../../assets/neodgm.ttf"))?,
        db: database::DatabaseRepository::at_path(PathBuf::from(&save)),
        fs: filesystem::CliFilesystem::at_path(PathBuf::from(save)),
    };
    loader::load(path, Box::new(platform))
}
fn deliver(emulator: &mut dyn wie_backend::Emulator, tape: &Mutex<checkpoint::Tape>, event: Event) {
    tape.lock().unwrap().event(&event);
    emulator.handle_event(event);
}
fn frame_bytes(frame: &Frame) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(frame.pixels.len() * 4 + 17);
    bytes.extend(frame.width.to_le_bytes());
    bytes.extend(frame.height.to_le_bytes());
    bytes.extend(frame.paints.to_le_bytes());
    bytes.push(u8::from(frame.redraw));
    for p in &frame.pixels {
        bytes.extend(p.to_le_bytes());
    }
    bytes
}
type Restored = (
    Box<dyn wie_backend::Emulator>,
    Arc<Mutex<checkpoint::Tape>>,
    std::collections::BTreeMap<PathBuf, Option<Vec<u8>>>,
);
fn restore(path: &str, save: &str, shared: &Arc<Shared>, slots: &checkpoint::Slots, name: &str, archive: &[u8]) -> anyhow::Result<Restored> {
    let (recording, initial, frame, expected) = slots.begin_load(name, archive)?;
    let before = shared.frame.lock().unwrap().clone();
    let audio_before = shared.active_audio.lock().unwrap().clone();
    let exit_before = shared.exiting.swap(false, std::sync::atomic::Ordering::Relaxed);
    *shared.frame.lock().unwrap() = Frame::default();
    shared.active_audio.lock().unwrap().clear();
    shared.audio.lock().unwrap().clear();
    shared.replaying.store(true, std::sync::atomic::Ordering::Relaxed);
    *shared.status.lock().unwrap() = "Reconstructing checkpoint…".into();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> anyhow::Result<Restored> {
        let tape = Arc::new(Mutex::new(recording));
        let mut candidate = create_emulator(path, save, shared.clone(), tape.clone())?;
        let started = HostInstant::now();
        loop {
            {
                let t = tape.lock().unwrap();
                t.check()?;
                if t.done() {
                    break;
                }
            }
            anyhow::ensure!(!shared.stopping.load(std::sync::atomic::Ordering::Relaxed), "Checkpoint load cancelled");
            anyhow::ensure!(
                started.elapsed() < Duration::from_secs(180),
                "Checkpoint reconstruction exceeded three minutes"
            );
            let step = tape.lock().unwrap().next()?;
            match step {
                checkpoint::Step::Event(event) => candidate.handle_event(event),
                checkpoint::Step::ConsumeRedraw => {
                    shared.frame.lock().unwrap().redraw = false;
                }
                checkpoint::Step::Tick => candidate.tick()?,
            }
        }
        anyhow::ensure!(
            frame_bytes(&shared.frame.lock().unwrap()) == frame,
            "Checkpoint display mismatch; current session retained"
        );
        anyhow::ensure!(
            checkpoint::tree(&slots.save)? == expected,
            "Checkpoint save-data mismatch; current session retained"
        );
        tape.lock().unwrap().resume()?;
        slots.commit_load()?;
        Ok((candidate, tape, initial))
    }));
    let result = match result {
        Ok(result) => result,
        Err(_) => Err(anyhow::anyhow!("Checkpoint reconstruction failed")),
    };
    shared.replaying.store(false, std::sync::atomic::Ordering::Relaxed);
    if result.is_err() {
        *shared.frame.lock().unwrap() = before;
        *shared.active_audio.lock().unwrap() = audio_before;
        shared.exiting.store(exit_before, std::sync::atomic::Ordering::Relaxed);
        slots.rollback()?;
    }
    shared.frame.lock().unwrap().dirty = true;
    shared.audio.lock().unwrap().extend(shared.active_audio.lock().unwrap().values().cloned());
    result
}

fn send(command: Command) {
    if let Some(s) = session().lock().unwrap().as_ref() {
        let _ = s.tx.send(command);
    }
}
fn shared() -> Option<Arc<Shared>> {
    session().lock().unwrap().as_ref().map(|s| s.shared.clone())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_local_wie_nativeapp_NativeBridge_start(mut env: JNIEnv, _: JClass, path: JString, save: JString) {
    let result = (|| -> jni::errors::Result<()> {
        let path: String = env.get_string(&path)?.into();
        let save: String = env.get_string(&save)?.into();
        stop();
        let state = Arc::new(Shared::default());
        *state.status.lock().unwrap() = "Loading…".into();
        let (tx, rx) = mpsc::channel();
        let worker = state.clone();
        let thread = thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(path, save, worker.clone(), rx)));
            *worker.status.lock().unwrap() = match result {
                Ok(Ok(())) => "Stopped".into(),
                Ok(Err(e)) => format!("Error: {e}"),
                Err(_) => "Error: emulator stopped unexpectedly".into(),
            };
        });
        *session().lock().unwrap() = Some(Session { shared: state, tx, thread });
        Ok(())
    })();
    if let Err(e) = result {
        let _ = env.throw_new("java/lang/IllegalStateException", e.to_string());
    }
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_local_wie_nativeapp_NativeBridge_stop(_: JNIEnv, _: JClass) {
    stop();
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_local_wie_nativeapp_NativeBridge_pause(_: JNIEnv, _: JClass, paused: jboolean) {
    send(Command::Pause(paused != 0));
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_local_wie_nativeapp_NativeBridge_checkpoint(mut env: JNIEnv, _: JClass, action: JString) -> jstring {
    let result = (|| -> anyhow::Result<String> {
        let action: String = env.get_string(&action)?.into();
        let (tx, rx) = mpsc::channel();
        {
            let guard = session().lock().unwrap();
            let s = guard.as_ref().ok_or_else(|| anyhow::anyhow!("No running game"))?;
            s.tx.send(Command::Checkpoint(action, tx))?;
        }
        rx.recv()?.map_err(anyhow::Error::msg)
    })();
    match result {
        Ok(message) => env.new_string(message).map_or(std::ptr::null_mut(), |s| s.into_raw()),
        Err(error) => {
            let _ = env.throw_new("java/lang/IllegalStateException", error.to_string());
            std::ptr::null_mut()
        }
    }
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_local_wie_nativeapp_NativeBridge_key(
    mut env: JNIEnv,
    _: JClass,
    key: JString,
    down: jboolean,
    id: jlong,
    event_ns: jlong,
    listener_ns: jlong,
) {
    trace::event(trace::INPUT, b'I', id as u64, event_ns as u64);
    trace::event(25, b'I', id as u64, listener_ns as u64);
    if let Ok(key) = env.get_string(&key) {
        let text: String = key.into();
        if [
            "UP", "DOWN", "LEFT", "RIGHT", "OK", "L", "R", "CLR", "CALL", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "*", "#",
        ]
        .contains(&text.as_str())
        {
            let code = match text.as_str() {
                "L" => KeyCode::LEFT_SOFT_KEY,
                "R" => KeyCode::RIGHT_SOFT_KEY,
                _ => KeyCode::parse(&text),
            };
            trace::event(trace::ENQUEUE, b'B', id as u64, u64::from(down != 0));
            send(Command::Key(code, down != 0, id as u64));
            trace::event(trace::ENQUEUE, b'E', id as u64, u64::from(down != 0));
        }
    }
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_local_wie_nativeapp_NativeBridge_frame(mut env: JNIEnv, _: JClass, out: JIntArray, metadata: JLongArray) -> jlong {
    let Some(s) = shared() else {
        return 0;
    };
    let _span = trace::span(trace::FRAME_COPY, 0);
    let mut f = s.frame.lock().unwrap();
    if !f.dirty {
        return 0;
    }
    if env.get_array_length(&out).unwrap_or(0) < f.pixels.len() as jint {
        let _ = env.throw_new("java/lang/IllegalArgumentException", "Frame buffer too small");
        return 0;
    }
    if env.set_int_array_region(&out, 0, &f.pixels).is_err() {
        return 0;
    }
    if env
        .set_long_array_region(&metadata, 0, &[f.paints as i64, f.published_ns as i64])
        .is_err()
    {
        return 0;
    }
    trace::event(trace::PICKUP, b'I', f.paints, f.published_ns);
    f.dirty = false;
    (i64::from(f.width) << 32) | i64::from(f.height)
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_local_wie_nativeapp_NativeBridge_status(env: JNIEnv, _: JClass) -> jstring {
    let text = shared().map_or_else(|| "Ready".into(), |s| s.status.lock().unwrap().clone());
    env.new_string(text).map_or(std::ptr::null_mut(), |s| s.into_raw())
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_local_wie_nativeapp_NativeBridge_paints(_: JNIEnv, _: JClass) -> jlong {
    shared().map_or(0, |s| s.frame.lock().unwrap().paints as i64)
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_local_wie_nativeapp_NativeBridge_audio(env: JNIEnv, _: JClass) -> jbyteArray {
    let command = shared().and_then(|s| s.audio.lock().unwrap().pop_front());
    let Some(c) = command else {
        return std::ptr::null_mut();
    };
    env.byte_array_from_slice(&audio::packet(c))
        .map_or(std::ptr::null_mut(), |b| b.into_raw())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_local_wie_nativeapp_NativeBridge_tracePoint(_: JNIEnv, _: JClass, kind: jint, id: jlong, value: jlong) {
    #[cfg(all(feature = "input-trace", target_os = "android"))]
    if kind == 200 {
        input_trace::census(value != 0);
        return;
    }
    trace::event(kind as u16, b'I', id as u64, value as u64);
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_local_wie_nativeapp_NativeBridge_traceControl(mut env: JNIEnv, _: JClass, active: jboolean, path: JString) {
    #[cfg(all(feature = "input-trace", target_os = "android"))]
    {
        if active != 0 {
            input_trace::start();
        } else if let Ok(path) = env.get_string(&path) {
            let path: String = path.into();
            if let Err(error) = input_trace::stop(std::path::Path::new(&path)) {
                let _ = env.throw_new("java/lang/IllegalStateException", error.to_string());
            }
        }
    }
    #[cfg(not(all(feature = "input-trace", target_os = "android")))]
    {
        let _ = (env, active, path);
    }
}
