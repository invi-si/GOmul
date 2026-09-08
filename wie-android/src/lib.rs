//! Native Android adapter. Guest CPU, memory and WIPI services live on one Rust
//! worker; JNI carries input, complete frames and audio commands only.
mod audio;
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
    objects::{JClass, JIntArray, JString},
    sys::{jboolean, jbyteArray, jint, jlong, jstring},
};
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{
        Arc, Mutex, OnceLock,
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant as HostInstant, SystemTime, UNIX_EPOCH},
};
use wie_backend::{AudioCommand, AudioSink, Event, Filesystem, Font, Instant, KeyCode, Platform, Screen, canvas::Image};

#[derive(Default)]
struct Shared {
    frame: Mutex<Frame>,
    audio: Mutex<VecDeque<AudioCommand>>,
    status: Mutex<String>,
    exiting: std::sync::atomic::AtomicBool,
}
struct Frame {
    width: u32,
    height: u32,
    pixels: Vec<i32>,
    dirty: bool,
    redraw: bool,
    paints: u64,
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
        self.0.audio.lock().unwrap().push_back(c);
    }
}
struct AndroidPlatform {
    phone_number: Option<String>,
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
        Instant::from_epoch_millis(SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64)
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
    Key(KeyCode, bool),
    Pause(bool),
    Stop,
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
        output: Output(shared.clone()),
        font: Font::try_from_static(include_bytes!("../../assets/neodgm.ttf"))?,
        db: database::DatabaseRepository::at_path(PathBuf::from(&save)),
        fs: filesystem::CliFilesystem::at_path(PathBuf::from(save)),
    };
    let mut emulator = loader::load(&path, Box::new(platform))?;
    *shared.status.lock().unwrap() = "Running".into();
    let mut held = HashMap::new();
    let mut paused = false;
    loop {
        let command = rx.recv_timeout(Duration::from_millis(if paused { 100 } else { 1 }));
        let commands = command.into_iter().chain(rx.try_iter());
        for command in commands {
            match command {
                Command::Stop => return Ok(()),
                Command::Pause(value) => {
                    paused = value;
                    for (key, _) in held.drain() {
                        emulator.handle_event(Event::Keyup(key));
                    }
                }
                Command::Key(key, true) if !paused => {
                    if let std::collections::hash_map::Entry::Vacant(e) = held.entry(key) {
                        e.insert(HostInstant::now());
                        emulator.handle_event(Event::Keydown(key));
                    }
                }
                Command::Key(key, false) => {
                    if held.remove(&key).is_some() {
                        emulator.handle_event(Event::Keyup(key));
                    }
                }
                _ => {}
            }
        }
        if paused {
            continue;
        }
        for (&key, time) in &mut held {
            if time.elapsed() > Duration::from_millis(100) {
                *time = HostInstant::now();
                emulator.handle_event(Event::Keyrepeat(key));
            }
        }
        let redraw = {
            let mut f = shared.frame.lock().unwrap();
            std::mem::take(&mut f.redraw)
        };
        if redraw {
            emulator.handle_event(Event::Redraw);
        }
        emulator.tick()?;
        #[cfg(feature = "cpu-replay")]
        cpu_capture.poll();
        if shared.exiting.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(());
        }
    }
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
pub extern "system" fn Java_local_wie_nativeapp_NativeBridge_key(mut env: JNIEnv, _: JClass, key: JString, down: jboolean) {
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
            send(Command::Key(code, down != 0));
        }
    }
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_local_wie_nativeapp_NativeBridge_frame(mut env: JNIEnv, _: JClass, out: JIntArray) -> jlong {
    let Some(s) = shared() else {
        return 0;
    };
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
