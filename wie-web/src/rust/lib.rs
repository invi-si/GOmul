#![no_std]
extern crate alloc;

mod audio_sink;
#[cfg(feature = "cpu-profiling")]
mod cpu_profile;
mod database;
mod filesystem;
mod indexed_db_store;
mod util;
mod window;

use alloc::{
    borrow::ToOwned,
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::{
    str,
    sync::atomic::{AtomicBool, Ordering},
};

use hashbrown::HashMap;
use tracing_subscriber::{Layer, filter::LevelFilter, fmt::time::UtcTime, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_web::MakeConsoleWriter;
use wasm_bindgen::{JsError, prelude::*};
use web_sys::HtmlCanvasElement;

use wie_backend::{Emulator, Event, Font, Instant, KeyCode, Options, Platform, Screen, extract_zip};
use wie_j2me::J2MEEmulator;
use wie_ktf::KtfEmulator;
use wie_lgt::LgtEmulator;
use wie_skt::SktEmulator;

use self::{
    audio_sink::{AudioPlayer, AudioSink},
    database::DatabaseRepository,
    filesystem::WebFilesystem,
    window::WindowImpl,
};

enum ArchivePlatform {
    Ktf,
    Lgt,
    Skt,
}

fn parse_archive(buf: &[u8]) -> anyhow::Result<(ArchivePlatform, BTreeMap<String, Vec<u8>>)> {
    let files = extract_zip(buf)?;

    if !files.keys().any(|name| name.to_ascii_lowercase().ends_with(".jar")) {
        anyhow::bail!("Archive does not contain a JAR file");
    }

    let platform = if KtfEmulator::loadable_archive(&files) {
        ArchivePlatform::Ktf
    } else if LgtEmulator::loadable_archive(&files) {
        ArchivePlatform::Lgt
    } else if SktEmulator::loadable_archive(&files) {
        ArchivePlatform::Skt
    } else {
        anyhow::bail!("Unknown archive format");
    };

    Ok((platform, files))
}

fn jar_app_id<'a>(filename: &'a str, buf: &[u8]) -> &'a str {
    if KtfEmulator::loadable_jar(buf) || LgtEmulator::loadable_jar(buf) || SktEmulator::loadable_jar(buf) {
        &filename[..filename.len() - 4]
    } else {
        filename
    }
}

struct WieWebPlatform {
    audio_player: AudioPlayer,
    database_repository: DatabaseRepository,
    filesystem: WebFilesystem,
    font: Font,
    window: WindowImpl,
    exited: Arc<AtomicBool>,
}

// XXX we're on single thread
unsafe impl Sync for WieWebPlatform {}
unsafe impl Send for WieWebPlatform {}

impl WieWebPlatform {
    fn new(window: WindowImpl, font: Font, audio_player: AudioPlayer, exited: Arc<AtomicBool>) -> Self {
        Self {
            audio_player,
            database_repository: DatabaseRepository::new(),
            filesystem: WebFilesystem::new(),
            font,
            window,
            exited,
        }
    }
}

impl Platform for WieWebPlatform {
    fn font(&self) -> &Font {
        &self.font
    }

    fn screen(&self) -> &dyn Screen {
        &self.window
    }

    fn now(&self) -> Instant {
        let millis = js_sys::Date::now();

        Instant::from_epoch_millis(millis as _)
    }

    fn database_repository(&self) -> &dyn wie_backend::DatabaseRepository {
        &self.database_repository
    }

    fn filesystem(&self) -> &dyn wie_backend::Filesystem {
        &self.filesystem
    }

    fn audio_sink(&self) -> Box<dyn wie_backend::AudioSink> {
        Box::new(AudioSink::new(self.audio_player.clone()))
    }

    fn write_stdout(&self, data: &[u8]) {
        let string = str::from_utf8(data).unwrap();
        tracing::info!("{}", string);
    }

    fn write_stderr(&self, data: &[u8]) {
        let string = str::from_utf8(data).unwrap();
        tracing::info!("{}", string);
    }

    fn exit(&self) {
        self.exited.store(true, Ordering::SeqCst);
    }

    fn vibrate(&self, duration_ms: u64, intensity: u8) {
        if duration_ms == 0 || intensity == 0 {
            return;
        }

        let Some(window) = web_sys::window() else { return };
        let navigator = window.navigator();
        if !js_sys::Reflect::has(navigator.as_ref(), &JsValue::from_str("vibrate")).unwrap_or(false) {
            return;
        }
        let duration = core::cmp::min(duration_ms, u32::MAX as u64) as u32;
        navigator.vibrate_with_duration(duration);
    }
}

#[wasm_bindgen]
pub struct WieWeb {
    emulator: Box<dyn Emulator>,
    audio_player: AudioPlayer,
    should_redraw: Arc<AtomicBool>,
    exited: Arc<AtomicBool>,
    key_events: HashMap<KeyCode, f64>,
    #[cfg(feature = "cpu-profiling")]
    cpu_profile: cpu_profile::WebCpuProfile,
    #[cfg(feature = "cpu-throughput")]
    cpu_throughput_core: Option<wie_core_arm::ArmCore>,
}

impl Drop for WieWeb {
    fn drop(&mut self) {
        // Runtime tasks can retain platform references after the view closes.
        self.audio_player.dispose();
    }
}

#[wasm_bindgen]
pub struct ImportedAppMetadata {
    id: String,
    title: String,
    icon: Vec<u8>,
}

#[wasm_bindgen]
impl ImportedAppMetadata {
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn title(&self) -> String {
        self.title.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn icon(&self) -> Vec<u8> {
        self.icon.clone()
    }
}

#[wasm_bindgen(js_name = extractAppMetadata)]
pub fn extract_app_metadata(filename: &str, buf: &[u8]) -> Result<ImportedAppMetadata, JsError> {
    read_app_metadata(filename, buf).map_err(|error| JsError::new(&error.to_string()))
}

fn read_app_metadata(filename: &str, buf: &[u8]) -> anyhow::Result<ImportedAppMetadata> {
    let lowercase_filename = filename.to_ascii_lowercase();
    let metadata = if lowercase_filename.ends_with(".zip") {
        let (platform, files) = parse_archive(buf)?;
        match platform {
            ArchivePlatform::Ktf => KtfEmulator::archive_id(&files)
                .zip(KtfEmulator::archive_title(&files))
                .map(|(id, title)| (id, title, KtfEmulator::archive_icon(&files))),
            ArchivePlatform::Lgt => LgtEmulator::archive_id(&files)
                .zip(LgtEmulator::archive_title(&files))
                .map(|(id, title)| (id, title, LgtEmulator::archive_icon(&files))),
            ArchivePlatform::Skt => SktEmulator::archive_id(&files)
                .zip(SktEmulator::archive_title(&files))
                .map(|(id, title)| (id, title, SktEmulator::archive_icon(&files))),
        }
    } else if lowercase_filename.ends_with(".jar") {
        let filename = filename.rsplit('/').next().unwrap();
        let metadata = J2MEEmulator::jar_metadata(buf)?.map(|(title, icon)| (jar_app_id(filename, buf).to_owned(), title, icon));

        // Native WIPI applications can be ZIP-backed JARs without a Java manifest.
        // Standalone imports use the same filename-based identity as from_jar below;
        // a carrier archive retains its original metadata and save identity instead.
        metadata.or_else(|| {
            (KtfEmulator::loadable_jar(buf) || LgtEmulator::loadable_jar(buf)).then(|| {
                let id = filename[..filename.len() - 4].to_owned();
                (id.clone(), id, None)
            })
        })
    } else {
        anyhow::bail!("Unknown file format");
    };
    let (id, title, icon) = metadata.ok_or_else(|| anyhow::anyhow!("App metadata does not contain an ID, title or entry point"))?;

    Ok(ImportedAppMetadata {
        id,
        title,
        icon: icon.unwrap_or_default(),
    })
}

#[wasm_bindgen]
impl WieWeb {
    #[wasm_bindgen(constructor)]
    pub fn new(filename: &str, buf: &[u8], canvas: HtmlCanvasElement, font_data: Vec<u8>) -> Result<WieWeb, JsError> {
        let audio_player = AudioPlayer::new();
        let result = (|| {
            let should_redraw = Arc::new(AtomicBool::new(true));
            let exited = Arc::new(AtomicBool::new(false));
            let window = WindowImpl::new(canvas, should_redraw.clone());
            let font = Font::try_from_vec(font_data)?;
            let platform = Box::new(WieWebPlatform::new(window, font, audio_player.clone(), exited.clone()));
            let options = Options {
                enable_gdbserver: false,
                profile: None,
            };

            #[cfg(any(feature = "cpu-profiling", feature = "cpu-throughput"))]
            let mut cpu_profile_core = None;
            let emulator: Box<dyn Emulator> = if filename.to_ascii_lowercase().ends_with(".zip") {
                let (archive_platform, files) = parse_archive(buf)?;

                match archive_platform {
                    ArchivePlatform::Ktf => Box::new(KtfEmulator::from_archive(platform, files, options)?),
                    ArchivePlatform::Lgt => {
                        let emulator = LgtEmulator::from_archive(platform, files, options)?;
                        #[cfg(any(feature = "cpu-profiling", feature = "cpu-throughput"))]
                        {
                            cpu_profile_core = Some(emulator.core_for_profiling());
                        }
                        Box::new(emulator)
                    }
                    ArchivePlatform::Skt => Box::new(SktEmulator::from_archive(platform, files)?),
                }
            } else if filename.to_ascii_lowercase().ends_with(".jar") {
                let filename_without_path = filename.rsplit('/').next().unwrap().to_owned();
                let app_id = jar_app_id(&filename_without_path, buf);

                if KtfEmulator::loadable_jar(buf) {
                    Box::new(KtfEmulator::from_jar(
                        platform,
                        &filename_without_path,
                        buf.to_vec(),
                        app_id,
                        app_id,
                        None,
                        options,
                    )?)
                } else if LgtEmulator::loadable_jar(buf) {
                    let emulator = LgtEmulator::from_jar(platform, &filename_without_path, buf.to_vec(), app_id, app_id, None, options)?;
                    #[cfg(any(feature = "cpu-profiling", feature = "cpu-throughput"))]
                    {
                        cpu_profile_core = Some(emulator.core_for_profiling());
                    }
                    Box::new(emulator)
                } else if SktEmulator::loadable_jar(buf) {
                    Box::new(SktEmulator::from_jar(platform, &filename_without_path, buf.to_vec(), app_id, None)?)
                } else {
                    Box::new(J2MEEmulator::from_jar(platform, &filename_without_path, buf.to_vec())?)
                }
            } else {
                anyhow::bail!("Unknown file format");
            };

            anyhow::Ok(Self {
                emulator,
                audio_player: audio_player.clone(),
                should_redraw,
                exited,
                key_events: HashMap::new(),
                #[cfg(feature = "cpu-profiling")]
                cpu_profile: cpu_profile::WebCpuProfile::new(cpu_profile_core.clone()),
                #[cfg(feature = "cpu-throughput")]
                cpu_throughput_core: cpu_profile_core,
            })
        })();
        if result.is_err() {
            audio_player.dispose();
        }
        result.map_err(|e| JsError::new(&e.to_string()))
    }

    pub fn update(&mut self) -> Result<(), JsError> {
        if self.has_exited() {
            return Ok(());
        }
        #[cfg(feature = "cpu-profiling")]
        let profile_events_token = self.cpu_profile.begin_events();
        if self.should_redraw.load(Ordering::SeqCst) {
            #[cfg(feature = "cpu-profiling")]
            {
                self.cpu_profile.redraw_events += u64::from(self.cpu_profile.enabled());
            }
            self.emulator.handle_event(Event::Redraw);
            self.should_redraw.store(false, Ordering::SeqCst)
        }

        let millis = js_sys::Date::now();

        for (key, key_millis) in self.key_events.iter_mut() {
            if millis - *key_millis > 100.0 {
                #[cfg(feature = "cpu-profiling")]
                {
                    self.cpu_profile.key_repeat_events += u64::from(self.cpu_profile.enabled());
                }
                self.emulator.handle_event(Event::Keyrepeat(*key));
                *key_millis = millis;
            }
        }

        #[cfg(feature = "cpu-profiling")]
        self.cpu_profile.end_events(profile_events_token);
        self.emulator.tick().map_err(|e| JsError::new(&e.to_string()))
    }

    pub fn has_exited(&self) -> bool {
        self.exited.load(Ordering::SeqCst)
    }

    pub fn key_down(&mut self, key: String) -> Result<(), JsError> {
        let millis = js_sys::Date::now();
        let key = KeyCode::parse(&key);

        #[cfg(feature = "cpu-profiling")]
        {
            self.cpu_profile.key_down_events += u64::from(self.cpu_profile.enabled());
        }
        self.emulator.handle_event(Event::Keydown(key));
        self.key_events.insert(key, millis);

        Ok(())
    }

    pub fn key_up(&mut self, key: String) -> Result<(), JsError> {
        let key = KeyCode::parse(&key);

        #[cfg(feature = "cpu-profiling")]
        {
            self.cpu_profile.key_up_events += u64::from(self.cpu_profile.enabled());
        }
        self.emulator.handle_event(Event::Keyup(key));
        self.key_events.remove(&key);

        Ok(())
    }

    pub fn set_pcm_volume(&self, volume: f32) {
        audio_sink::set_pcm_volume(volume);
    }

    #[cfg(feature = "cpu-profiling")]
    pub fn profile_configure(&mut self, mode: &str, mean_interval: u32, seed: u32) -> Result<(), JsError> {
        self.cpu_profile.configure(mode, mean_interval, seed)
    }

    #[cfg(feature = "cpu-profiling")]
    pub fn profile_reset(&mut self) -> Result<(), JsError> {
        self.cpu_profile.reset()
    }

    #[cfg(feature = "cpu-profiling")]
    pub fn profile_snapshot(&self) -> Result<JsValue, JsError> {
        self.cpu_profile.snapshot()
    }

    #[cfg(feature = "cpu-throughput")]
    pub fn throughput_reset(&self) -> Result<(), JsError> {
        self.cpu_throughput_core
            .as_ref()
            .ok_or_else(|| JsError::new("CPU throughput counting is available for LGT ARM games in this diagnostic build"))?
            .reset_cpu_throughput()
            .map_err(|e| JsError::new(&e.to_string()))
    }

    #[cfg(feature = "cpu-throughput")]
    pub fn throughput_snapshot(&self) -> Result<JsValue, JsError> {
        let snapshot = self
            .cpu_throughput_core
            .as_ref()
            .ok_or_else(|| JsError::new("CPU throughput counting is unavailable for this platform"))?
            .cpu_throughput_snapshot()
            .map_err(|e| JsError::new(&e.to_string()))?;
        let json = serde_json::to_string(&snapshot).map_err(|e| JsError::new(&e.to_string()))?;
        js_sys::JSON::parse(&json).map_err(|_| JsError::new("Could not serialize CPU throughput counters"))
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_timer(UtcTime::rfc_3339())
        .with_writer(MakeConsoleWriter)
        .with_filter(LevelFilter::INFO);

    tracing_subscriber::registry().with(fmt_layer).init();
}

#[cfg(test)]
mod tests {
    use super::read_app_metadata;

    // Synthetic stored ZIP entries: no game code or assets.
    const NATIVE_JAR: &[u8] = b"PK\x03\x04\x14\x00\x00\x00\x00\x00\x00\x00!\x00Q\xc4:\xa7\x04\x00\
        \x00\x00\x04\x00\x00\x00\x0a\x00\x00\x00binary.mod\
        \x7fELFPK\x01\x02\x14\x03\x14\x00\x00\x00\x00\x00\x00\x00!\x00\
        Q\xc4:\xa7\x04\x00\x00\x00\x04\x00\x00\x00\x0a\x00\x00\x00\x00\x00\x00\x00\
        \x00\x00\x00\x00\x80\x01\x00\x00\x00\x00binary.mod\
        PK\x05\x06\x00\x00\x00\x00\x01\x00\x01\x008\x00\x00\x00,\x00\x00\x00\
        \x00\x00";
    const JAVA_JAR: &[u8] = b"PK\x03\x04\x14\x00\x00\x00\x00\x00\x00\x00!\x00#U\xcfe?\x00\
        \x00\x00?\x00\x00\x00\x14\x00\x00\x00META-INF/M\
        ANIFEST.MFMIDlet-1: \
        Example Game,,exampl\
        e.Main\x0aMIDlet-Name: \
        Example Game\x0aPK\x01\x02\x14\x03\x14\
        \x00\x00\x00\x00\x00\x00\x00!\x00#U\xcfe?\x00\x00\x00?\x00\x00\
        \x00\x14\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x80\x01\x00\x00\x00\x00M\
        ETA-INF/MANIFEST.MFP\
        K\x05\x06\x00\x00\x00\x00\x01\x00\x01\x00B\x00\x00\x00q\x00\x00\x00\x00\
        \x00";

    #[test]
    fn imports_native_jar_without_a_java_manifest() {
        let metadata = read_app_metadata("folder/Example.JAR", NATIVE_JAR).unwrap();
        assert_eq!(metadata.id, "Example");
        assert_eq!(metadata.title, "Example");
        assert!(metadata.icon.is_empty());
    }

    #[test]
    fn preserves_java_manifest_title_and_identity() {
        let metadata = read_app_metadata("folder/application.jar", JAVA_JAR).unwrap();
        assert_eq!(metadata.id, "application.jar");
        assert_eq!(metadata.title, "Example Game");
    }

    #[test]
    fn rejects_non_application_archives_with_a_jar_extension() {
        let mut archive = NATIVE_JAR.to_vec();
        // Rename the entry in both ZIP headers while preserving its data and CRC.
        for offset in [30, 90] {
            assert_eq!(&archive[offset..offset + 10], b"binary.mod");
            archive[offset..offset + 10].copy_from_slice(b"readme.txt");
        }
        assert!(read_app_metadata("unrelated.jar", &archive).is_err());
    }
}
