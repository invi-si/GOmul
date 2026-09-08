//! CPU-only transcript of one timer callback. Environmental transitions include
//! host calls AND other tasks that ran while the callback was suspended.
//! Capture/verification write tracking is absent from ordinary replay builds.
#[cfg(feature = "cpu-transcript-capture")]
use super::EngineRunResult;
use super::{Arm32CpuEngine, ArmEngine, Cpu, Exit, PAGE_COUNT, PAGE_SIZE, Page};
use alloc::{collections::BTreeMap, format, string::String, vec, vec::Vec};
use serde::{Deserialize, Serialize};

#[cfg(any(feature = "cpu-transcript-capture", feature = "cpu-transcript-verify"))]
pub(crate) struct DirtyPages {
    bits: Vec<bool>,
    pages: Vec<usize>,
}
#[cfg(any(feature = "cpu-transcript-capture", feature = "cpu-transcript-verify"))]
impl DirtyPages {
    fn new() -> Self {
        Self {
            bits: vec![false; PAGE_COUNT],
            pages: Vec::new(),
        }
    }
    pub(crate) fn touch(&mut self, index: usize) {
        if index < PAGE_COUNT && !self.bits[index] {
            self.bits[index] = true;
            self.pages.push(index);
        }
    }
    fn drain(&mut self) -> Vec<usize> {
        let mut pages = core::mem::take(&mut self.pages);
        for &index in &pages {
            self.bits[index] = false;
        }
        pages.sort_unstable();
        pages
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Patch {
    offset: usize,
    bytes: Vec<u8>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Delta {
    index: usize,
    newly_mapped: bool,
    patches: Vec<Patch>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Segment {
    before: Cpu,
    environment: Vec<Delta>,
    end: u32,
    budget: u32,
    after: Cpu,
    writes: Vec<Delta>,
    exit: Exit,
    steps: u32,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Transcript {
    version: u32,
    pub callback: u64,
    entry: Cpu,
    pages: Vec<Page>,
    segments: Vec<Segment>,
    tail: Vec<Delta>,
    final_cpu: Cpu,
    pub returned_ok: bool,
}
impl Transcript {
    pub fn run_distribution(&self) -> Vec<(u32, String, u64)> {
        let mut counts = BTreeMap::new();
        for segment in &self.segments {
            *counts.entry((segment.steps, format!("{:?}", segment.exit))).or_insert(0) += 1;
        }
        counts.into_iter().map(|((steps, exit), count)| (steps, exit, count)).collect()
    }
    pub fn segments(&self) -> usize {
        self.segments.len()
    }
    pub fn steps(&self) -> u64 {
        self.segments.iter().map(|s| s.steps as u64).sum()
    }
    pub fn svc_exits(&self) -> usize {
        self.segments.iter().filter(|s| matches!(s.exit, Exit::Svc { .. })).count()
    }
    pub fn delta_bytes(&self) -> usize {
        self.segments
            .iter()
            .flat_map(|s| s.environment.iter().chain(&s.writes))
            .chain(&self.tail)
            .flat_map(|d| &d.patches)
            .map(|p| p.bytes.len())
            .sum()
    }
}
fn apply(pages: &mut BTreeMap<usize, Vec<u8>>, deltas: &[Delta]) -> core::result::Result<(), String> {
    let mut previous = None;
    for delta in deltas {
        if delta.index >= PAGE_COUNT || previous.is_some_and(|p| p >= delta.index) {
            return Err("Invalid delta page order".into());
        }
        previous = Some(delta.index);
        if delta.newly_mapped {
            if pages.insert(delta.index, vec![0; PAGE_SIZE]).is_some() {
                return Err("Page mapped twice".into());
            }
        }
        let page = pages.get_mut(&delta.index).ok_or("Delta targets unmapped page")?;
        let mut end = 0;
        for patch in &delta.patches {
            let next = patch.offset.checked_add(patch.bytes.len()).ok_or("Patch overflow")?;
            if patch.bytes.is_empty() || patch.offset < end || next > PAGE_SIZE {
                return Err("Invalid memory patch".into());
            }
            page[patch.offset..next].copy_from_slice(&patch.bytes);
            end = next;
        }
    }
    Ok(())
}

/// This is determined in the actual core dependency, including unified features.
pub const TIMING_ELIGIBLE: bool = !cfg!(any(
    feature = "cpu-transcript-capture",
    feature = "cpu-transcript-verify",
    feature = "cpu-profiling",
    feature = "cpu-throughput",
    feature = "cpu-exact-counts"
)) && !wie_util::input_trace::COMPILED_IN;

/// prepare/validate/reset are outside the timed run_segment call. A verification
/// build checks all actually dirtied pages, not only expected writes. Every build
/// also checks every mapped page at the end of the complete transcript.
pub struct Replay {
    capture: Transcript,
    engine: Arm32CpuEngine,
    expected: BTreeMap<usize, Vec<u8>>,
    next: usize,
    prepared: bool,
    last: Option<Exit>,
}
impl Replay {
    pub fn new(capture: Transcript) -> core::result::Result<Self, String> {
        if capture.version != 1 || capture.segments.is_empty() {
            return Err("Unsupported or empty transcript".into());
        }
        let mut expected = BTreeMap::new();
        let mut previous = None;
        for page in &capture.pages {
            if page.index >= PAGE_COUNT || page.bytes.len() != PAGE_SIZE || previous.is_some_and(|p| p >= page.index) {
                return Err("Invalid base pages".into());
            }
            previous = Some(page.index);
            expected.insert(page.index, page.bytes.clone());
        }
        // Validate all external deltas before executing any guest code.
        for segment in &capture.segments {
            if segment.steps > segment.budget {
                return Err("Steps exceed budget".into());
            }
            apply(&mut expected, &segment.environment)?;
            apply(&mut expected, &segment.writes)?;
        }
        apply(&mut expected, &capture.tail)?;
        let mut result = Self {
            capture,
            engine: Arm32CpuEngine::new(),
            expected,
            next: 0,
            prepared: false,
            last: None,
        };
        result.reset();
        Ok(result)
    }
    pub fn transcript(&self) -> &Transcript {
        &self.capture
    }
    pub fn reset(&mut self) {
        self.expected.clear();
        for page in &self.capture.pages {
            self.expected.insert(page.index, page.bytes.clone());
        }
        for (index, page) in self.engine.mem.pages.iter_mut().enumerate() {
            *page = self.expected.get(&index).map(|p| p.clone().into_boxed_slice().try_into().unwrap());
        }
        self.engine.cpu = self.capture.entry;
        // Keep translation caches warm across rounds. The block engine checks
        // current guest bytes before reuse, including bytes restored here.
        #[cfg(any(feature = "cpu-transcript-capture", feature = "cpu-transcript-verify"))]
        {
            self.engine.mem.dirty = Some(DirtyPages::new());
        }
        self.next = 0;
        self.prepared = false;
        self.last = None;
    }
    fn environment(&mut self, deltas: &[Delta]) -> core::result::Result<(), String> {
        apply(&mut self.expected, deltas)?;
        for delta in deltas {
            let expected = &self.expected[&delta.index];
            if delta.newly_mapped {
                self.engine.mem.pages[delta.index] = Some(expected.clone().into_boxed_slice().try_into().unwrap());
            } else {
                let page = self.engine.mem.pages[delta.index].as_mut().ok_or("Replay mapping disappeared")?;
                for patch in &delta.patches {
                    page[patch.offset..patch.offset + patch.bytes.len()].copy_from_slice(&patch.bytes);
                }
            }
        }
        Ok(())
    }
    pub fn prepare_segment(&mut self) -> core::result::Result<bool, String> {
        if self.prepared {
            return Err("Segment already prepared".into());
        }
        if self.next == self.capture.segments.len() {
            return Ok(false);
        }
        let environment = self.capture.segments[self.next].environment.clone();
        self.environment(&environment)?;
        self.engine.cpu = self.capture.segments[self.next].before;
        #[cfg(any(feature = "cpu-transcript-capture", feature = "cpu-transcript-verify"))]
        {
            self.engine.mem.dirty.as_mut().unwrap().drain();
        }
        self.prepared = true;
        self.last = None;
        Ok(true)
    }
    pub fn run_segment(&mut self) -> core::result::Result<(), String> {
        if !self.prepared || self.last.is_some() {
            return Err("Segment is not ready to run".into());
        }
        let segment = &self.capture.segments[self.next];
        #[cfg(feature = "cpu-exact-counts")]
        arm32_cpu::exact_counts::enable(true);
        self.last = Some(Exit::from_result(&self.engine.run(segment.end, segment.budget)));
        #[cfg(feature = "cpu-exact-counts")]
        arm32_cpu::exact_counts::enable(false);
        Ok(())
    }
    pub fn validate_segment(&mut self) -> core::result::Result<(), String> {
        if !self.prepared {
            return Err("No prepared segment".into());
        }
        let segment = &self.capture.segments[self.next];
        if self.last.as_ref() != Some(&segment.exit) || self.engine.cpu != segment.after {
            return Err(format!("Segment {} exit/CPU differs", self.next));
        }
        apply(&mut self.expected, &segment.writes)?;
        #[cfg(any(feature = "cpu-transcript-capture", feature = "cpu-transcript-verify"))]
        {
            if self.engine.last_run_steps != segment.steps {
                return Err(format!("Segment {} step count differs", self.next));
            }
            let mut pages = self.engine.mem.dirty.as_mut().unwrap().drain();
            pages.extend(segment.writes.iter().map(|d| d.index));
            pages.sort_unstable();
            pages.dedup();
            for index in pages {
                self.check_page(index)?;
            }
        }
        self.next += 1;
        self.prepared = false;
        Ok(())
    }
    fn check_page(&self, index: usize) -> core::result::Result<(), String> {
        if self.engine.mem.pages[index].as_ref().map(|p| p.as_slice()) != self.expected.get(&index).map(|p| p.as_slice()) {
            return Err(format!("Segment {} memory page {index:#x} differs", self.next));
        }
        Ok(())
    }
    pub fn finish(&mut self) -> core::result::Result<(), String> {
        if self.next != self.capture.segments.len() || self.prepared {
            return Err("Transcript incomplete".into());
        }
        // Check BEFORE restoring tail state so restoration cannot hide bad CPU writes.
        for index in 0..PAGE_COUNT {
            self.check_page(index)?;
        }
        self.environment(&self.capture.tail.clone())?;
        self.engine.cpu = self.capture.final_cpu;
        for index in 0..PAGE_COUNT {
            self.check_page(index)?;
        }
        Ok(())
    }
}

#[cfg(feature = "cpu-transcript-capture")]
mod capture {
    use super::*;
    use core::sync::atomic::{AtomicU8, Ordering};
    use spin::Mutex;
    use wie_util::{Result, input_trace::TimerRegistration};
    static STATE: AtomicU8 = AtomicU8::new(0); // idle, armed, active, ready
    static RECORDER: Mutex<Option<Recorder>> = Mutex::new(None);
    static READY: Mutex<Option<core::result::Result<Transcript, String>>> = Mutex::new(None);
    const MAX_SEGMENTS: usize = 100_000;
    const MAX_DELTA_BYTES: usize = 128 * 1024 * 1024;
    pub fn request_capture() -> bool {
        STATE.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Relaxed).is_ok()
    }
    pub fn take_capture() -> Option<core::result::Result<Transcript, String>> {
        if STATE.load(Ordering::Acquire) != 3 {
            return None;
        }
        let result = READY.lock().take();
        STATE.store(0, Ordering::Release);
        result
    }
    pub fn cancel_capture() {
        *RECORDER.lock() = None;
        *READY.lock() = None;
        STATE.store(0, Ordering::Release);
    }
    pub fn cancel_pending() {
        let _ = STATE.compare_exchange(1, 0, Ordering::AcqRel, Ordering::Relaxed);
    }
    pub(super) struct Recorder {
        transcript: Transcript,
        shadow: BTreeMap<usize, Vec<u8>>,
        bytes: usize,
    }
    impl Recorder {
        pub(super) fn new(engine: &mut Arm32CpuEngine, callback: u64) -> Self {
            let pages = engine
                .mem
                .pages
                .iter()
                .enumerate()
                .filter_map(|(index, p)| p.as_ref().map(|p| Page { index, bytes: p.to_vec() }))
                .collect();
            engine.mem.dirty = Some(DirtyPages::new());
            Self {
                transcript: Transcript {
                    version: 1,
                    callback,
                    entry: engine.cpu,
                    pages,
                    segments: Vec::new(),
                    tail: Vec::new(),
                    final_cpu: engine.cpu,
                    returned_ok: false,
                },
                shadow: BTreeMap::new(),
                bytes: 0,
            }
        }
        fn delta(&mut self, engine: &mut Arm32CpuEngine) -> Vec<Delta> {
            let indices = engine.mem.dirty.as_mut().unwrap().drain();
            let mut deltas = Vec::new();
            for index in indices {
                let Some(current) = engine.mem.pages[index].as_ref() else {
                    continue;
                };
                let old = self.shadow.get(&index).map(|p| p.as_slice()).or_else(|| {
                    self.transcript
                        .pages
                        .binary_search_by_key(&index, |p| p.index)
                        .ok()
                        .map(|i| self.transcript.pages[i].bytes.as_slice())
                });
                let newly_mapped = old.is_none();
                let mut patches = Vec::new();
                // Coalesce changed 64-byte lanes; avoid thousands of tiny allocations
                // for interleaved pixel/stack writes. All unchanged bytes stay exact.
                for offset in (0..PAGE_SIZE).step_by(64) {
                    let bytes = &current[offset..offset + 64];
                    if old.map_or_else(|| bytes.iter().any(|&b| b != 0), |p| &p[offset..offset + 64] != bytes) {
                        self.bytes += 64;
                        if let Some(Patch {
                            offset: start,
                            bytes: previous,
                        }) = patches.last_mut()
                        {
                            if *start + previous.len() == offset {
                                previous.extend_from_slice(bytes);
                                continue;
                            }
                        }
                        patches.push(Patch {
                            offset,
                            bytes: bytes.to_vec(),
                        });
                    }
                }
                if newly_mapped || !patches.is_empty() {
                    self.shadow.insert(index, current.to_vec());
                    deltas.push(Delta {
                        index,
                        newly_mapped,
                        patches,
                    });
                }
            }
            deltas
        }
        pub(super) fn run(&mut self, engine: &mut Arm32CpuEngine, end: u32, budget: u32) -> Result<EngineRunResult> {
            let environment = self.delta(engine);
            let before = engine.cpu;
            let result = engine.run(end, budget);
            let writes = self.delta(engine);
            self.transcript.segments.push(Segment {
                before,
                environment,
                end,
                budget,
                after: engine.cpu,
                writes,
                exit: Exit::from_result(&result),
                steps: engine.last_run_steps,
            });
            result
        }
    }
    pub fn begin(engine: &mut dyn ArmEngine, callback: u64) {
        if callback == 0 || STATE.load(Ordering::Relaxed) != 1 {
            return;
        }
        let Some(engine) = engine.as_any_mut().downcast_mut::<Arm32CpuEngine>() else {
            return;
        };
        if STATE.compare_exchange(1, 2, Ordering::AcqRel, Ordering::Relaxed).is_ok() {
            *RECORDER.lock() = Some(Recorder::new(engine, callback));
        }
    }
    pub(crate) fn try_record(engine: &mut dyn ArmEngine, end: u32, count: u32) -> Option<Result<EngineRunResult>> {
        if STATE.load(Ordering::Relaxed) != 2 {
            return None;
        }
        let mut slot = RECORDER.lock();
        let recorder = slot.as_mut()?;
        if recorder.transcript.callback != TimerRegistration::current_callback() {
            return None;
        }
        let engine = engine.as_any_mut().downcast_mut::<Arm32CpuEngine>()?;
        let result = recorder.run(engine, end, count);
        if recorder.transcript.segments.len() >= MAX_SEGMENTS || recorder.bytes > MAX_DELTA_BYTES {
            engine.mem.dirty = None;
            *slot = None;
            *READY.lock() = Some(Err("Transcript capture exceeded diagnostic size limit; discarded".into()));
            STATE.store(3, Ordering::Release);
        }
        Some(result)
    }
    pub fn end(engine: &mut dyn ArmEngine, callback: u64, ok: bool) {
        if STATE.load(Ordering::Relaxed) != 2 {
            return;
        }
        let mut slot = RECORDER.lock();
        if slot.as_ref().is_none_or(|r| r.transcript.callback != callback) {
            return;
        }
        let Some(engine) = engine.as_any_mut().downcast_mut::<Arm32CpuEngine>() else {
            return;
        };
        let mut recorder = slot.take().unwrap();
        recorder.transcript.tail = recorder.delta(engine);
        recorder.transcript.final_cpu = engine.cpu;
        recorder.transcript.returned_ok = ok;
        engine.mem.dirty = None;
        *READY.lock() = Some(if recorder.bytes > MAX_DELTA_BYTES {
            Err("Transcript tail exceeded size limit".into())
        } else if recorder.transcript.segments.is_empty() {
            Err("Callback contained no ARM CPU segments".into())
        } else {
            Ok(recorder.transcript)
        });
        STATE.store(3, Ordering::Release);
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        use arm32_cpu::{Mode, reg};
        fn engine(code: &[u16]) -> Arm32CpuEngine {
            let mut e = Arm32CpuEngine::new();
            e.mem.map(0x10000, PAGE_SIZE);
            for (i, word) in code.iter().enumerate() {
                e.mem.write_range(0x10000 + i as u32 * 2, &word.to_le_bytes()).unwrap();
            }
            e.cpu.reg_set(Mode::User, reg::CPSR, 0xa0000030);
            e.cpu.reg_set(Mode::User, reg::PC, 0x10000);
            e
        }
        fn finish(mut recorder: Recorder, e: &mut Arm32CpuEngine) -> Transcript {
            recorder.transcript.tail = recorder.delta(e);
            recorder.transcript.final_cpu = e.cpu;
            recorder.transcript.returned_ok = true;
            e.mem.dirty = None;
            recorder.transcript
        }
        fn replay(c: Transcript) {
            let json = serde_json::to_vec(&c).unwrap();
            let mut replay = Replay::new(serde_json::from_slice(&json).unwrap()).unwrap();
            for _ in 0..2 {
                replay.reset();
                while replay.prepare_segment().unwrap() {
                    replay.run_segment().unwrap();
                    replay.validate_segment().unwrap();
                }
                replay.finish().unwrap();
            }
        }
        #[test]
        fn transcript_host_writes_mapping_svc_flags_banked_registers() {
            let mut e = engine(&[0x3001, 0xdf42]);
            let mut r = Recorder::new(&mut e, 7);
            assert!(matches!(r.run(&mut e, 0x20000, 8), Ok(EngineRunResult::Svc { category: 0x42, .. })));
            e.mem.map(0x30000, PAGE_SIZE);
            e.mem.write_range(0x30010, &[1, 2, 3]).unwrap();
            e.mem.write_range(0x10000, &0x3002u16.to_le_bytes()).unwrap();
            e.cpu.reg_set(Mode::User, reg::CPSR, 0x60000030);
            e.cpu.reg_set(Mode::User, reg::PC, 0x10000);
            e.cpu.reg_set(Mode::Supervisor, reg::SP, 0x30080);
            r.run(&mut e, 0x10002, 3).unwrap();
            e.mem.write_range(0x30010, &[4, 5]).unwrap();
            let c = finish(r, &mut e);
            assert_eq!(c.segments(), 2);
            assert_eq!(c.steps(), 3);
            replay(c);
        }
        #[test]
        fn transcript_faults_budgets_branches_self_modifying_code() {
            for (code, budget) in [
                (&[0x3001, 0xe7fd][..], 100),
                (&[0x6811][..], 1),
                (&[0xbf01][..], 1),
                (&[0x3001][..], 0),
                (&[0x801a, 0x2001][..], 2),
            ] {
                let mut e = engine(code);
                e.cpu.reg_set(Mode::User, 2, 0x2009);
                e.cpu.reg_set(Mode::User, 3, 0x10002);
                let mut r = Recorder::new(&mut e, 1);
                let _ = r.run(&mut e, 0x20000, budget);
                replay(finish(r, &mut e));
            }
        }
        #[test]
        fn transcript_arm_thumb_transition() {
            let mut e = engine(&[0, 0, 0, 0, 0x2107]);
            e.mem.write_range(0x10000, &0xe12fff10u32.to_le_bytes()).unwrap();
            e.cpu.reg_set(Mode::User, reg::CPSR, 0xa0000010);
            e.cpu.reg_set(Mode::User, 0, 0x10009);
            let mut r = Recorder::new(&mut e, 1);
            r.run(&mut e, 0x1000a, 4).unwrap();
            replay(finish(r, &mut e));
        }
        #[test]
        fn transcript_interrupt_boundaries_replay_full_banked_state() {
            for exception in [arm32_cpu::Exception::Interrupt, arm32_cpu::Exception::FastInterrupt] {
                let mut e = engine(&[0x3001, 0xe7fd]);
                let mut r = Recorder::new(&mut e, 1);
                r.run(&mut e, 0x20000, 3).unwrap();
                e.cpu.exception(exception);
                assert!(r.run(&mut e, 0x20000, 3).is_err()); // Existing low-vector fault policy.
                replay(finish(r, &mut e));
            }
        }
        #[test]
        fn transcript_validation_rejects_unexpected_write_and_bad_deltas() {
            let mut e = engine(&[0x3001]);
            let mut r = Recorder::new(&mut e, 1);
            r.run(&mut e, 0x10002, 1).unwrap();
            let c = finish(r, &mut e);
            let mut replay = Replay::new(c.clone()).unwrap();
            replay.prepare_segment().unwrap();
            replay.run_segment().unwrap();
            replay.engine.mem.write_range(0x10100, &[123]).unwrap();
            assert!(replay.validate_segment().is_err());
            let mut bad = c;
            bad.segments[0].environment.push(Delta {
                index: PAGE_COUNT,
                newly_mapped: true,
                patches: Vec::new(),
            });
            assert!(Replay::new(bad).is_err());
        }
        #[test]
        fn callback_scope_excludes_suspended_work_and_other_callbacks() {
            use core::{
                future::Future,
                pin::Pin,
                task::{Context, Poll, Waker},
            };
            let mut e = engine(&[0x3001, 0xe7fd]);
            let timer = TimerRegistration::new(1, 2, 3, 60, 60, 0);
            assert!(request_capture());
            begin(&mut e, timer.id());
            // CPU work outside this callback becomes environmental state only.
            assert!(try_record(&mut e, 0x20000, 1).is_none());
            e.run(0x20000, 1).unwrap();
            struct SegmentFuture<'a> {
                engine: &'a mut Arm32CpuEngine,
                polls: u8,
            }
            impl Future for SegmentFuture<'_> {
                type Output = core::result::Result<(), ()>;
                fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
                    try_record(self.engine, 0x20000, 5).unwrap().unwrap();
                    self.polls += 1;
                    if self.polls == 1 { Poll::Pending } else { Poll::Ready(Err(())) }
                }
            }
            {
                let mut future = core::pin::pin!(timer.observe(1, 2, 3, SegmentFuture { engine: &mut e, polls: 0 }));
                let mut cx = Context::from_waker(Waker::noop());
                assert!(future.as_mut().poll(&mut cx).is_pending());
                assert_eq!(TimerRegistration::current_callback(), 0);
                assert!(future.as_mut().poll(&mut cx).is_ready());
            }
            end(&mut e, timer.id() + 1, true);
            assert!(take_capture().is_none());
            end(&mut e, timer.id(), false);
            let c = take_capture().unwrap().unwrap();
            assert_eq!(c.steps(), 10);
            assert_eq!(c.segments(), 2);
            assert!(!c.returned_ok);
            replay(c);
        }
    }
}
#[cfg(feature = "cpu-transcript-capture")]
pub(crate) use capture::try_record;
#[cfg(feature = "cpu-transcript-capture")]
pub use capture::{begin, cancel_capture, cancel_pending, end, request_capture, take_capture};

#[cfg(feature = "cpu-exact-counts")]
pub use arm32_cpu::exact_counts;
