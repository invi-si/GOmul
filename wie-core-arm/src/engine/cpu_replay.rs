//! Optional single-run CPU recording. No host calls are replayed: SVC/end/fault
//! boundaries terminate the recorded run just as they do in the interpreter.
use super::{Arm32CpuEngine, ArmEngine, Cpu, EngineRunResult, PAGE_COUNT, PAGE_SIZE};
use alloc::{format, string::String, vec::Vec};
use core::sync::atomic::{AtomicU8, Ordering};
use serde::{Deserialize, Serialize};
use spin::Mutex;
use wie_util::Result;

// 0 idle, 1 armed, 2 capturing, 3 ready. The ordinary diagnostic-build path
// performs one relaxed load per run, not per guest instruction.
static STATE: AtomicU8 = AtomicU8::new(0);
static CAPTURE: Mutex<Option<CapturedRun>> = Mutex::new(None);

pub fn request_capture() -> bool {
    STATE.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Relaxed).is_ok()
}
fn claim_request() -> bool {
    STATE.load(Ordering::Relaxed) == 1 && STATE.compare_exchange(1, 2, Ordering::AcqRel, Ordering::Relaxed).is_ok()
}
pub fn cancel_capture() {
    let _ = STATE.compare_exchange(1, 0, Ordering::AcqRel, Ordering::Relaxed);
    drop(take_capture());
}
pub fn take_capture() -> Option<CapturedRun> {
    if STATE.load(Ordering::Acquire) != 3 {
        return None;
    }
    let capture = CAPTURE.lock().take();
    STATE.store(0, Ordering::Release);
    capture
}
pub(crate) fn try_record(engine: &mut dyn ArmEngine, end: u32, count: u32) -> Option<Result<EngineRunResult>> {
    if STATE.load(Ordering::Relaxed) != 1 {
        return None;
    }
    let engine = engine.as_any_mut().downcast_mut::<Arm32CpuEngine>()?;
    if !claim_request() {
        return None;
    }
    Some(capture_requested(engine, end, count))
}

fn capture_requested(engine: &mut Arm32CpuEngine, end: u32, count: u32) -> Result<EngineRunResult> {
    let (capture, result) = record(engine, end, count);
    *CAPTURE.lock() = Some(capture);
    STATE.store(3, Ordering::Release);
    result
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Exit {
    End,
    CountExhausted,
    Svc { category: u32, lr: u32, spsr: u32 },
    Error(String),
}
impl Exit {
    fn from_result(result: &Result<EngineRunResult>) -> Self {
        match result {
            Ok(EngineRunResult::End) => Self::End,
            Ok(EngineRunResult::CountExhausted) => Self::CountExhausted,
            Ok(EngineRunResult::Svc { category, lr, spsr }) => Self::Svc {
                category: *category,
                lr: *lr,
                spsr: *spsr,
            },
            Err(error) => Self::Error(format!("{error:?}")),
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Page {
    index: usize,
    #[serde(with = "page_bytes")]
    bytes: Vec<u8>,
}
// Large guest heaps are mostly zero. Omit only the trailing zeros on disk;
// restore the complete fixed-size page before any execution or validation.
mod page_bytes {
    use super::*;
    pub fn serialize<S: serde::Serializer>(bytes: &[u8], serializer: S) -> core::result::Result<S::Ok, S::Error> {
        let end = bytes.iter().rposition(|&byte| byte != 0).map_or(0, |i| i + 1);
        bytes[..end].serialize(serializer)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> core::result::Result<Vec<u8>, D::Error> {
        let mut bytes = Vec::<u8>::deserialize(deserializer)?;
        if bytes.len() > PAGE_SIZE {
            return Err(serde::de::Error::custom("Page exceeds 64 KiB"));
        }
        bytes.resize(PAGE_SIZE, 0);
        Ok(bytes)
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct CapturedRun {
    version: u32,
    before: Cpu,
    pages: Vec<Page>,
    end: u32,
    count: u32,
    after: Cpu,
    changed: Vec<Page>,
    pub exit: Exit,
}
impl CapturedRun {
    /// Exact instruction count is known only for a fully exhausted budget.
    pub fn instructions(&self) -> Option<u32> {
        matches!(self.exit, Exit::CountExhausted).then_some(self.count)
    }
    pub fn mapped_bytes(&self) -> usize {
        self.pages.len() * PAGE_SIZE
    }
}

fn record(engine: &mut Arm32CpuEngine, end: u32, count: u32) -> (CapturedRun, Result<EngineRunResult>) {
    let before = engine.cpu;
    let pages: Vec<Page> = engine
        .mem
        .pages
        .iter()
        .enumerate()
        .filter_map(|(index, page)| page.as_ref().map(|page| Page { index, bytes: page.to_vec() }))
        .collect();
    // Recording is entered by the caller, never by run itself. Execute the
    // unchanged run implementation, preserving all boundary ordering.
    let result = engine.run(end, count);
    let changed = pages
        .iter()
        .filter_map(|page| {
            let after = engine.mem.pages[page.index].as_ref()?;
            (after.as_slice() != page.bytes).then(|| Page {
                index: page.index,
                bytes: after.to_vec(),
            })
        })
        .collect();
    (
        CapturedRun {
            version: 1,
            before,
            pages,
            end,
            count,
            after: engine.cpu,
            changed,
            exit: Exit::from_result(&result),
        },
        result,
    )
}

/// Restoring and validating memory are deliberately separate from timed run().
/// Every mapped page is checked, including pages expected to remain unchanged.
pub struct Replay {
    capture: CapturedRun,
    engine: Arm32CpuEngine,
    last: Option<Exit>,
}
impl Replay {
    pub fn new(capture: CapturedRun) -> core::result::Result<Self, String> {
        if capture.version != 1 {
            return Err("Unsupported capture version".into());
        }
        for pages in [&capture.pages, &capture.changed] {
            let mut previous = None;
            for page in pages {
                if page.index >= PAGE_COUNT || page.bytes.len() != PAGE_SIZE || previous.is_some_and(|p| p >= page.index) {
                    return Err("Invalid or unsorted memory pages".into());
                }
                previous = Some(page.index);
            }
        }
        if capture
            .changed
            .iter()
            .any(|p| capture.pages.binary_search_by_key(&p.index, |p| p.index).is_err())
        {
            return Err("Changed page was not mapped in input".into());
        }
        let mut engine = Arm32CpuEngine::new();
        for page in &capture.pages {
            engine.mem.pages[page.index] = Some(
                page.bytes
                    .clone()
                    .into_boxed_slice()
                    .try_into()
                    .map_err(|_| String::from("Invalid page size"))?,
            );
        }
        engine.cpu = capture.before;
        Ok(Self { capture, engine, last: None })
    }
    pub fn reset(&mut self) {
        self.engine.cpu = self.capture.before;
        #[cfg(feature = "cpu-throughput")]
        self.engine.reset_throughput();
        for page in &self.capture.pages {
            self.engine.mem.pages[page.index]
                .as_mut()
                .expect("replay page remains mapped")
                .copy_from_slice(&page.bytes);
        }
        // Reset caches when the experimental block engine is compiled, so code
        // restoration never bypasses its ordinary invalidation requirements.
        #[cfg(feature = "experimental-thumb-blocks")]
        {
            self.engine.blocks = super::thumb_blocks::BlockCache::new();
        }
        self.last = None;
    }
    pub fn run(&mut self) {
        self.last = Some(Exit::from_result(&self.engine.run(self.capture.end, self.capture.count)));
    }
    pub fn validate(&self) -> core::result::Result<(), String> {
        if self.last.as_ref() != Some(&self.capture.exit) {
            return Err("Run exit differs".into());
        }
        if self.engine.cpu != self.capture.after {
            return Err("Registers, flags, PC or bank differ".into());
        }
        let mapped = self.engine.mem.pages.iter().filter(|p| p.is_some()).count();
        if mapped != self.capture.pages.len() {
            return Err("Mapped page count differs".into());
        }
        for before in &self.capture.pages {
            let expected = self
                .capture
                .changed
                .binary_search_by_key(&before.index, |p| p.index)
                .map(|i| &self.capture.changed[i].bytes)
                .unwrap_or(&before.bytes);
            if self.engine.mem.pages[before.index].as_ref().map(|p| p.as_slice()) != Some(expected.as_slice()) {
                return Err(format!("Memory differs at page {:#x}", before.index));
            }
        }
        Ok(())
    }
    pub fn instructions(&self) -> Option<u32> {
        self.capture.instructions()
    }
    #[cfg(feature = "cpu-throughput")]
    pub fn instruction_counts(&self) -> crate::cpu_throughput::CpuThroughputSnapshot {
        self.engine.throughput_snapshot()
    }
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
    fn replay_twice(capture: CapturedRun) {
        let mut replay = Replay::new(capture).unwrap();
        for _ in 0..2 {
            replay.reset();
            replay.run();
            replay.validate().unwrap();
        }
    }
    #[test]
    fn capture_trigger_preserves_execution_and_returns_to_idle() {
        for code in [&[0x3001, 0xe7fd][..], &[0xdf01][..], &[0x6808][..]] {
            let mut recorded = engine(code);
            let mut oracle = engine(code);
            assert!(try_record(&mut recorded, 0x20000, 20).is_none());
            assert!(request_capture());
            assert!(!request_capture());
            let actual = try_record(&mut recorded, 0x20000, 20).unwrap();
            let expected = oracle.run(0x20000, 20);
            assert_eq!(Exit::from_result(&actual), Exit::from_result(&expected));
            assert!(recorded.cpu == oracle.cpu);
            for (actual, expected) in recorded.mem.pages.iter().zip(oracle.mem.pages.iter()) {
                assert_eq!(actual, expected);
            }
            replay_twice(take_capture().unwrap());
            assert!(take_capture().is_none());
        }
        assert!(request_capture());
        cancel_capture();
        assert!(request_capture());
        cancel_capture();
    }
    #[test]
    fn budget_branches_and_all_state_repeat() {
        let mut e = engine(&[0x3001, 0xe7fd]);
        let (c, _) = record(&mut e, 0x20000, 100);
        assert_eq!(c.instructions(), Some(100));
        replay_twice(c);
    }
    #[test]
    fn modified_code_is_restored_between_replays() {
        let mut e = engine(&[0x801a, 0x2001]); // STRH r2,[r3]; MOV r0,1
        e.cpu.reg_set(Mode::User, 2, 0x2009);
        e.cpu.reg_set(Mode::User, 3, 0x10002);
        let (c, _) = record(&mut e, 0x10004, 2);
        assert_eq!(c.after.reg_get(Mode::User, 0), 9);
        assert_eq!(c.changed.len(), 1);
        replay_twice(c);
    }
    #[test]
    fn svc_fault_and_zero_budget_boundaries_repeat() {
        for (code, count) in [(0xdf42, 2), (0x6811, 1), (0x3001, 0), (0xbf01, 1)] {
            let mut e = engine(&[code]);
            e.cpu.reg_set(Mode::User, 2, 0x30000);
            let (c, _) = record(&mut e, 0x20000, count);
            if code == 0xdf42 {
                assert!(matches!(c.exit, Exit::Svc { category: 0x42, .. }));
            }
            if code == 0x6811 || code == 0xbf01 {
                assert!(matches!(c.exit, Exit::Error(_)));
            }
            replay_twice(c);
        }
    }
    #[test]
    fn validation_detects_register_and_unexpected_memory_changes() {
        let mut e = engine(&[0x3001]);
        let (c, _) = record(&mut e, 0x10002, 1);
        let mut r = Replay::new(c).unwrap();
        r.run();
        r.validate().unwrap();
        r.engine.cpu.reg_set(Mode::User, 7, 123);
        assert!(r.validate().is_err());
        r.reset();
        r.run();
        r.engine.mem.write_range(0x10100, &[123]).unwrap();
        assert!(r.validate().is_err());
    }
    #[test]
    fn arm_to_thumb_transition_replays() {
        let mut e = engine(&[0, 0, 0, 0, 0x2107]);
        e.mem.write_range(0x10000, &0xe12fff10u32.to_le_bytes()).unwrap(); // BX r0
        e.cpu.reg_set(Mode::User, reg::CPSR, 0xa0000010);
        e.cpu.reg_set(Mode::User, 0, 0x10009);
        let (c, _) = record(&mut e, 0x1000a, 100);
        assert_eq!(c.after.reg_get(Mode::User, 1), 7);
        assert!(c.after.thumb_mode());
        replay_twice(c);
    }
    #[test]
    fn serialized_memory_fixture_roundtrips() {
        extern crate std;
        let mut e = engine(&[0x6811, 0x3101, 0x6011, 0xe7fb]);
        e.cpu.reg_set(Mode::User, 2, 0x10100);
        let (c, _) = record(&mut e, 0x20000, 10000);
        assert_eq!(c.instructions(), Some(10000));
        let json = serde_json::to_vec(&c).unwrap();
        replay_twice(serde_json::from_slice(&json).unwrap());
        if let Ok(directory) = std::env::var("WIE_REPLAY_FIXTURE_DIR") {
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(std::path::Path::new(&directory).join("synthetic-memory.json"), json).unwrap();
        }
    }
    #[test]
    fn compact_pages_restore_zeros_and_reject_oversize() {
        let page = Page {
            index: 3,
            bytes: alloc::vec![0; PAGE_SIZE],
        };
        let json = serde_json::to_string(&page).unwrap();
        assert!(json.len() < 32);
        assert_eq!(serde_json::from_str::<Page>(&json).unwrap().bytes, page.bytes);
        let padded: Page = serde_json::from_str(r#"{"index":3,"bytes":[1,2]}"#).unwrap();
        assert_eq!(&padded.bytes[..2], &[1, 2]);
        assert!(padded.bytes[2..].iter().all(|&b| b == 0));
        let oversized = serde_json::json!({"index":3,"bytes":alloc::vec![1; PAGE_SIZE + 1]});
        assert!(serde_json::from_value::<Page>(oversized).is_err());
    }
    #[test]
    fn malformed_page_rejected() {
        let mut e = engine(&[0x3001]);
        let (mut c, _) = record(&mut e, 0x10002, 1);
        c.pages[0].index = PAGE_COUNT;
        assert!(Replay::new(c).is_err());
    }
}
