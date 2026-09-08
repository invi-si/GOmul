use alloc::{borrow::ToOwned, boxed::Box, collections::BTreeMap, format, string::String, sync::Arc, vec, vec::Vec};
use core::mem::size_of;

use spin::Mutex;

use wie_backend::{ProfileCallback, ProfileSample, YieldFuture};
use wie_util::{ByteRead, ByteWrite, Result, WieError, read_generic};

#[cfg(feature = "cpu-profiling")]
use crate::cpu_profile::{CoreProfileSnapshot, HostClock, ProfileMode, SvcSnapshot};
#[cfg(feature = "cpu-throughput")]
use crate::cpu_throughput::CpuThroughputSnapshot;
use crate::{
    EmulatedFunction, ResultWriter, ThreadId,
    context::ArmCoreContext,
    engine::{Arm32CpuEngine, ArmEngine, ArmRegister, EngineRunResult, MemoryPermission},
    function::{RegisteredFunction, RegisteredFunctionHolder},
    thread::ThreadState,
    thread_wrapper::ArmCoreThreadWrapper,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::{
    engine::{DebugInner, DebuggedArm32CpuEngine},
    gdb::GdbTarget,
};

const GLOBAL_DATA_BASE: u32 = 0x7fff0000;
const FUNCTIONS_BASE: u32 = 0x71000000;
const FUNCTIONS_SIZE: usize = 0x10000;
const SVC_STUB_SIZE: u32 = 16;
pub const RUN_FUNCTION_LR: u32 = 0x7f000000;
pub const HEAP_BASE: u32 = 0x40000000;
pub const HEAP_SIZE: u32 = 0x10000000;

/// Limit on stack frames recorded per sample. Bounds memory and protects
/// against runaway loops if the R7 chain forms a cycle.
const PROFILE_MAX_STACK: usize = 32;
/// Flush the per-stack counter map every this many samples taken.
const PROFILE_FLUSH_INTERVAL: u32 = 1000;

struct ProfileState {
    samples: BTreeMap<Vec<u32>, u64>,
    counter: u32,
    callback: ProfileCallback,
}

pub(crate) struct ArmCoreInner {
    pub(crate) engine: Box<dyn ArmEngine>,
    last_thread_id: ThreadId,
    threads: BTreeMap<ThreadId, ThreadState>,
    svc_handlers: BTreeMap<u32, Arc<Box<dyn RegisteredFunction>>>,
    next_stub_address: u32,
    profile: Option<ProfileState>,
    #[cfg(feature = "cpu-profiling")]
    cpu_profile: (ProfileMode, HostClock, SvcSnapshot),
}

impl Drop for ArmCoreInner {
    fn drop(&mut self) {
        if let Some(mut profile) = self.profile.take() {
            let batch = drain_samples(&mut profile.samples);
            if !batch.is_empty() {
                (profile.callback)(batch);
            }
        }
    }
}

fn drain_samples(samples: &mut BTreeMap<Vec<u32>, u64>) -> Vec<ProfileSample> {
    core::mem::take(samples)
        .into_iter()
        .map(|(stack, count)| ProfileSample { stack, count })
        .collect()
}

#[derive(Clone)]
pub struct ArmCore {
    pub(crate) inner: Arc<Mutex<ArmCoreInner>>, // TODO can we change it to another lock like async-lock?
}

impl ArmCore {
    pub fn new(enable_gdbserver: bool, profile: Option<ProfileCallback>) -> Result<Self> {
        let mut engine = if enable_gdbserver {
            #[cfg(not(target_arch = "wasm32"))]
            let engine = Box::new(DebuggedArm32CpuEngine::new()) as Box<dyn ArmEngine>;
            #[cfg(target_arch = "wasm32")]
            let engine = Box::new(Arm32CpuEngine::new());

            engine
        } else {
            Box::new(Arm32CpuEngine::new())
        };

        engine.mem_map(FUNCTIONS_BASE, FUNCTIONS_SIZE, MemoryPermission::ReadExecute);
        engine.mem_map(GLOBAL_DATA_BASE, 0x4000, MemoryPermission::ReadWriteExecute);

        let profile = profile.map(|callback| ProfileState {
            samples: BTreeMap::new(),
            counter: 0,
            callback,
        });

        let inner = ArmCoreInner {
            engine,
            last_thread_id: 0,
            threads: BTreeMap::new(),
            svc_handlers: BTreeMap::new(),
            next_stub_address: FUNCTIONS_BASE,
            profile,
            #[cfg(feature = "cpu-profiling")]
            cpu_profile: (ProfileMode::Off, || 0, SvcSnapshot::default()),
        };

        let result = Self {
            inner: Arc::new(Mutex::new(inner)),
        };

        if enable_gdbserver {
            #[cfg(not(target_arch = "wasm32"))]
            GdbTarget::start(result.clone())?;
            #[cfg(target_arch = "wasm32")]
            panic!("GDB server is not supported on wasm32");
        }

        Ok(result)
    }

    /// Clear instruction throughput totals without changing guest state.
    #[cfg(feature = "cpu-throughput")]
    pub fn reset_cpu_throughput(&self) -> Result<()> {
        let mut inner = self.inner.lock();
        let engine = inner
            .engine
            .as_any_mut()
            .downcast_mut::<Arm32CpuEngine>()
            .ok_or_else(|| WieError::FatalError("CPU throughput counters require the normal ARM engine without the GDB adapter".into()))?;
        engine.reset_throughput();
        Ok(())
    }

    /// Read instruction totals. This does not read a clock or execute guest code.
    #[cfg(feature = "cpu-throughput")]
    pub fn cpu_throughput_snapshot(&self) -> Result<CpuThroughputSnapshot> {
        let inner = self.inner.lock();
        let engine = inner
            .engine
            .as_any()
            .downcast_ref::<Arm32CpuEngine>()
            .ok_or_else(|| WieError::FatalError("CPU throughput counters are unavailable on this engine".into()))?;
        Ok(engine.throughput_snapshot())
    }

    #[cfg(feature = "cpu-profiling")]
    pub fn set_cpu_profiling(&self, mode: ProfileMode, mean_interval: u32, seed: u32, clock: HostClock) -> Result<()> {
        let mut inner = self.inner.lock();
        let engine = inner
            .engine
            .as_any_mut()
            .downcast_mut::<Arm32CpuEngine>()
            .ok_or_else(|| WieError::FatalError("CPU profiling requires the normal ARM engine without the GDB adapter".into()))?;
        engine.set_profiling(mode, mean_interval, seed, clock);
        inner.cpu_profile = (mode, clock, SvcSnapshot::default());
        Ok(())
    }

    #[cfg(feature = "cpu-profiling")]
    pub fn reset_cpu_profiling(&self) -> Result<()> {
        let mut inner = self.inner.lock();
        let engine = inner
            .engine
            .as_any_mut()
            .downcast_mut::<Arm32CpuEngine>()
            .ok_or_else(|| WieError::FatalError("CPU profiling is unavailable on this engine".into()))?;
        engine.reset_profiling();
        inner.cpu_profile.2 = SvcSnapshot::default();
        Ok(())
    }

    #[cfg(feature = "cpu-profiling")]
    pub fn cpu_profiling_snapshot(&self) -> Result<CoreProfileSnapshot> {
        let inner = self.inner.lock();
        let engine = inner
            .engine
            .as_any()
            .downcast_ref::<Arm32CpuEngine>()
            .ok_or_else(|| WieError::FatalError("CPU profiling is unavailable on this engine".into()))?;
        Ok(CoreProfileSnapshot {
            engine: engine.profiling_snapshot(),
            svc: inner.cpu_profile.2,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn debug_inner(&self) -> Option<Arc<DebugInner>> {
        let inner = self.inner.lock();

        inner
            .engine
            .as_any()
            .downcast_ref::<DebuggedArm32CpuEngine>()
            .map(|engine| engine.debug_inner())
    }

    pub fn load(&mut self, data: &[u8], address: u32, map_size: usize) -> Result<()> {
        let mut inner = self.inner.lock();

        inner
            .engine
            .mem_map(address, map_size.next_multiple_of(0x1000), MemoryPermission::ReadWriteExecute);
        inner.engine.mem_write(address, data)?;

        Ok(())
    }

    pub fn run_in_thread<F, Fut>(&self, entry: F) -> Result<ArmCoreThreadWrapper>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let state = ThreadState::new(self.clone())?;

        let thread_id = {
            let mut inner = self.inner.lock();

            let thread_id = inner.last_thread_id + 1;
            inner.last_thread_id += 1;
            inner.threads.insert(thread_id, state);

            thread_id
        };

        tracing::info!("Create thread: {thread_id}");

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(debug) = self.debug_inner() {
                debug.on_thread_created(thread_id);
            }
        }

        ArmCoreThreadWrapper::new(self.clone(), thread_id, entry)
    }

    pub fn delete_thread_context(&self, thread_id: ThreadId) {
        tracing::info!("Terminate thread: {thread_id}");

        // we should exit inner lock first to run cleanup on thread state drop
        let _thread_state = {
            let mut inner = self.inner.lock();
            inner.threads.remove(&thread_id)
        };

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(debug) = self.debug_inner() {
            debug.on_thread_deleted(thread_id);
        }
    }

    pub fn enter_thread_context(&self, thread_id: ThreadId) -> ThreadContextGuard {
        ThreadContextGuard::new(self.clone(), thread_id)
    }

    pub fn read_thread_context(&self, thread_id: ThreadId) -> Result<ArmCoreContext> {
        let inner = self.inner.lock();

        let context = inner.threads.get(&thread_id).unwrap().context.clone();

        Ok(context)
    }

    pub fn write_thread_context(&mut self, thread_id: ThreadId, context: &ArmCoreContext) {
        let mut inner = self.inner.lock();

        inner.threads.get_mut(&thread_id).unwrap().context = context.clone();
    }

    pub fn get_thread_ids(&self) -> Vec<ThreadId> {
        let inner = self.inner.lock();

        inner.threads.keys().cloned().collect()
    }

    /// Conservative guest roots for a non-moving, guest-heap object collector.
    /// Snapshot the loaded image, registers and live portions of every ARM stack.
    /// No guest instructions or callbacks run while this snapshot is taken.
    pub fn native_heap_reference_candidates(&self, image_base: u32, image_size: u32) -> Result<Vec<usize>> {
        let current = self.save_context();
        let mut inner = self.inner.lock();
        let mut roots = Vec::new();
        let mut add = |word: u32| {
            if (HEAP_BASE..HEAP_BASE + HEAP_SIZE).contains(&word) && word.is_multiple_of(4) {
                roots.push(word as usize);
            }
        };
        let contexts: Vec<_> = inner
            .threads
            .values()
            .map(|thread| {
                let end = (thread.stack_base + thread.stack_size) as u32;
                let context = if (thread.stack_base as u32..=end).contains(&current.sp) {
                    current.clone()
                } else {
                    thread.context.clone()
                };
                (context, thread.stack_base as u32, end)
            })
            .collect();
        for context in core::iter::once(&current).chain(contexts.iter().map(|(context, _, _)| context)) {
            for word in [
                context.r0, context.r1, context.r2, context.r3, context.r4, context.r5, context.r6, context.r7, context.r8, context.sb, context.sl,
                context.fp, context.ip, context.sp, context.lr, context.pc,
            ] {
                add(word);
            }
        }
        let mut scan = |start: u32, size: u32| -> Result<()> {
            let mut buffer = [0; 4096];
            let mut offset = 0;
            while offset < size {
                let count = (size - offset).min(buffer.len() as u32) as usize;
                inner.engine.mem_read(start + offset, count, &mut buffer[..count])?;
                for word in buffer[..count].as_chunks::<4>().0 {
                    add(u32::from_le_bytes(*word));
                }
                offset += count as u32;
            }
            Ok(())
        };
        scan(image_base, image_size)?;
        for (context, base, end) in contexts {
            if context.sp < base || context.sp > end || !context.sp.is_multiple_of(4) {
                return Err(WieError::FatalError("Invalid native stack bounds during GC".into()));
            }
            scan(context.sp, end - context.sp)?;
        }
        roots.sort_unstable();
        roots.dedup();
        Ok(roots)
    }

    fn sample_profile(&self) {
        let mut inner = self.inner.lock();
        if inner.profile.is_none() {
            return;
        }
        let pc = inner.engine.reg_read(ArmRegister::PC);
        let mut stack = vec![pc];
        let mut r7 = inner.engine.reg_read(ArmRegister::R7);
        for _ in 0..PROFILE_MAX_STACK {
            // Thumb frame: [saved R7 | saved LR] at [r7].
            let mut buf = [0u8; 8];
            if inner.engine.mem_read(r7, 8, &mut buf).is_err() {
                break;
            }
            let prev_r7 = u32::from_le_bytes(buf[0..4].try_into().unwrap());
            let lr = u32::from_le_bytes(buf[4..8].try_into().unwrap());
            // Heuristic stop: zero/null frame or non-Thumb LR (we only ever
            // call into Thumb code from the guest).
            if prev_r7 == 0 || lr == 0 || lr & 1 == 0 {
                break;
            }
            stack.push(lr);
            if prev_r7 <= r7 {
                // R7 chain must walk upward; bail on any inversion to avoid loops.
                break;
            }
            r7 = prev_r7;
        }
        let profile = inner.profile.as_mut().unwrap();
        *profile.samples.entry(stack).or_insert(0) += 1;
        profile.counter = profile.counter.wrapping_add(1);
        if profile.counter >= PROFILE_FLUSH_INTERVAL {
            profile.counter = 0;
            let batch = drain_samples(&mut profile.samples);
            (profile.callback)(batch);
        }
    }

    #[cfg(feature = "cpu-transcript-capture")]
    pub fn transcript_begin(&mut self, callback: u64) {
        crate::cpu_replay::transcript::begin(self.inner.lock().engine.as_mut(), callback);
    }
    #[cfg(feature = "cpu-transcript-capture")]
    pub fn transcript_end(&mut self, callback: u64, ok: bool) {
        crate::cpu_replay::transcript::end(self.inner.lock().engine.as_mut(), callback, ok);
    }

    pub async fn run_function<R>(&mut self, address: u32, params: &[u32]) -> Result<R>
    where
        R: RunFunctionResult<R>,
    {
        // we don't need to save r0-r3, but to make it simple, we save all registers
        let previous_context = self.save_context();
        {
            let mut inner = self.inner.lock();

            if !params.is_empty() {
                inner.engine.reg_write(ArmRegister::R0, params[0]);
            }
            if params.len() > 1 {
                inner.engine.reg_write(ArmRegister::R1, params[1]);
            }
            if params.len() > 2 {
                inner.engine.reg_write(ArmRegister::R2, params[2]);
            }
            if params.len() > 3 {
                inner.engine.reg_write(ArmRegister::R3, params[3]);
            }
            if params.len() > 4 {
                for param in params[4..].iter().rev() {
                    let sp: u32 = inner.engine.reg_read(ArmRegister::SP) - 4;

                    inner.engine.mem_write(sp, &param.to_le_bytes())?;
                    inner.engine.reg_write(ArmRegister::SP, sp);
                }
            }

            inner.engine.reg_write(ArmRegister::PC, address);
            inner.engine.reg_write(ArmRegister::LR, RUN_FUNCTION_LR);

            let cpsr = inner.engine.reg_read(ArmRegister::Cpsr);
            let new_cpsr = (cpsr & !0x3f) | 0x1f | ((address & 1) << 5);
            inner.engine.reg_write(ArmRegister::Cpsr, new_cpsr);
        }

        loop {
            let result = {
                let mut inner = self.inner.lock();
                #[cfg(feature = "cpu-replay")]
                {
                    if let Some(result) = crate::cpu_replay::try_record(inner.engine.as_mut(), RUN_FUNCTION_LR, 10_000) {
                        result?
                    } else {
                        inner.engine.run(RUN_FUNCTION_LR, 10_000)?
                    }
                }
                #[cfg(not(feature = "cpu-replay"))]
                inner.engine.run(RUN_FUNCTION_LR, 10_000)?
            };

            self.sample_profile();

            wie_util::input_trace::event(
                wie_util::input_trace::EXIT,
                b'I',
                0,
                match &result {
                    EngineRunResult::End => 0,
                    EngineRunResult::CountExhausted => 1,
                    EngineRunResult::Svc { category, .. } => 0x100000000 | u64::from(*category),
                },
            );
            match result {
                EngineRunResult::End => break,
                EngineRunResult::CountExhausted => YieldFuture::new().await, // yield to allow other tasks to run
                EngineRunResult::Svc { category, lr, spsr } => {
                    {
                        let mut inner = self.inner.lock();
                        // Restore the pre-exception execution state before running the Rust SVC handler.
                        inner.engine.reg_write(ArmRegister::Cpsr, spsr);
                        inner.engine.reg_write(ArmRegister::PC, lr);
                    }

                    let function = {
                        let inner = self.inner.lock();
                        inner
                            .svc_handlers
                            .get(&category)
                            .cloned()
                            .ok_or_else(|| WieError::FatalError(format!("Unknown SVC handler category: {category}")))?
                    };

                    let mut self1 = self.clone();
                    #[cfg(not(feature = "cpu-profiling"))]
                    {
                        let mut future = core::pin::pin!(function.call(&mut self1));
                        core::future::poll_fn(|cx| {
                            let mut trace = wie_util::input_trace::span(wie_util::input_trace::HOST_POLL, u64::from(category));
                            let result = future.as_mut().poll(cx);
                            trace.result = u64::from(result.is_ready());
                            result
                        })
                        .await?;
                    }
                    #[cfg(feature = "cpu-profiling")]
                    {
                        let profile_core = self.clone();
                        {
                            let mut inner = profile_core.inner.lock();
                            if inner.cpu_profile.0 != ProfileMode::Off {
                                inner.cpu_profile.2.calls += 1;
                            }
                        }
                        let mut future = core::pin::pin!(function.call(&mut self1));
                        core::future::poll_fn(|cx| {
                            let clock = {
                                let mut inner = profile_core.inner.lock();
                                if inner.cpu_profile.0 != ProfileMode::Off {
                                    inner.cpu_profile.2.polls += 1;
                                }
                                (inner.cpu_profile.0 == ProfileMode::Sampled).then_some(inner.cpu_profile.1)
                            };
                            let started = clock.map(|clock| clock());
                            let result = future.as_mut().poll(cx);
                            if let (Some(clock), Some(started)) = (clock, started) {
                                let ended = clock();
                                let mut inner = profile_core.inner.lock();
                                inner.cpu_profile.2.poll_inclusive_ns += ended.saturating_sub(started);
                                inner.cpu_profile.2.clock_regressions += u64::from(ended < started);
                            }
                            result
                        })
                        .await?;
                    }
                }
            }
        }

        let result = R::get(self);
        self.restore_context(&previous_context);

        Ok(result)
    }

    pub fn register_svc_handler<F, C, R, P>(&mut self, category: u32, handler: F, context: &C) -> Result<()>
    where
        F: EmulatedFunction<C, R, P> + 'static + Sync + Send,
        C: Clone + 'static + Sync + Send,
        R: ResultWriter<R> + Sync + Send + 'static,
        P: Sync + Send + 'static,
    {
        let mut inner = self.inner.lock();

        if inner.svc_handlers.contains_key(&category) {
            return Err(WieError::FatalError(format!("SVC handler already registered for {category}")));
        }

        inner
            .svc_handlers
            .insert(category, Arc::new(Box::new(RegisteredFunctionHolder::new(handler, context))));

        Ok(())
    }

    pub fn make_svc_stub(&mut self, category: u32, id: impl Into<u32>) -> Result<u32> {
        let mut inner = self.inner.lock();
        let id = id.into();

        if !inner.svc_handlers.contains_key(&category) {
            return Err(WieError::FatalError(format!("Unknown SVC handler category: {category}")));
        }

        let address = inner.next_stub_address;
        if address + SVC_STUB_SIZE > FUNCTIONS_BASE + FUNCTIONS_SIZE as u32 {
            return Err(WieError::FatalError("SVC stub space exhausted".into()));
        }
        inner.next_stub_address += SVC_STUB_SIZE;

        let stub = [
            0x10,
            0xb4, // push {r4}
            0x02,
            0x4c, // ldr r4, [pc, #8]
            0xa4,
            0x46, // mov r12, r4
            0x10,
            0xbc, // pop {r4}
            category as u8,
            0xdf, // svc #category
            0x70,
            0x47, // bx lr
        ]
        .into_iter()
        .chain(id.to_le_bytes())
        .collect::<Vec<_>>();
        inner.engine.mem_write(address, &stub)?;

        tracing::trace!("Register SVC stub at {address:#x}, category={category}, id={id}");

        Ok(address + 1)
    }

    pub fn map(&mut self, address: u32, size: u32) -> Result<()> {
        tracing::trace!("Map address: {address:#x}, size: {size:#x}");

        let mut inner = self.inner.lock();

        inner.engine.mem_map(address, size as usize, MemoryPermission::ReadWrite);

        Ok(())
    }

    pub fn dump_reg_stack(&self, image_base: u32) -> String {
        format!(
            "\n{}\nPossible call stack:\n{}\nStack:\n{}",
            self.dump_regs(),
            self.dump_call_stack(image_base).unwrap(),
            self.dump_stack().unwrap()
        )
    }

    pub fn restore_context(&mut self, context: &ArmCoreContext) {
        let mut inner = self.inner.lock();

        inner.engine.reg_write(ArmRegister::R0, context.r0);
        inner.engine.reg_write(ArmRegister::R1, context.r1);
        inner.engine.reg_write(ArmRegister::R2, context.r2);
        inner.engine.reg_write(ArmRegister::R3, context.r3);
        inner.engine.reg_write(ArmRegister::R4, context.r4);
        inner.engine.reg_write(ArmRegister::R5, context.r5);
        inner.engine.reg_write(ArmRegister::R6, context.r6);
        inner.engine.reg_write(ArmRegister::R7, context.r7);
        inner.engine.reg_write(ArmRegister::R8, context.r8);
        inner.engine.reg_write(ArmRegister::SB, context.sb);
        inner.engine.reg_write(ArmRegister::SL, context.sl);
        inner.engine.reg_write(ArmRegister::FP, context.fp);
        inner.engine.reg_write(ArmRegister::IP, context.ip);
        inner.engine.reg_write(ArmRegister::SP, context.sp);
        inner.engine.reg_write(ArmRegister::LR, context.lr);
        inner.engine.reg_write(ArmRegister::PC, context.pc);
        inner.engine.reg_write(ArmRegister::Cpsr, context.cpsr);
    }

    pub fn save_context(&self) -> ArmCoreContext {
        let inner = self.inner.lock();

        ArmCoreContext {
            r0: inner.engine.reg_read(ArmRegister::R0),
            r1: inner.engine.reg_read(ArmRegister::R1),
            r2: inner.engine.reg_read(ArmRegister::R2),
            r3: inner.engine.reg_read(ArmRegister::R3),
            r4: inner.engine.reg_read(ArmRegister::R4),
            r5: inner.engine.reg_read(ArmRegister::R5),
            r6: inner.engine.reg_read(ArmRegister::R6),
            r7: inner.engine.reg_read(ArmRegister::R7),
            r8: inner.engine.reg_read(ArmRegister::R8),
            sb: inner.engine.reg_read(ArmRegister::SB),
            sl: inner.engine.reg_read(ArmRegister::SL),
            fp: inner.engine.reg_read(ArmRegister::FP),
            ip: inner.engine.reg_read(ArmRegister::IP),
            sp: inner.engine.reg_read(ArmRegister::SP),
            lr: inner.engine.reg_read(ArmRegister::LR),
            pc: inner.engine.reg_read(ArmRegister::PC),
            cpsr: inner.engine.reg_read(ArmRegister::Cpsr),
        }
    }

    pub fn read_pc_lr(&self) -> Result<(u32, u32)> {
        let inner = self.inner.lock();

        let lr = inner.engine.reg_read(ArmRegister::LR);
        let pc = inner.engine.reg_read(ArmRegister::PC);

        Ok((pc, lr))
    }

    pub fn write_return_value(&mut self, result: &[u32]) -> Result<()> {
        let mut inner = self.inner.lock();

        if !result.is_empty() {
            inner.engine.reg_write(ArmRegister::R0, result[0]);
        }
        if result.len() > 1 {
            inner.engine.reg_write(ArmRegister::R1, result[1]);
        }
        if result.len() > 2 {
            todo!() // TODO
        }

        Ok(())
    }

    pub fn set_next_pc(&mut self, pc: u32) -> Result<()> {
        let mut inner = self.inner.lock();

        inner.engine.reg_write(ArmRegister::PC, pc);

        let cpsr = inner.engine.reg_read(ArmRegister::Cpsr);
        let new_cpsr = if pc & 1 == 1 { cpsr | 0x20 } else { cpsr & !0x20 };
        inner.engine.reg_write(ArmRegister::Cpsr, new_cpsr);

        Ok(())
    }

    pub fn read_param(&self, pos: usize) -> Result<u32> {
        let inner = self.inner.lock();

        let result = if pos == 0 {
            inner.engine.reg_read(ArmRegister::R0)
        } else if pos == 1 {
            inner.engine.reg_read(ArmRegister::R1)
        } else if pos == 2 {
            inner.engine.reg_read(ArmRegister::R2)
        } else if pos == 3 {
            inner.engine.reg_read(ArmRegister::R3)
        } else {
            let sp = inner.engine.reg_read(ArmRegister::SP);

            drop(inner);

            read_generic(self, sp + 4 * (pos as u32 - 4))?
        };

        Ok(result)
    }

    pub(crate) fn dump_regs_inner(engine: &dyn ArmEngine) -> String {
        [
            format!(
                "R0: {:#x} R1: {:#x} R2: {:#x} R3: {:#x} R4: {:#x} R5: {:#x} R6: {:#x} R7: {:#x} R8: {:#x}",
                engine.reg_read(ArmRegister::R0),
                engine.reg_read(ArmRegister::R1),
                engine.reg_read(ArmRegister::R2),
                engine.reg_read(ArmRegister::R3),
                engine.reg_read(ArmRegister::R4),
                engine.reg_read(ArmRegister::R5),
                engine.reg_read(ArmRegister::R6),
                engine.reg_read(ArmRegister::R7),
                engine.reg_read(ArmRegister::R8),
            ),
            format!(
                "SB: {:#x} SL: {:#x} FP: {:#x} IP: {:#x} SP: {:#x} LR: {:#x} PC: {:#x}",
                engine.reg_read(ArmRegister::SB),
                engine.reg_read(ArmRegister::SL),
                engine.reg_read(ArmRegister::FP),
                engine.reg_read(ArmRegister::IP),
                engine.reg_read(ArmRegister::SP),
                engine.reg_read(ArmRegister::LR),
                engine.reg_read(ArmRegister::PC),
            ),
            format!("CPSR: {:032b}\n", engine.reg_read(ArmRegister::Cpsr)),
        ]
        .join("\n")
    }

    fn is_code_address(address: u32, image_base: u32) -> bool {
        // TODO image size temp

        (address % 2 == 1 && (image_base..image_base + 0x100000).contains(&address))
            || (FUNCTIONS_BASE..FUNCTIONS_BASE + FUNCTIONS_SIZE as u32).contains(&address)
    }

    fn dump_regs(&self) -> String {
        let inner = self.inner.lock();

        Self::dump_regs_inner(&*inner.engine)
    }

    fn format_callstack_address(address: u32, image_base: u32) -> String {
        let description = if (image_base..image_base + 0x100000).contains(&address) {
            format!("<Base>+{:#x}", address - image_base)
        } else if (FUNCTIONS_BASE..FUNCTIONS_BASE + FUNCTIONS_SIZE as u32).contains(&address) {
            "<Native function>".to_owned()
        } else {
            "<Unknown>".to_owned()
        };

        format!("{address:#x}: {description}\n")
    }

    fn dump_call_stack(&self, image_base: u32) -> Result<String> {
        let mut inner = self.inner.lock();

        let sp = inner.engine.reg_read(ArmRegister::SP);
        let pc = inner.engine.reg_read(ArmRegister::PC);
        let lr = inner.engine.reg_read(ArmRegister::LR);

        let mut call_stack = Self::format_callstack_address(pc, image_base);
        if lr != RUN_FUNCTION_LR && lr != 0 {
            call_stack += &Self::format_callstack_address(lr - 5, image_base);
        }

        for i in 0..128 {
            let address = sp + (i * 4);
            if !inner.engine.is_mapped(address, size_of::<u32>()) {
                break;
            }

            let mut value = [0; size_of::<u32>()];
            inner.engine.mem_read(address, size_of::<u32>(), &mut value)?;
            let value_u32 = u32::from_le_bytes(value);

            if value_u32 > 5 && Self::is_code_address(value_u32 - 4, image_base) {
                call_stack += &Self::format_callstack_address(value_u32 - 5, image_base);
            }
        }

        Ok(call_stack)
    }

    fn dump_stack(&self) -> Result<String> {
        let mut inner = self.inner.lock();

        let sp = inner.engine.reg_read(ArmRegister::SP);

        let mut result = String::new();
        for i in 0..16 {
            let address = sp + (i * 4);

            if !inner.engine.is_mapped(address, size_of::<u32>()) {
                break;
            }

            let mut value = [0; size_of::<u32>()];
            inner.engine.mem_read(address, size_of::<u32>(), &mut value)?;
            let value_u32 = u32::from_le_bytes(value);

            result += &format!("SP+{:#x}: {value_u32:#x}\n", i * 4);
        }

        Ok(result)
    }
}

impl ByteRead for ArmCore {
    fn read_bytes(&self, address: u32, result: &mut [u8]) -> wie_util::Result<usize> {
        let mut inner = self.inner.lock();

        let read = inner.engine.mem_read(address, result.len(), result)?;

        Ok(read)
    }
}

impl ByteWrite for ArmCore {
    fn write_bytes(&mut self, address: u32, data: &[u8]) -> wie_util::Result<()> {
        let mut inner = self.inner.lock();

        inner.engine.mem_write(address, data)?;

        Ok(())
    }
}

pub trait RunFunctionResult<R> {
    fn get(core: &ArmCore) -> R;
}

impl RunFunctionResult<u32> for u32 {
    fn get(core: &ArmCore) -> u32 {
        core.read_param(0).unwrap()
    }
}

impl RunFunctionResult<()> for () {
    fn get(_: &ArmCore) {}
}

pub struct ThreadContextGuard {
    core: ArmCore,
    thread_id: ThreadId,
}

impl ThreadContextGuard {
    pub fn new(mut core: ArmCore, thread_id: ThreadId) -> Self {
        let context = core.inner.lock().threads.get(&thread_id).unwrap().context.clone(); // TODO we might not need clone
        core.restore_context(&context);

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(debug) = core.debug_inner() {
            debug.on_thread_entered(thread_id);
        }

        Self { core, thread_id }
    }
}

impl Drop for ThreadContextGuard {
    fn drop(&mut self) {
        let context = self.core.save_context();

        let mut inner = self.core.inner.lock();
        inner.threads.get_mut(&self.thread_id).unwrap().context = context;
        drop(inner);

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(debug) = self.core.debug_inner() {
            debug.on_thread_exited(self.thread_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_roots_snapshot_image_registers_and_live_stacks() -> Result<()> {
        use wie_util::write_generic;
        let mut core = ArmCore::new(false, None)?;
        crate::Allocator::init(&mut core)?;
        core.map(0x100000, 0x1000)?;
        let mut thread = ThreadState::new(core.clone())?;
        let end = (thread.stack_base + thread.stack_size) as u32;
        thread.context.sp = end - 8;
        thread.context.r8 = HEAP_BASE + 0x100;
        write_generic(&mut core, end - 12, HEAP_BASE + 0x200)?; // dead stack area
        write_generic(&mut core, end - 8, HEAP_BASE + 0x300)?;
        write_generic(&mut core, end - 4, HEAP_BASE + 0x400)?;
        write_generic(&mut core, 0x100000, HEAP_BASE + 0x500)?;
        core.inner.lock().threads.insert(1, thread);
        let roots = core.native_heap_reference_candidates(0x100000, 0x1000)?;
        for offset in [0x100, 0x300, 0x400, 0x500] {
            assert!(roots.contains(&((HEAP_BASE + offset) as usize)));
        }
        assert!(!roots.contains(&((HEAP_BASE + 0x200) as usize)));
        let mut active = core.save_context();
        active.sp = end - 4;
        active.r5 = HEAP_BASE + 0x600;
        core.restore_context(&active);
        let roots = core.native_heap_reference_candidates(0x100000, 0x1000)?;
        assert!(roots.contains(&((HEAP_BASE + 0x600) as usize)));
        assert!(roots.contains(&((HEAP_BASE + 0x400) as usize)));
        assert!(!roots.contains(&((HEAP_BASE + 0x300) as usize)));
        assert!(!roots.contains(&((HEAP_BASE + 0x100) as usize)));
        assert!(core.native_heap_reference_candidates(0x200000, 4).is_err());
        core.delete_thread_context(1);
        Ok(())
    }

    async fn test_svc_handler(_core: &mut ArmCore, seen_id: &mut Option<u32>, id: crate::SvcId) -> Result<()> {
        *seen_id = Some(id.0);

        Ok(())
    }

    #[test]
    fn test_thumb_svc_stub_dispatch() {
        let mut core = ArmCore::new(false, None).unwrap();
        core.map(0x1000, 0x1000).unwrap();

        let mut context = core.save_context();
        context.sp = 0x2000;
        core.restore_context(&context);

        core.register_svc_handler(1, test_svc_handler, &None).unwrap();
        let first = core.make_svc_stub(1, 0u32).unwrap();
        let second = core.make_svc_stub(1, 1u32).unwrap();
        assert_eq!(first, FUNCTIONS_BASE + 1);
        assert_eq!(second, FUNCTIONS_BASE + SVC_STUB_SIZE + 1);

        let result = {
            let mut inner = core.inner.lock();
            inner.engine.reg_write(ArmRegister::Cpsr, 0x3f);
            inner.engine.reg_write(ArmRegister::PC, second);
            inner.engine.reg_write(ArmRegister::LR, RUN_FUNCTION_LR);
            inner.engine.run(RUN_FUNCTION_LR, 10).unwrap()
        };

        match result {
            EngineRunResult::Svc { category, lr, spsr } => {
                assert_eq!(category, 1);
                assert_eq!(lr, FUNCTIONS_BASE + SVC_STUB_SIZE + 10);
                assert_ne!(spsr & 0x20, 0);
            }
            EngineRunResult::End => panic!("expected SVC, got end"),
            EngineRunResult::CountExhausted => panic!("expected SVC, got count exhausted"),
        }
    }

    #[cfg(feature = "cpu-profiling")]
    mod profiling_regressions {
        use super::*;
        use ::core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

        static TICKS: AtomicU64 = AtomicU64::new(0);
        fn clock() -> u64 {
            TICKS.fetch_add(10, Ordering::Relaxed)
        }
        fn panic_clock() -> u64 {
            panic!("untimed core observations read the host clock")
        }

        async fn pending_handler(_core: &mut ArmCore, calls: &mut Arc<AtomicU32>, id: crate::SvcId) -> Result<u32> {
            calls.fetch_add(1, Ordering::Relaxed);
            let mut pending = true;
            ::core::future::poll_fn(|cx| {
                if pending {
                    pending = false;
                    cx.waker().wake_by_ref();
                    ::core::task::Poll::Pending
                } else {
                    ::core::task::Poll::Ready(())
                }
            })
            .await;
            Ok(id.0 + 9)
        }

        #[test]
        fn normal_engine_profile_api_and_pending_svc_preserve_results_in_all_modes() {
            for mode in [ProfileMode::Off, ProfileMode::Counts, ProfileMode::Sampled] {
                let mut core = ArmCore::new(false, None).unwrap();
                core.map(0x1000, 0x1000).unwrap();
                let mut context = core.save_context();
                context.sp = 0x1800;
                core.restore_context(&context);
                let calls = Arc::new(AtomicU32::new(0));
                core.register_svc_handler(1, pending_handler, &calls).unwrap();
                let stub = core.make_svc_stub(1, 7u32).unwrap();
                core.set_cpu_profiling(mode, 1, 59, if mode == ProfileMode::Sampled { clock } else { panic_clock })
                    .unwrap();
                let result: u32 = futures::executor::block_on(core.run_function(stub, &[])).unwrap();
                assert_eq!(result, 16);
                assert_eq!(calls.load(Ordering::Relaxed), 1);
                assert_eq!(core.save_context().sp, 0x1800);
                let profile = core.cpu_profiling_snapshot().unwrap();
                assert_eq!(profile.engine.cpu.mode, mode);
                assert_eq!(profile.engine.cpu.seed, 59);
                if mode == ProfileMode::Off {
                    assert_eq!(profile.svc.calls, 0);
                    assert_eq!(profile.svc.polls, 0);
                    assert_eq!(profile.engine.wrapper.run_calls, 0);
                    assert_eq!(profile.engine.cpu.instruction_attempts, 0);
                } else {
                    assert_eq!(profile.svc.calls, 1);
                    assert_eq!(profile.svc.polls, 2);
                    assert_eq!(profile.engine.wrapper.svc_exits, 1);
                    assert_eq!(profile.engine.wrapper.function_returns, 1);
                    assert_eq!(profile.engine.wrapper.run_calls, 2);
                    assert_eq!(profile.engine.cpu.exception_entries, 1);
                    assert!(profile.engine.cpu.instruction_attempts >= 5);
                }
                if mode == ProfileMode::Sampled {
                    assert!(profile.svc.poll_inclusive_ns > 0);
                    assert!(profile.engine.cpu.clock_reads > 0);
                } else {
                    assert_eq!(profile.svc.poll_inclusive_ns, 0);
                    assert_eq!(profile.engine.cpu.clock_reads, 0);
                    assert_eq!(profile.engine.wrapper.clock_reads, 0);
                }
                core.reset_cpu_profiling().unwrap();
                let reset = core.cpu_profiling_snapshot().unwrap();
                assert_eq!(reset.engine.cpu.mode, mode);
                assert_eq!(reset.engine.cpu.seed, 59);
                assert_eq!(reset.engine.cpu.instruction_attempts, 0);
                assert_eq!(reset.engine.wrapper.run_calls, 0);
                assert_eq!(reset.svc.calls, 0);
                assert_eq!(reset.svc.polls, 0);
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        #[test]
        fn profile_api_rejects_debug_adapter_without_starting_a_server() {
            let core = ArmCore::new(false, None).unwrap();
            core.inner.lock().engine = Box::new(DebuggedArm32CpuEngine::new());
            assert!(core.set_cpu_profiling(ProfileMode::Counts, 1, 1, panic_clock).is_err());
            assert!(core.reset_cpu_profiling().is_err());
            assert!(core.cpu_profiling_snapshot().is_err());
        }
    }

    #[cfg(feature = "cpu-throughput")]
    #[test]
    fn throughput_api_counts_returning_function_and_reset_keeps_registers() {
        let mut core = ArmCore::new(false, None).unwrap();
        core.map(0x1000, 0x1000).unwrap();
        let mut context = core.save_context();
        context.sp = 0x1800;
        context.r4 = 0xfeed;
        core.restore_context(&context);
        core.inner.lock().engine.mem_write(0x1000, &[0x07, 0x20, 0x70, 0x47]).unwrap(); // MOV r0,7; BX lr
        assert_eq!(core.cpu_throughput_snapshot().unwrap().total_instructions, 0);
        let result: u32 = futures::executor::block_on(core.run_function(0x1001, &[])).unwrap();
        assert_eq!(result, 7);
        let snapshot = core.cpu_throughput_snapshot().unwrap();
        assert_eq!(
            (snapshot.arm_instructions, snapshot.thumb_instructions, snapshot.total_instructions),
            (0, 2, 2)
        );
        assert_eq!(snapshot.full_profiling_compiled, cfg!(feature = "cpu-profiling"));
        core.reset_cpu_throughput().unwrap();
        assert_eq!(core.cpu_throughput_snapshot().unwrap().total_instructions, 0);
        let after = core.save_context();
        assert_eq!(
            (after.sp, after.r4, after.pc, after.cpsr),
            (context.sp, context.r4, context.pc, context.cpsr)
        );
    }

    #[cfg(all(feature = "cpu-throughput", not(target_arch = "wasm32")))]
    #[test]
    fn throughput_api_rejects_debug_adapter_without_starting_server() {
        let core = ArmCore::new(false, None).unwrap();
        core.inner.lock().engine = Box::new(DebuggedArm32CpuEngine::new());
        assert!(core.reset_cpu_throughput().is_err());
        assert!(core.cpu_throughput_snapshot().is_err());
    }
}
