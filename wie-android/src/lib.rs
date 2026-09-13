// GOmul provenance: gomul-component:5b3221ec-dfc4-4817-9693-367ec65b6180 (android-launcher); see PROVENANCE.json.
// GOmul contributions: Copyright (c) 2026 invi-si. SPDX-License-Identifier: MIT
//! Native Android adapter. Guest CPU, memory and WIPI services live on one Rust
//! worker; JNI carries input, complete frames and audio commands only.

// Public, inert binary-origin marker; no execution, guest state, or network access.
#[unsafe(no_mangle)]
pub static GOMUL_ORIGIN_ID: [u8; 42] = *b"gomul:004bbd10-6346-4879-b26a-f1cf14832c19";

mod audio;
mod checkpoint;
#[cfg(all(feature = "input-trace", target_os = "android"))]
mod input_trace;
mod rescue;
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
#[cfg(feature = "local-web")]
pub mod local_web;

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
    korean_input: std::sync::atomic::AtomicBool,
    frame: Mutex<Frame>,
    rescue: Mutex<Option<(rescue::Rescue, checkpoint::Slots)>>,
    fast_forward: std::sync::atomic::AtomicBool,
    display_override: Mutex<Option<(u32, u32, bool)>>,
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
    pending_pixels: Option<(wie_backend::canvas::PackedPixelFormat, Vec<u8>)>,
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
            pending_pixels: None,
            dirty: true,
            redraw: true,
            paints: 0,
            published_ns: 0,
        }
    }
}
impl Frame {
    fn materialize(&mut self) {
        if let Some((format, raw)) = self.pending_pixels.take() {
            use wie_backend::canvas::{PackedPixelFormat, PixelType, Rgb565Pixel};
            self.pixels = match format {
                PackedPixelFormat::Rgb565 => raw
                    .chunks_exact(2)
                    .map(|p| {
                        let c = Rgb565Pixel::to_color(u16::from_ne_bytes([p[0], p[1]]));
                        (0xff000000u32 | ((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32) as i32
                    })
                    .collect(),
                PackedPixelFormat::Argb8888 => raw
                    .chunks_exact(4)
                    .map(|p| (u32::from_ne_bytes(p.try_into().unwrap()) | 0xff000000) as i32)
                    .collect(),
            };
        }
    }
}
#[derive(Clone)]
struct Output(Arc<Shared>);
impl Screen for Output {
    fn display_override(&self) -> Option<(u32, u32, bool)> {
        *self.0.display_override.lock().unwrap()
    }
    fn resize(&self, width: u32, height: u32) -> wie_util::Result<()> {
        if width == 0 || height == 0 || width > 1024 || height > 1024 {
            return Err(wie_util::WieError::FatalError("Unsupported display size".into()));
        }
        let mut f = self.0.frame.lock().unwrap();
        f.width = width;
        f.height = height;
        f.materialize();
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
        // Guest drawing still completes. Only conversion for host presentation
        // is deferred, and every paint replaces the canonical pending frame.
        let fast = self.0.fast_forward.load(std::sync::atomic::Ordering::Relaxed);
        let bpp = image.bytes_per_pixel();
        let pending = if let Some(format) = image.packed_pixel_format().filter(|_| fast) {
            let raw = image.raw().into_owned();
            if raw.len() == image.width() as usize * image.height() as usize * bpp as usize {
                Some((format, raw))
            } else {
                None
            }
        } else {
            None
        };
        let pixels = if pending.is_none() {
            Some(
                image
                    .colors()
                    .iter()
                    .map(|c| (0xff000000u32 | ((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32) as i32)
                    .collect(),
            )
        } else {
            None
        };
        let mut f = self.0.frame.lock().unwrap();
        f.width = image.width();
        f.height = image.height();
        f.pending_pixels = pending;
        if let Some(pixels) = pixels {
            f.pixels = pixels;
        }
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
        #[cfg(feature = "compatibility-audit")]
        tracing::warn!("Guest requested platform exit: {}", std::backtrace::Backtrace::force_capture());
        self.output.0.exiting.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    fn vibrate(&self, _: u64, _: u8) {}
}
enum Command {
    TextInputMode(bool),
    Key(KeyCode, bool, u64),
    Pause(bool),
    Speed(u32),
    Stop,
    Checkpoint(String, Sender<std::result::Result<String, String>>),
    #[cfg(feature = "local-web")]
    BrowserStorage(Sender<std::result::Result<(), String>>),
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
        // Replay restores the save tree. Audit logs must survive that replacement,
        // and must not be confused with historical logs bundled in a rescue.
        let file = std::fs::File::create(diagnostic_log_path(&save))?;
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(Mutex::new(file))
            .with_env_filter(std::env::var("GOMUL_DIAGNOSTIC_FILTER").unwrap_or_else(|_| "warn,wie_frames=debug,wie_midp::classes::javax::microedition::lcdui::display=debug,wie_wipi_c::api::database=debug,wie_wipi_java::classes::org::kwis::msp::lcdui::card=debug".into()))
            .finish();
        tracing::subscriber::set_default(subscriber)
    };
    #[cfg(feature = "compatibility-audit")]
    {
        static PANIC_LOG: std::sync::Once = std::sync::Once::new();
        PANIC_LOG.call_once(|| {
            let previous = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                tracing::error!("Native panic: {info}\n{}", std::backtrace::Backtrace::force_capture());
                previous(info);
            }));
        });
    }
    #[cfg(feature = "cpu-replay")]
    let mut cpu_capture = cpu_replay::Controller::new(&save);
    let options = std::fs::read_to_string(PathBuf::from(&save).join("display-options")).unwrap_or_default();
    let values: Vec<u32> = options.split_whitespace().filter_map(|v| v.parse().ok()).collect();
    if values.len() == 3 && (64..=1024).contains(&values[0]) && (64..=1024).contains(&values[1]) {
        *shared.display_override.lock().unwrap() = Some((values[0], values[1], values[2] != 0));
        Output(shared.clone()).resize(values[0], values[1])?;
    }
    let mut slots = checkpoint::Slots::new(&save)?;
    let archive_id = Sha256::digest(std::fs::read(&path)?).to_vec();
    let mut tape = Arc::new(Mutex::new(checkpoint::Tape::default()));
    *shared.rescue.lock().unwrap() = Some((
        rescue::Rescue::new(&slots, tape.clone(), &archive_id),
        checkpoint::Slots {
            root: slots.root.clone(),
            save: slots.save.clone(),
            initial: Default::default(),
        },
    ));
    let mut emulator = create_emulator(&path, &save, shared.clone(), tape.clone())?;
    *shared.status.lock().unwrap() = "Running".into();
    let mut held = HashMap::new();
    let mut speed = 1000;
    let mut paused = false;
    let mut halted = false;
    loop {
        let command = rx.recv_timeout(if paused || halted {
            Duration::from_millis(100)
        } else {
            Duration::from_millis(1)
        });
        let commands = command.into_iter().chain(rx.try_iter());
        for command in commands {
            match command {
                Command::Stop => return Ok(()),
                #[cfg(feature = "local-web")]
                Command::BrowserStorage(reply) => {
                    let _ = reply.send(local_web::snapshot_storage(&slots).map_err(|error| error.to_string()));
                }
                Command::Checkpoint(action, reply) => {
                    let (action, quick_slot) = checkpoint::quick_action(&action);
                    for (key, _) in held.drain() {
                        deliver(&mut *emulator, &tape, Event::Keyup(key));
                    }
                    let result = (|| -> anyhow::Result<String> {
                        if action == "save" || (cfg!(feature = "local-web") && action == "save-startup") {
                            anyhow::ensure!(!halted, "Cannot save a stopped game; Quick Load remains available");
                            slots.store(
                                if action == "save-startup" { "startup" } else { quick_slot },
                                &tape.lock().unwrap(),
                                &frame_bytes(&mut shared.frame.lock().unwrap()),
                                &archive_id,
                            )?;
                            return Ok("Quick Save created on this device (experimental).".into());
                        }
                        let diagnostic = cfg!(feature = "rescue-replay") && matches!(action, "rescue-capture" | "rescue-verify");
                        anyhow::ensure!(
                            action == "load" || action == "recover" || diagnostic || (cfg!(feature = "local-web") && action == "load-startup"),
                            "Unknown checkpoint action"
                        );
                        let name = if cfg!(feature = "local-web") && action == "load-startup" {
                            "startup"
                        } else if diagnostic {
                            action
                        } else if action == "recover" {
                            "recovery"
                        } else {
                            quick_slot
                        };
                        // The current runtime remains alive until reconstruction succeeds.
                        let recovery = action == "load" && !halted && tape.lock().unwrap().error.is_none();
                        if recovery {
                            slots.store(
                                "recovery",
                                &tape.lock().unwrap(),
                                &frame_bytes(&mut shared.frame.lock().unwrap()),
                                &archive_id,
                            )?;
                        }
                        let restored = restore(&path, &save, &shared, &slots, name, &archive_id)?;
                        emulator = restored.0;
                        tape = restored.1;
                        shared
                            .korean_input
                            .store(tape.lock().unwrap().korean_input, std::sync::atomic::Ordering::Relaxed);
                        tape.lock().unwrap().set_speed(speed);
                        slots.initial = restored.2;
                        shared.rescue.lock().unwrap().as_mut().unwrap().0.reset(&slots, tape.clone());
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
                    if action != "save" && action != "save-startup" {
                        let mut queue = shared.audio.lock().unwrap();
                        queue.clear();
                        queue.extend(shared.active_audio.lock().unwrap().values().cloned());
                    }
                    *shared.status.lock().unwrap() = if halted { "Game stopped; Quick Load available" } else { "Running" }.into();
                    let _ = reply.send(result);
                }
                Command::TextInputMode(korean) if !halted => {
                    deliver(&mut *emulator, &tape, Event::TextInputMode(korean));
                    shared.korean_input.store(korean, std::sync::atomic::Ordering::Relaxed);
                }
                Command::Speed(value) => {
                    speed = value.clamp(250, 3000);
                    shared.fast_forward.store(speed > 1000, std::sync::atomic::Ordering::Relaxed);
                    tape.lock().unwrap().set_speed(speed);
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
            let message = format!("Error: {error}. Quick Load available.");
            capture_rescue(&shared, &message);
            *shared.status.lock().unwrap() = message;
            continue;
        }
        tape.lock().unwrap().mark_safe();
        #[cfg(feature = "cpu-replay")]
        cpu_capture.poll();
        if shared.exiting.load(std::sync::atomic::Ordering::Relaxed) {
            halted = true;
            *shared.status.lock().unwrap() = "Game stopped; Quick Load available".into();
        }
    }
}
fn capture_rescue(shared: &Arc<Shared>, message: &str) {
    if let Some((rescue, slots)) = shared.rescue.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        let frame = frame_bytes(&mut shared.frame.lock().unwrap_or_else(|e| e.into_inner()));
        if let Err(error) = rescue.capture(message, &frame, slots) {
            log::warn!("Rescue capture failed: {error}");
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
fn frame_bytes(frame: &mut Frame) -> Vec<u8> {
    frame.materialize();
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
        #[cfg(feature = "rescue-replay")]
        if name == "rescue-capture" {
            let reference = slots.root.join("rescue-reference");
            anyhow::ensure!(!reference.exists(), "Rescue reference already exists");
            std::fs::create_dir_all(&reference)?;
            std::fs::write(reference.join("frame"), frame_bytes(&mut shared.frame.lock().unwrap()))?;
            checkpoint::write_tree(&reference.join("expected"), &checkpoint::tree(&slots.save)?)?;
        }
        let capture = cfg!(feature = "rescue-replay") && name == "rescue-capture";
        anyhow::ensure!(
            capture || frame_bytes(&mut shared.frame.lock().unwrap()) == frame,
            "Checkpoint display mismatch; current session retained"
        );
        anyhow::ensure!(
            capture || checkpoint::tree(&slots.save)? == expected,
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
            let failed = !matches!(&result, Ok(Ok(())));
            let message = match result {
                Ok(Ok(())) => "Stopped".into(),
                Ok(Err(e)) => format!("Error: {e}"),
                Err(payload) => {
                    let message = payload
                        .downcast_ref::<String>()
                        .map(String::as_str)
                        .or_else(|| payload.downcast_ref::<&str>().copied())
                        .unwrap_or("unknown panic");
                    format!("Error: emulator stopped unexpectedly: {message}")
                }
            };
            if failed && !worker.stopping.load(std::sync::atomic::Ordering::Relaxed) {
                capture_rescue(&worker, &message);
            }
            *worker.status.lock().unwrap_or_else(|e| e.into_inner()) = message;
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
pub extern "system" fn Java_local_wie_nativeapp_NativeBridge_speed(_: JNIEnv, _: JClass, value: jint) {
    send(Command::Speed(value.clamp(250, 3000) as u32));
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
// Capture existing host-side evidence directly: a stuck guest may never service commands.
fn manual_rescue(shared: &Shared) -> anyhow::Result<PathBuf> {
    anyhow::ensure!(
        !shared.replaying.load(std::sync::atomic::Ordering::Relaxed),
        "Wait for Quick Load to finish before capturing a rescue"
    );
    let state = shared
        .rescue
        .try_lock()
        .map_err(|_| anyhow::anyhow!("Rescue capture is busy; try again"))?;
    let (rescue, slots) = state.as_ref().ok_or_else(|| anyhow::anyhow!("No session recording is available yet"))?;
    let frame = {
        let mut frame = shared.frame.try_lock().map_err(|_| anyhow::anyhow!("Frame capture is busy; try again"))?;
        frame_bytes(&mut frame)
    };
    rescue.capture_manual(&frame, slots)
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_local_wie_nativeapp_NativeBridge_manualRescue(mut env: JNIEnv, _: JClass) -> jstring {
    let result = shared()
        .ok_or_else(|| anyhow::anyhow!("No game session is available"))
        .and_then(|state| manual_rescue(&state));
    match result {
        Ok(path) => env.new_string(path.to_string_lossy()).map_or(std::ptr::null_mut(), |s| s.into_raw()),
        Err(error) => {
            let _ = env.throw_new("java/lang/IllegalStateException", error.to_string());
            std::ptr::null_mut()
        }
    }
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_local_wie_nativeapp_NativeBridge_textInputMode(_: JNIEnv, _: JClass, korean: jboolean) {
    send(Command::TextInputMode(korean != 0));
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
    f.materialize();
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

#[cfg(test)]
mod fast_forward_tests {
    use super::*;
    use wie_backend::canvas::{AbgrPixel, ArgbPixel, Rgb332Pixel, Rgb565Pixel, VecImageBuffer};
    fn compare(image: &dyn Image) {
        let normal = Arc::new(Shared::default());
        let fast = Arc::new(Shared::default());
        fast.fast_forward.store(true, std::sync::atomic::Ordering::Relaxed);
        Output(normal.clone()).paint(image);
        Output(fast.clone()).paint(image);
        assert_eq!(fast.frame.lock().unwrap().pending_pixels.is_some(), image.packed_pixel_format().is_some());
        assert_eq!(
            frame_bytes(&mut normal.frame.lock().unwrap()),
            frame_bytes(&mut fast.frame.lock().unwrap())
        );
    }
    #[test]
    fn pending_frame_matches_eager_checkpoint_pixels() {
        compare(&VecImageBuffer::<Rgb565Pixel>::from_raw(
            3,
            2,
            vec![0, 0xffff, 0xf800, 0x07e0, 0x001f, 0x1234],
        ));
        compare(&VecImageBuffer::<ArgbPixel>::from_raw(3, 1, vec![0x00123456, 0x80123456, 0xffffffff]));
        // Equal byte size does not imply equal channel order: ABGR must fall back.
        compare(&VecImageBuffer::<AbgrPixel>::from_raw(3, 1, vec![0x00123456, 0x80123456, 0xffffffff]));
        compare(&VecImageBuffer::<Rgb332Pixel>::from_raw(3, 1, vec![0x12, 0x34, 0xff]));
    }
    #[test]
    fn unpresented_frames_keep_latest_pixels_and_every_paint_count() {
        let shared = Arc::new(Shared::default());
        shared.fast_forward.store(true, std::sync::atomic::Ordering::Relaxed);
        let output = Output(shared.clone());
        for color in [0u32, 0x123456, 0xabcdef] {
            output.paint(&VecImageBuffer::<ArgbPixel>::from_raw(1, 1, vec![color]));
        }
        let mut frame = shared.frame.lock().unwrap();
        frame.materialize();
        assert_eq!(frame.paints, 3);
        assert_eq!(frame.pixels, vec![0xffabcdefu32 as i32]);
        drop(frame);
        output.resize(2, 1).unwrap();
        assert_eq!(shared.frame.lock().unwrap().pixels, vec![0xffabcdefu32 as i32, 0]);
        output.paint(&VecImageBuffer::<ArgbPixel>::from_raw(1, 1, vec![0x123456]));
        shared.fast_forward.store(false, std::sync::atomic::Ordering::Relaxed);
        output.paint(&VecImageBuffer::<ArgbPixel>::from_raw(1, 1, vec![0x654321]));
        let mut frame = shared.frame.lock().unwrap();
        frame.materialize();
        assert_eq!(frame.pixels, vec![0xff654321u32 as i32]);
        assert_eq!(frame.paints, 5);
    }
}

#[cfg(feature = "frame-diagnostics")]
fn diagnostic_log_path(save: &str) -> PathBuf {
    if cfg!(feature = "compatibility-audit") {
        PathBuf::from(save).with_extension("frames.log")
    } else {
        PathBuf::from(save).join("frames.log")
    }
}

#[cfg(all(test, feature = "compatibility-audit"))]
mod compatibility_audit_tests {
    #[test]
    fn live_log_survives_replay_restoring_historical_logs() {
        use std::io::Write;
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("gomul-audit-log-{stamp}"));
        let save = root.join("case");
        std::fs::create_dir_all(&save).unwrap();
        let log = super::diagnostic_log_path(save.to_str().unwrap());
        let mut writer = std::fs::File::create(&log).unwrap();
        writer.write_all(b"before replay\n").unwrap();
        std::fs::remove_dir_all(&save).unwrap();
        std::fs::create_dir_all(&save).unwrap();
        std::fs::write(save.join("frames.log"), b"historical exception").unwrap();
        writer.write_all(b"after replay\n").unwrap();
        assert_eq!(std::fs::read_to_string(log).unwrap(), "before replay\nafter replay\n");
        assert_eq!(std::fs::read_to_string(save.join("frames.log")).unwrap(), "historical exception");
        drop(writer);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod manual_rescue_tests {
    use super::*;
    #[test]
    fn capture_does_not_need_a_worker_or_command_reply() -> anyhow::Result<()> {
        let root = std::env::temp_dir().join(format!(
            "gomul-manual-worker-{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos()
        ));
        let save = root.join("saves/game");
        std::fs::create_dir_all(&save)?;
        let slots = checkpoint::Slots::new(save.to_str().unwrap())?;
        let tape = Arc::new(Mutex::new(checkpoint::Tape::default()));
        tape.lock().unwrap().tick();
        tape.lock().unwrap().mark_safe();
        let shared = Shared::default();
        assert!(manual_rescue(&shared).is_err());
        *shared.rescue.lock().unwrap() = Some((rescue::Rescue::new(&slots, tape, b"test"), slots));
        let report = manual_rescue(&shared)?;
        assert!(report.join("frame").is_file());
        assert!(report.join("safe-trace").is_file());
        assert!(shared.status.lock().unwrap().is_empty());
        let busy = shared.frame.lock().unwrap();
        assert!(manual_rescue(&shared).is_err());
        drop(busy);
        shared.replaying.store(true, std::sync::atomic::Ordering::Relaxed);
        assert!(manual_rescue(&shared).is_err());
        std::fs::remove_dir_all(root)?;
        Ok(())
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_local_wie_nativeapp_NativeBridge_isKoreanInput(_: JNIEnv, _: JClass) -> jboolean {
    shared().is_some_and(|s| s.korean_input.load(std::sync::atomic::Ordering::Relaxed)) as jboolean
}
