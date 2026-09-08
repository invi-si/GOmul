//! Headless diagnostic runner using the desktop emulator's persistent save adapters.
//! Example: cargo run --release --example probe_lgt -- game.zip output --seconds 30 --keys '5:OK,8:OK'
//! While running, send key names such as OK or DOWN on stdin, one per line.

#[path = "../src/database.rs"]
mod database;
#[path = "../src/filesystem.rs"]
mod filesystem;

use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufRead, BufWriter, LineWriter, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant as WallInstant, SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use wie_backend::{
    AudioCommand, AudioSink, Emulator, Event, Filesystem, Font, Instant, KeyCode, Options, Platform, ProfileCallback, ProfileSample, Screen,
    canvas::Image, extract_zip,
};
use wie_ktf::KtfEmulator;
use wie_lgt::LgtEmulator;

#[derive(Parser)]
struct Args {
    /// LGT/KTF game directory or archive ZIP, or an LGT application JAR.
    game: PathBuf,
    /// Directory receiving PPM snapshots and stats.txt.
    output: PathBuf,
    #[arg(long, default_value_t = 30.0)]
    seconds: f64,
    /// Comma-separated seconds:key[:hold_seconds], e.g. 5:OK,8:DOWN:1,10:LEFT_SOFT_KEY.
    #[arg(long, default_value = "")]
    keys: String,
    /// Write ARM sampling stacks in flamegraph folded format.
    #[arg(long)]
    profile_out: Option<PathBuf>,
    /// Separate save directory for diagnostic runs.
    #[arg(long)]
    data_root: Option<PathBuf>,
}

struct State {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    redraw: bool,
    exited: bool,
    paints: u64,
    changed_frames: u64,
    audio_plays: u64,
    audio_stops: u64,
    frame_metrics: Vec<(u128, usize)>,
}

#[derive(Clone)]
struct ProbeScreen(Arc<Mutex<State>>);

impl Screen for ProbeScreen {
    fn resize(&self, width: u32, height: u32) -> wie_util::Result<()> {
        if width == 0 || height == 0 || width > 4096 || height > 4096 {
            return Err(wie_util::WieError::FatalError(format!("Invalid display size: {width}x{height}")));
        }
        let mut state = self.0.lock().unwrap();
        state.width = width;
        state.height = height;
        state.pixels = vec![0; width as usize * height as usize * 3];
        eprintln!("screen resized to {width}x{height}");
        Ok(())
    }

    fn request_redraw(&self) -> wie_util::Result<()> {
        self.0.lock().unwrap().redraw = true;
        Ok(())
    }

    fn paint(&self, image: &dyn Image) {
        let pixels: Vec<u8> = image.colors().iter().flat_map(|color| [color.r, color.g, color.b]).collect();
        let mut state = self.0.lock().unwrap();
        state.paints += 1;
        let white_pixels = pixels.chunks_exact(3).filter(|pixel| *pixel == [255, 255, 255]).count();
        state
            .frame_metrics
            .push((SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros(), white_pixels));
        if state.pixels != pixels {
            state.changed_frames += 1;
        }
        state.width = image.width();
        state.height = image.height();
        state.pixels = pixels;
    }

    fn width(&self) -> u32 {
        self.0.lock().unwrap().width
    }

    fn height(&self) -> u32 {
        self.0.lock().unwrap().height
    }
}

impl AudioSink for ProbeScreen {
    fn send(&self, command: AudioCommand) {
        let mut state = self.0.lock().unwrap();
        match command {
            AudioCommand::Play { handle, sequence, repeat } => {
                state.audio_plays += 1;
                eprintln!(
                    "audio play handle={handle} duration_ms={} events={} repeat={repeat}",
                    sequence.duration,
                    sequence.events.len()
                );
            }
            AudioCommand::Stop { handle } => {
                state.audio_stops += 1;
                eprintln!("audio stop handle={handle}");
            }
        }
    }
}

struct ProbePlatform {
    screen: ProbeScreen,
    font: Font,
    filesystem: filesystem::CliFilesystem,
    database: database::DatabaseRepository,
}

impl Platform for ProbePlatform {
    fn font(&self) -> &Font {
        &self.font
    }
    fn screen(&self) -> &dyn Screen {
        &self.screen
    }
    fn now(&self) -> Instant {
        Instant::from_epoch_millis(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64)
    }
    fn database_repository(&self) -> &dyn wie_backend::DatabaseRepository {
        &self.database
    }
    fn filesystem(&self) -> &dyn Filesystem {
        &self.filesystem
    }
    fn audio_sink(&self) -> Box<dyn AudioSink> {
        Box::new(self.screen.clone())
    }
    fn write_stdout(&self, buf: &[u8]) {
        print!("{}", String::from_utf8_lossy(buf));
    }
    fn write_stderr(&self, buf: &[u8]) {
        eprint!("{}", String::from_utf8_lossy(buf));
    }
    fn exit(&self) {
        self.screen.0.lock().unwrap().exited = true;
    }
    fn vibrate(&self, duration_ms: u64, intensity: u8) {
        eprintln!("vibrate duration_ms={duration_ms} intensity={intensity}");
    }
}

fn read_directory(base: &Path, path: &Path, files: &mut BTreeMap<String, Vec<u8>>) -> anyhow::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            read_directory(base, &entry.path(), files)?;
        } else if entry.file_type()?.is_file() {
            files.insert(
                entry.path().strip_prefix(base)?.to_string_lossy().replace('\\', "/"),
                fs::read(entry.path())?,
            );
        }
    }
    Ok(())
}

fn keycode(text: &str) -> anyhow::Result<KeyCode> {
    Ok(match text.to_ascii_uppercase().as_str() {
        "UP" => KeyCode::UP,
        "DOWN" => KeyCode::DOWN,
        "LEFT" => KeyCode::LEFT,
        "RIGHT" => KeyCode::RIGHT,
        "OK" => KeyCode::OK,
        "LEFT_SOFT_KEY" => KeyCode::LEFT_SOFT_KEY,
        "RIGHT_SOFT_KEY" => KeyCode::RIGHT_SOFT_KEY,
        "CLEAR" | "CLR" => KeyCode::CLEAR,
        "0" => KeyCode::NUM0,
        "1" => KeyCode::NUM1,
        "2" => KeyCode::NUM2,
        "3" => KeyCode::NUM3,
        "4" => KeyCode::NUM4,
        "5" => KeyCode::NUM5,
        "6" => KeyCode::NUM6,
        "7" => KeyCode::NUM7,
        "8" => KeyCode::NUM8,
        "9" => KeyCode::NUM9,
        "#" => KeyCode::HASH,
        "*" => KeyCode::STAR,
        _ => anyhow::bail!("Unknown key: {text}"),
    })
}

fn key_events(script: &str) -> anyhow::Result<Vec<(Duration, Event)>> {
    let mut events = Vec::new();
    for item in script.split(',').filter(|item| !item.is_empty()) {
        let parts: Vec<_> = item.split(':').collect();
        anyhow::ensure!((2..=3).contains(&parts.len()), "Expected seconds:key[:hold_seconds], got {item}");
        let due = Duration::try_from_secs_f64(parts[0].parse()?)?;
        let hold = Duration::try_from_secs_f64(parts.get(2).map_or(Ok(0.15), |text| text.parse())?)?;
        let key = keycode(parts[1])?;
        events.push((due, Event::Keydown(key)));
        let mut repeat = Duration::from_millis(100);
        while repeat < hold {
            events.push((due + repeat, Event::Keyrepeat(key)));
            repeat += Duration::from_millis(100);
        }
        events.push((due + hold, Event::Keyup(key)));
    }
    events.sort_by_key(|(due, _)| *due);
    Ok(events)
}

fn snapshot(output: &Path, name: &str, state: &State) -> anyhow::Result<()> {
    let mut writer = BufWriter::new(File::create(output.join(format!("{name}.ppm")))?);
    writeln!(writer, "P6\n{} {}\n255", state.width, state.height)?;
    writer.write_all(&state.pixels)?;
    writer.flush()?;
    Ok(())
}

fn profile_callback(path: &Path) -> anyhow::Result<ProfileCallback> {
    let writer = Mutex::new(LineWriter::new(File::create(path)?));
    Ok(Box::new(move |batch: Vec<ProfileSample>| {
        let mut writer = writer.lock().unwrap();
        for sample in batch {
            let stack: Vec<String> = sample.stack.iter().rev().map(|pc| format!("0x{pc:x}")).collect();
            if let Err(error) = writeln!(writer, "{} {}", stack.join(";"), sample.count) {
                tracing::error!("Failed to write ARM profile: {error}");
                break;
            }
        }
    }))
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let args = Args::parse();
    let duration = Duration::try_from_secs_f64(args.seconds)?;
    let mut events = key_events(&args.keys)?.into_iter().peekable();
    fs::create_dir_all(&args.output)?;
    let screen = ProbeScreen(Arc::new(Mutex::new(State {
        width: 240,
        height: 320,
        pixels: vec![0; 240 * 320 * 3],
        redraw: true,
        exited: false,
        paints: 0,
        changed_frames: 0,
        audio_plays: 0,
        audio_stops: 0,
        frame_metrics: Vec::new(),
    })));
    let platform = Box::new(ProbePlatform {
        screen: screen.clone(),
        font: Font::try_from_static(include_bytes!("../assets/neodgm.ttf"))?,
        filesystem: args.data_root.clone().map(filesystem::CliFilesystem::at_path).unwrap_or_default(),
        database: args
            .data_root
            .clone()
            .map(database::DatabaseRepository::at_path)
            .unwrap_or_else(database::DatabaseRepository::new),
    });
    let options = Options {
        enable_gdbserver: false,
        profile: args.profile_out.as_deref().map(profile_callback).transpose()?,
    };
    let mut emulator: Box<dyn Emulator> = if args.game.is_dir() {
        let mut files = BTreeMap::new();
        read_directory(&args.game, &args.game, &mut files)?;
        if KtfEmulator::loadable_archive(&files) {
            Box::new(KtfEmulator::from_archive(platform, files, options)?)
        } else {
            Box::new(LgtEmulator::from_archive(platform, files, options)?)
        }
    } else if args.game.extension().is_some_and(|extension| extension == "jar") {
        let filename = args.game.file_name().unwrap().to_string_lossy();
        let id = args.game.file_stem().unwrap().to_string_lossy();
        Box::new(LgtEmulator::from_jar(
            platform,
            &filename,
            fs::read(&args.game)?,
            &id,
            &id,
            None,
            options,
        )?)
    } else {
        let files = extract_zip(&fs::read(&args.game)?)?;
        if KtfEmulator::loadable_archive(&files) {
            Box::new(KtfEmulator::from_archive(platform, files, options)?)
        } else {
            Box::new(LgtEmulator::from_archive(platform, files, options)?)
        }
    };
    let (input_tx, input_rx) = mpsc::channel();
    thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            match line {
                Ok(line) => {
                    if input_tx.send(line).is_err() {
                        break;
                    }
                }
                Err(error) => {
                    eprintln!("stdin read failed: {error}");
                    break;
                }
            }
        }
    });
    let started = WallInstant::now();
    let mut pending_keyups: Vec<(Duration, KeyCode)> = Vec::new();
    let mut next_snapshot = Duration::ZERO;
    let mut ticks = 0_u64;
    let mut tick_time = Duration::ZERO;
    let mut longest_tick = Duration::ZERO;
    let mut failure = None;
    while started.elapsed() < duration && !screen.0.lock().unwrap().exited {
        let elapsed = started.elapsed();
        for line in input_rx.try_iter() {
            let text = line.trim();
            if text.is_empty() {
                continue;
            }
            match keycode(text) {
                Ok(key) => {
                    eprintln!("stdin input at {:.3}s key={key:?}", elapsed.as_secs_f64());
                    emulator.handle_event(Event::Keydown(key));
                    pending_keyups.push((elapsed + Duration::from_millis(150), key));
                }
                Err(error) => eprintln!("stdin input ignored: {error}"),
            }
        }
        pending_keyups.retain(|(due, key)| {
            if *due <= elapsed {
                emulator.handle_event(Event::Keyup(*key));
                false
            } else {
                true
            }
        });
        while events.peek().is_some_and(|(due, _)| *due <= elapsed) {
            let (due, event) = events.next().unwrap();
            eprintln!("input at {:.3}s (scheduled {:.3}s)", elapsed.as_secs_f64(), due.as_secs_f64());
            emulator.handle_event(event);
        }
        if std::mem::take(&mut screen.0.lock().unwrap().redraw) {
            emulator.handle_event(Event::Redraw);
        }
        let tick_start = WallInstant::now();
        let result = emulator.tick();
        let spent = tick_start.elapsed();
        ticks += 1;
        tick_time += spent;
        longest_tick = longest_tick.max(spent);
        if let Err(error) = result {
            failure = Some(error.to_string());
            break;
        }
        if started.elapsed() >= next_snapshot {
            let state = screen.0.lock().unwrap();
            snapshot(&args.output, &format!("frame-{:06}", started.elapsed().as_millis()), &state)?;
            eprintln!(
                "elapsed={:.3}s paints={} changed={} size={}x{}",
                started.elapsed().as_secs_f64(),
                state.paints,
                state.changed_frames,
                state.width,
                state.height
            );
            next_snapshot = started.elapsed() + Duration::from_secs(1);
        }
        thread::sleep(Duration::from_millis(1));
    }
    let elapsed = started.elapsed().as_secs_f64();
    let state = screen.0.lock().unwrap();
    snapshot(&args.output, "final", &state)?;
    let stats = format!(
        "wall_seconds={elapsed:.3}\nticks={ticks}\ntick_seconds={:.3}\nmax_tick_ms={:.3}\npaint_calls={}\nchanged_frames={}\npaint_calls_per_second={:.3}\nchanged_frames_per_second={:.3}\naudio_plays={}\naudio_stops={}\nguest_exited={}\nstatus={}\n",
        tick_time.as_secs_f64(),
        longest_tick.as_secs_f64() * 1000.0,
        state.paints,
        state.changed_frames,
        state.paints as f64 / elapsed,
        state.changed_frames as f64 / elapsed,
        state.audio_plays,
        state.audio_stops,
        state.exited,
        failure.as_deref().unwrap_or("ok"),
    );
    let mut metrics = BufWriter::new(File::create(args.output.join("frames.csv"))?);
    writeln!(metrics, "frame,epoch_us,white_pixels")?;
    for (index, (time, white)) in state.frame_metrics.iter().enumerate() {
        writeln!(metrics, "{index},{time},{white}")?;
    }
    fs::write(args.output.join("stats.txt"), &stats)?;
    eprintln!("{stats}");
    if let Some(error) = failure {
        anyhow::bail!(error);
    }
    Ok(())
}
