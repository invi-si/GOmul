//! Own-process AArch64 PC samples from SIGPROF. No guest instruction hooks.
//! Build with only `pc-sampling`; use the same executable for sampled=0 and 1.

#[cfg(all(target_os = "android", target_arch = "aarch64"))]
mod android {
    use std::{
        collections::BTreeMap,
        env,
        ffi::CStr,
        io, mem, ptr,
        sync::atomic::{AtomicBool, AtomicUsize, Ordering},
        time::Instant,
    };

    use wie_cpu_bench::{Benchmark, INSTRUCTIONS_PER_BATCH, Workload};

    // Verified in Android NDK 29 sysroot/usr/include/linux/time.h. The Rust
    // libc Android bindings expose setitimer but omit this selector constant.
    const ITIMER_PROF: libc::c_int = 2;
    // libc 0.2.189's Android AArch64 ucontext_t omits the kernel's sigmask
    // padding. Use the NDK sys/ucontext.h layout explicitly; otherwise its
    // uc_mcontext.pc reads x16, producing plausible code pointers and masks.
    // android_pc_layout.c independently checks these offsets against the NDK.
    #[repr(C)]
    struct AndroidUcontext {
        flags: libc::c_ulong,
        link: *mut libc::c_void,
        stack: libc::stack_t,
        sigmask: libc::sigset_t,
        sigmask_padding: [u8; 128 - mem::size_of::<libc::sigset_t>()],
        mcontext: libc::mcontext_t,
    }
    const CONTEXT_PC_OFFSET: usize = mem::offset_of!(AndroidUcontext, mcontext) + mem::offset_of!(libc::mcontext_t, pc);
    const _: () = assert!(mem::offset_of!(AndroidUcontext, mcontext) == 176);
    const _: () = assert!(mem::offset_of!(libc::mcontext_t, pc) == 264);
    const _: () = assert!(CONTEXT_PC_OFFSET == 440);
    const CAPACITY: usize = 131_072;
    static SAMPLES: [AtomicUsize; CAPACITY] = [const { AtomicUsize::new(0) }; CAPACITY];
    static RECORDING: AtomicBool = AtomicBool::new(false);
    static DELIVERED: AtomicUsize = AtomicUsize::new(0);
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    static DROPPED: AtomicUsize = AtomicUsize::new(0);
    static INVALID_CONTEXT: AtomicUsize = AtomicUsize::new(0);
    static OUTSIDE_WINDOW: AtomicUsize = AtomicUsize::new(0);

    // AArch64 usize atomics are lock-free. No allocation, locks, clocks,
    // formatting, symbol lookup or libc calls are performed in this handler.
    extern "C" fn handler(signal: libc::c_int, _: *mut libc::siginfo_t, context: *mut libc::c_void) {
        if signal != libc::SIGPROF {
            return;
        }
        DELIVERED.fetch_add(1, Ordering::Relaxed);
        if !RECORDING.load(Ordering::Acquire) {
            OUTSIDE_WINDOW.fetch_add(1, Ordering::Relaxed);
            return;
        }
        if context.is_null() {
            INVALID_CONTEXT.fetch_add(1, Ordering::Relaxed);
            return;
        }
        // The kernel supplies the interrupted thread's correctly aligned
        // ucontext for SA_SIGINFO; read its saved native PC, never modify it.
        let pc = unsafe { (*(context as *const AndroidUcontext)).mcontext.pc } as usize;
        let index = NEXT.fetch_add(1, Ordering::Relaxed);
        if let Some(slot) = SAMPLES.get(index) {
            slot.store(pc, Ordering::Relaxed);
        } else {
            DROPPED.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn syscall_result(result: libc::c_int, operation: &str) -> Result<(), String> {
        if result == 0 {
            Ok(())
        } else {
            Err(format!("{operation}: {}", io::Error::last_os_error()))
        }
    }

    fn prof_set() -> libc::sigset_t {
        let mut set = unsafe { mem::zeroed() };
        unsafe {
            libc::sigemptyset(&mut set);
            libc::sigaddset(&mut set, libc::SIGPROF);
        }
        set
    }

    struct BlockedSigprof {
        previous: libc::sigset_t,
    }

    impl BlockedSigprof {
        fn new() -> Result<Self, String> {
            let mut previous = unsafe { mem::zeroed() };
            let code = unsafe { libc::pthread_sigmask(libc::SIG_BLOCK, &prof_set(), &mut previous) };
            if code != 0 {
                return Err(format!("pthread_sigmask: {}", io::Error::from_raw_os_error(code)));
            }
            Ok(Self { previous })
        }
    }

    impl Drop for BlockedSigprof {
        fn drop(&mut self) {
            unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, &self.previous, ptr::null_mut()) };
        }
    }

    struct Sampler {
        previous_action: libc::sigaction,
        previous_timer: libc::itimerval,
        restore_needed: bool,
    }

    impl Sampler {
        fn install(sampled: bool, interval_us: u32) -> Result<Self, String> {
            // The benchmark is deliberately single-threaded. This also makes
            // blocking SIGPROF sufficient to wait for all our handler activity.
            let threads = std::fs::read_dir("/proc/self/task")
                .map_err(|error| format!("Cannot inspect own thread count: {error}"))?
                .count();
            if threads != 1 {
                return Err(format!("Own-process sampler requires one thread, found {threads}"));
            }
            let blocked = BlockedSigprof::new()?;
            if unsafe { libc::sigismember(&blocked.previous, libc::SIGPROF) } != 0 {
                return Err("SIGPROF was already blocked; original mask preserved".into());
            }
            let mut pending = unsafe { mem::zeroed() };
            syscall_result(unsafe { libc::sigpending(&mut pending) }, "sigpending")?;
            if unsafe { libc::sigismember(&pending, libc::SIGPROF) } != 0 {
                return Err("SIGPROF was already pending; original state preserved".into());
            }
            let mut previous_action = unsafe { mem::zeroed() };
            let mut previous_timer = unsafe { mem::zeroed() };
            syscall_result(
                unsafe { libc::sigaction(libc::SIGPROF, ptr::null(), &mut previous_action) },
                "read sigaction",
            )?;
            syscall_result(unsafe { libc::getitimer(ITIMER_PROF, &mut previous_timer) }, "getitimer")?;
            let guard = Self {
                previous_action,
                previous_timer,
                restore_needed: true,
            };
            let disabled: libc::itimerval = unsafe { mem::zeroed() };
            syscall_result(unsafe { libc::setitimer(ITIMER_PROF, &disabled, ptr::null_mut()) }, "disable timer")?;
            RECORDING.store(false, Ordering::Release);
            for counter in [&DELIVERED, &NEXT, &DROPPED, &INVALID_CONTEXT, &OUTSIDE_WINDOW] {
                counter.store(0, Ordering::Relaxed);
            }
            // Touch the preallocated pages before the measurement window.
            for slot in &SAMPLES {
                slot.store(0, Ordering::Relaxed);
            }
            let mut action: libc::sigaction = unsafe { mem::zeroed() };
            action.sa_sigaction = handler as *const () as usize;
            action.sa_flags = libc::SA_SIGINFO | libc::SA_RESTART;
            unsafe { libc::sigemptyset(&mut action.sa_mask) };
            syscall_result(unsafe { libc::sigaction(libc::SIGPROF, &action, ptr::null_mut()) }, "install sigaction")?;
            if sampled {
                let period = libc::timeval {
                    tv_sec: (interval_us / 1_000_000).into(),
                    tv_usec: (interval_us % 1_000_000).into(),
                };
                let timer = libc::itimerval {
                    it_interval: period,
                    it_value: period,
                };
                syscall_result(unsafe { libc::setitimer(ITIMER_PROF, &timer, ptr::null_mut()) }, "arm ITIMER_PROF")?;
            }
            drop(blocked);
            Ok(guard)
        }

        fn restore(&mut self) -> Result<usize, String> {
            if !self.restore_needed {
                return Ok(0);
            }
            RECORDING.store(false, Ordering::Release);
            let _blocked = BlockedSigprof::new()?;
            let mut first_error = None;
            let mut check = |result, operation| {
                if let Err(error) = syscall_result(result, operation)
                    && first_error.is_none()
                {
                    first_error = Some(error);
                }
            };
            let disabled: libc::itimerval = unsafe { mem::zeroed() };
            check(unsafe { libc::setitimer(ITIMER_PROF, &disabled, ptr::null_mut()) }, "stop timer");
            let timeout = libc::timespec { tv_sec: 0, tv_nsec: 0 };
            let mut drained = 0;
            loop {
                let result = unsafe { libc::sigtimedwait(&prof_set(), ptr::null_mut(), &timeout) };
                if result == libc::SIGPROF {
                    drained += 1;
                    continue;
                }
                if result == -1 && io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                    continue;
                }
                break;
            }
            check(
                unsafe { libc::sigaction(libc::SIGPROF, &self.previous_action, ptr::null_mut()) },
                "restore sigaction",
            );
            check(
                unsafe { libc::setitimer(ITIMER_PROF, &self.previous_timer, ptr::null_mut()) },
                "restore timer",
            );
            self.restore_needed = false;
            if let Some(error) = first_error { Err(error) } else { Ok(drained) }
        }
    }

    impl Drop for Sampler {
        fn drop(&mut self) {
            let _ = self.restore();
        }
    }

    #[inline(never)]
    fn run_measured(benchmark: &mut Benchmark, batches: u32) -> Result<(), String> {
        benchmark.run_batches(batches)
    }

    fn object_at(pc: usize) -> Option<(String, usize)> {
        let mut info: libc::Dl_info = unsafe { mem::zeroed() };
        if unsafe { libc::dladdr(pc as *const libc::c_void, &mut info) } == 0 || info.dli_fbase.is_null() {
            return None;
        }
        let name = if info.dli_fname.is_null() {
            "<unnamed>".into()
        } else {
            unsafe { CStr::from_ptr(info.dli_fname) }.to_string_lossy().into_owned()
        };
        Some((name, info.dli_fbase as usize))
    }

    fn cpu_time_ns() -> Result<u64, String> {
        let mut ts: libc::timespec = unsafe { mem::zeroed() };
        syscall_result(
            unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut ts) },
            "process CPU clock",
        )?;
        Ok(ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64)
    }

    fn argument(args: &[String], index: usize, default: u32) -> Result<u32, String> {
        args.get(index)
            .map_or(Ok(default), |value| value.parse().map_err(|_| format!("Invalid integer: {value}")))
    }

    pub fn run() -> Result<(), String> {
        if cfg!(feature = "profiling") || cfg!(feature = "throughput") {
            return Err("Build this control/sample executable with only the pc-sampling feature".into());
        }
        let args = env::args().skip(1).collect::<Vec<_>>();
        if args.len() > 5 || args.iter().any(|arg| arg == "--help") {
            return Err("Usage: android_pc_sample [workload 0..2=0] [batches=10000] [sampled 0|1=1] [warmup_batches=1000] [period_us=1000]".into());
        }
        let workload = Workload::from_id(argument(&args, 0, 0)?)?;
        let batches = argument(&args, 1, 10_000)?;
        let mode = argument(&args, 2, 1)?;
        let warmup_batches = argument(&args, 3, 1000)?;
        let interval_us = argument(&args, 4, 1000)?;
        if mode > 1 || !(100..=1_000_000).contains(&interval_us) || batches == 0 {
            return Err("sampled must be 0 or 1, batches positive, period_us in 100..=1000000".into());
        }
        let instructions = batches.checked_mul(INSTRUCTIONS_PER_BATCH).ok_or("Instruction count overflow")?;
        log::set_max_level(log::LevelFilter::Warn);
        if warmup_batches != 0 {
            let mut warmup = Benchmark::new(workload)?;
            warmup.configure(0, 1, 1)?;
            warmup.run_batches(warmup_batches)?;
            warmup.finish()?;
        }
        let mut benchmark = Benchmark::new(workload)?;
        benchmark.configure(0, 1, 1)?;
        let anchor = run_measured as *const () as usize;
        let (executable, base) = object_at(anchor).ok_or("dladdr could not identify this executable")?;
        let mut sampler = Sampler::install(mode == 1, interval_us)?;
        let started_cpu = cpu_time_ns()?;
        let started = Instant::now();
        RECORDING.store(true, Ordering::Release);
        let run_result = run_measured(&mut benchmark, batches);
        RECORDING.store(false, Ordering::Release);
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        let elapsed_cpu_ns = cpu_time_ns()?.saturating_sub(started_cpu);
        let drained = sampler.restore()?;
        run_result?;
        let validation = benchmark.finish()?;
        let recorded = NEXT.load(Ordering::Relaxed).min(CAPACITY);
        let mut counts = BTreeMap::<usize, usize>::new();
        for sample in &SAMPLES[..recorded] {
            *counts.entry(sample.load(Ordering::Relaxed)).or_default() += 1;
        }
        let samples = counts
            .into_iter()
            .map(|(pc, count)| {
                let object = object_at(pc);
                serde_json::json!({
                    "pc": format!("0x{pc:x}"), "count": count,
                    "module": object.as_ref().map(|x| &x.0),
                    "moduleBase": object.as_ref().map(|x| format!("0x{:x}", x.1)),
                    "offset": object.as_ref().map(|x| format!("0x{:x}", pc - x.1)),
                    "inExecutable": object.as_ref().is_some_and(|x| x.1 == base),
                })
            })
            .collect::<Vec<_>>();
        println!(
            "{}",
            serde_json::json!({
                "formatVersion": 2, "runtime": "native-aarch64", "mechanism": "own_process_SIGPROF_ITIMER_PROF",
                "contextPcOffset": CONTEXT_PC_OFFSET,
                "sampled": mode == 1, "periodUsRequested": interval_us,
                "samplingStatus": if mode == 0 { "control" } else if recorded == 0 { "no_samples" } else { "captured" },
                "workload": workload, "batches": batches, "warmupBatches": warmup_batches,
                "guestInstructions": instructions, "elapsedMs": elapsed_ms, "elapsedCpuNs": elapsed_cpu_ns,
                "executable": executable, "executableBase": format!("0x{base:x}"), "anchorOffset": format!("0x{:x}", anchor - base),
                "bufferCapacity": CAPACITY, "recordedSamples": recorded, "droppedSamples": DROPPED.load(Ordering::Relaxed),
                "deliveredSignals": DELIVERED.load(Ordering::Relaxed), "outsideWindowSignals": OUTSIDE_WINDOW.load(Ordering::Relaxed),
                "invalidContexts": INVALID_CONTEXT.load(Ordering::Relaxed), "pendingSignalsDrained": drained,
                "timerAndHandlerRestored": true, "validated": validation["validated"], "validation": validation["validation"],
                "samples": samples,
            })
        );
        Ok(())
    }
}

#[cfg(all(target_os = "android", target_arch = "aarch64"))]
fn main() -> std::process::ExitCode {
    match android::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(not(all(target_os = "android", target_arch = "aarch64")))]
fn main() -> std::process::ExitCode {
    eprintln!("This diagnostic example supports native Android AArch64 only");
    std::process::ExitCode::FAILURE
}
