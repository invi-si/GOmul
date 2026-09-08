use alloc::{boxed::Box, format, vec};
use core::cell::RefCell;

use arm32_cpu::{Cpu, Memory, Mode, reg};

use wie_util::{Result, WieError};

#[cfg(feature = "cpu-profiling")]
use crate::cpu_profile::{Boundary, EngineProfileSnapshot, HostClock, ProfileMode, WrapperProfiler};
#[cfg(feature = "cpu-throughput")]
use crate::cpu_throughput::{CpuThroughputSnapshot, ThroughputCounters};
use crate::engine::{ArmEngine, ArmRegister, EngineRunResult, MemoryPermission};

#[cfg(test)]
#[path = "arm32_cpu_execution_regressions.rs"]
mod execution_regressions;

#[cfg(feature = "experimental-thumb-blocks")]
#[path = "thumb_blocks.rs"]
mod thumb_blocks;

#[cfg(feature = "cpu-replay")]
#[path = "cpu_replay.rs"]
pub mod replay;

pub struct Arm32CpuEngine {
    #[cfg(feature = "experimental-thumb-blocks")]
    blocks: thumb_blocks::BlockCache,
    cpu: Cpu,
    mem: EmulatedMemory,
    #[cfg(feature = "cpu-throughput")]
    throughput: ThroughputCounters,
    #[cfg(feature = "cpu-profiling")]
    profiling: WrapperProfiler,
}

impl Default for Arm32CpuEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Arm32CpuEngine {
    pub fn new() -> Self {
        Self {
            cpu: Cpu::new(),
            #[cfg(feature = "experimental-thumb-blocks")]
            blocks: thumb_blocks::BlockCache::new(),
            mem: EmulatedMemory::new(),
            #[cfg(feature = "cpu-throughput")]
            throughput: ThroughputCounters::default(),
            #[cfg(feature = "cpu-profiling")]
            profiling: WrapperProfiler::default(),
        }
    }

    #[cfg(feature = "cpu-throughput")]
    pub fn reset_throughput(&mut self) {
        self.throughput.reset();
    }

    #[cfg(feature = "cpu-throughput")]
    pub fn throughput_snapshot(&self) -> CpuThroughputSnapshot {
        self.throughput.snapshot()
    }

    #[cfg(feature = "cpu-profiling")]
    pub fn set_profiling(&mut self, mode: ProfileMode, mean_interval: u32, seed: u32, clock: HostClock) {
        self.cpu.set_profiling(mode, mean_interval, seed, clock);
        self.profiling.configure(mode, mean_interval, seed, clock);
    }

    #[cfg(feature = "cpu-profiling")]
    pub fn reset_profiling(&mut self) {
        self.cpu.reset_profiling();
        self.profiling.reset();
    }

    #[cfg(feature = "cpu-profiling")]
    pub fn profiling_snapshot(&self) -> EngineProfileSnapshot {
        EngineProfileSnapshot {
            cpu: self.cpu.profiling_snapshot(),
            wrapper: self.profiling.stats,
        }
    }

    fn is_svc_exception(&self) -> bool {
        self.cpu.reg_get(Mode::User, reg::PC) == 0x08 && (self.cpu.reg_get(Mode::User, reg::CPSR) & 0x1f) == 0x13
    }

    fn read_svc_result(&mut self) -> Result<EngineRunResult> {
        let lr = self.cpu.reg_get(Mode::Supervisor, reg::LR);
        let spsr = self.cpu.reg_get(Mode::Supervisor, reg::SPSR);

        let svc_address = lr.checked_sub(2).ok_or(WieError::InvalidMemoryAccess(lr))?;
        let mut svc_bytes = [0u8; 2];
        self.mem.read_range(svc_address, 2, &mut svc_bytes)?;
        let instruction = u16::from_le_bytes(svc_bytes);
        if instruction & 0xff00 != 0xdf00 {
            return Err(WieError::FatalError(format!(
                "Invalid Thumb SVC instruction {instruction:#06x} at {svc_address:#x}"
            )));
        }

        let category = instruction as u32 & 0xff;

        Ok(EngineRunResult::Svc { category, lr, spsr })
    }
}

impl ArmEngine for Arm32CpuEngine {
    fn run(&mut self, end: u32, mut count: u32) -> Result<EngineRunResult> {
        #[cfg(feature = "cpu-throughput")]
        let (mut arm_steps, mut thumb_steps) = (0u32, 0u32);
        #[cfg(feature = "cpu-throughput")]
        let mut block_steps = 0u32;
        #[cfg(feature = "cpu-profiling")]
        let run_token = self.profiling.begin_run();

        macro_rules! finish {
            ($result:expr) => {{
                #[cfg(feature = "cpu-throughput")]
                {
                    self.throughput.record_run(arm_steps, thumb_steps);
                    self.throughput.record_blocks(block_steps);
                }
                #[cfg(feature = "cpu-profiling")]
                {
                    let result = $result;
                    self.profiling.end_run(run_token);
                    if self.profiling.mode != ProfileMode::Off {
                        match &result {
                            Ok(EngineRunResult::End) => self.profiling.stats.function_returns += 1,
                            Ok(EngineRunResult::CountExhausted) => self.profiling.stats.count_exhaustions += 1,
                            Ok(EngineRunResult::Svc { .. }) => self.profiling.stats.svc_exits += 1,
                            Err(_) => self.profiling.stats.errors += 1,
                        }
                    }
                    return result;
                }
                #[cfg(not(feature = "cpu-profiling"))]
                return $result;
            }};
        }
        loop {
            #[cfg(feature = "cpu-profiling")]
            let check_token = self.profiling.begin(Boundary::Checks);
            let pc = self.cpu.reg_get(Mode::User, reg::PC);

            if self.is_svc_exception() {
                #[cfg(feature = "cpu-profiling")]
                self.profiling.end(Boundary::Checks, check_token);
                finish!(self.read_svc_result());
            }

            if pc < 0x1000 {
                #[cfg(feature = "cpu-profiling")]
                self.profiling.end(Boundary::Checks, check_token);
                finish!(Err(WieError::InvalidMemoryAccess(pc)));
            }

            if pc == end {
                #[cfg(feature = "cpu-profiling")]
                self.profiling.end(Boundary::Checks, check_token);
                finish!(Ok(EngineRunResult::End));
            }

            if count == 0 {
                #[cfg(feature = "cpu-profiling")]
                self.profiling.end(Boundary::Checks, check_token);
                finish!(Ok(EngineRunResult::CountExhausted));
            }
            #[cfg(feature = "cpu-profiling")]
            self.profiling.end(Boundary::Checks, check_token);

            // Detailed per-instruction profiling deliberately uses the oracle path.
            #[cfg(all(feature = "experimental-thumb-blocks", not(feature = "cpu-profiling")))]
            if let Some((steps, fault)) = self.blocks.run(&mut self.cpu, &mut self.mem, end, count) {
                count -= steps;
                #[cfg(feature = "cpu-throughput")]
                {
                    thumb_steps += steps;
                    block_steps += steps;
                }
                if let Some(address) = fault {
                    finish!(Err(WieError::InvalidMemoryAccess(address)));
                }
                continue;
            }

            let mut arm32cpu_memory = self.mem.as_arm32cpu_memory();

            // Capture the mode before the instruction, since BX/SVC and other
            // instructions may change it. Counters stay in locals until exit.
            #[cfg(feature = "cpu-throughput")]
            let was_thumb = self.cpu.thumb_mode();
            let decoded = self.cpu.step(&mut arm32cpu_memory);
            #[cfg(feature = "cpu-throughput")]
            if was_thumb {
                thumb_steps += 1;
            } else {
                arm_steps += 1;
            }
            if !decoded {
                finish!(Err(WieError::FatalError("Undefined instruction".into())));
            }
            count -= 1;

            if let Some(x) = arm32cpu_memory.memory_error() {
                finish!(Err(WieError::InvalidMemoryAccess(x)));
            }
        }
    }

    fn reg_write(&mut self, reg: ArmRegister, value: u32) {
        if reg == ArmRegister::PC && value % 2 == 1 {
            self.cpu.reg_set(Mode::User, reg.into_armv4t(), value - 1);

            let cpsr = self.cpu.reg_get(Mode::User, reg::CPSR);
            self.cpu.reg_set(Mode::User, reg::CPSR, cpsr | (1 << 5)); // T bit

            return;
        }
        self.cpu.reg_set(Mode::User, reg.into_armv4t(), value);
    }

    fn reg_read(&self, reg: ArmRegister) -> u32 {
        self.cpu.reg_get(Mode::User, reg.into_armv4t())
    }

    fn mem_map(&mut self, address: u32, size: usize, _permission: MemoryPermission) {
        self.mem.map(address, size);
    }

    fn mem_write(&mut self, address: u32, data: &[u8]) -> Result<()> {
        #[cfg(feature = "cpu-profiling")]
        let token = self.profiling.begin(Boundary::HostWrite);
        let result = self.mem.write_range(address, data);
        #[cfg(feature = "cpu-profiling")]
        {
            if self.profiling.mode != ProfileMode::Off {
                self.profiling.stats.host_write_bytes += data.len() as u64;
            }
            self.profiling.end(Boundary::HostWrite, token);
        }
        result
    }

    fn mem_read(&mut self, address: u32, size: usize, result: &mut [u8]) -> Result<usize> {
        #[cfg(feature = "cpu-profiling")]
        let token = self.profiling.begin(Boundary::HostRead);
        let result = self.mem.read_range(address, size, result);
        #[cfg(feature = "cpu-profiling")]
        {
            if self.profiling.mode != ProfileMode::Off {
                self.profiling.stats.host_read_bytes += size as u64;
            }
            self.profiling.end(Boundary::HostRead, token);
        }
        result
    }

    fn is_mapped(&self, address: u32, size: usize) -> bool {
        self.mem.is_mapped(address, size)
    }
}

impl ArmRegister {
    fn into_armv4t(self) -> u8 {
        match self {
            ArmRegister::R0 => 0,
            ArmRegister::R1 => 1,
            ArmRegister::R2 => 2,
            ArmRegister::R3 => 3,
            ArmRegister::R4 => 4,
            ArmRegister::R5 => 5,
            ArmRegister::R6 => 6,
            ArmRegister::R7 => 7,
            ArmRegister::R8 => 8,
            ArmRegister::SB => 9,
            ArmRegister::SL => 10,
            ArmRegister::FP => 11,
            ArmRegister::IP => 12,
            ArmRegister::SP => reg::SP,
            ArmRegister::LR => reg::LR,
            ArmRegister::PC => reg::PC,
            ArmRegister::Cpsr => reg::CPSR,
        }
    }
}

const TOTAL_MEMORY: u64 = 0x100000000;
const PAGE_SIZE: usize = 0x10000;
const PAGE_MASK: u32 = (PAGE_SIZE - 1) as _;
const PAGE_COUNT: usize = (TOTAL_MEMORY / PAGE_SIZE as u64) as usize;
type PageTable = [Option<Box<[u8; PAGE_SIZE]>>; PAGE_COUNT];

struct EmulatedMemory {
    pages: Box<PageTable>,
}

impl EmulatedMemory {
    fn new() -> Self {
        Self {
            // Allocate on the heap, then preserve the fixed address-space size in
            // the type so guest page indices need no dynamic length check.
            pages: vec![None; PAGE_COUNT].into_boxed_slice().try_into().ok().expect("fixed page count"),
        }
    }

    fn as_arm32cpu_memory(&mut self) -> Arm32CpuMemory<'_> {
        Arm32CpuMemory::new(self)
    }

    fn map(&mut self, address: u32, size: usize) {
        let page_start = address & !PAGE_MASK;
        let page_end = (address + size as u32 + PAGE_MASK) & !PAGE_MASK;

        for page in (page_start..page_end).step_by(PAGE_SIZE) {
            let page_data = &mut self.pages[page as usize / PAGE_SIZE];
            if page_data.is_none() {
                *page_data = Some(Box::new([0; PAGE_SIZE]));
            }
        }
    }

    fn read_range(&self, address: u32, size: usize, result: &mut [u8]) -> Result<usize> {
        let mut remaining_size = size;
        let mut current_address = address;

        while remaining_size > 0 {
            let page_address = current_address & !PAGE_MASK;
            let page_data = self.pages[page_address as usize / PAGE_SIZE]
                .as_ref()
                .ok_or(WieError::InvalidMemoryAccess(current_address))?;
            let offset = (current_address - page_address) as usize;
            let available_bytes = (PAGE_SIZE - offset).min(remaining_size);

            result[size - remaining_size..size - remaining_size + available_bytes].copy_from_slice(&page_data[offset..offset + available_bytes]);
            remaining_size -= available_bytes;
            current_address += available_bytes as u32;
        }

        Ok(size)
    }

    fn write_range(&mut self, address: u32, data: &[u8]) -> Result<()> {
        let mut current_address = address;
        let mut data_index = 0;

        while data_index < data.len() {
            let page_address = current_address & !PAGE_MASK;
            let page_data = self.pages[page_address as usize / PAGE_SIZE]
                .as_mut()
                .ok_or(WieError::InvalidMemoryAccess(current_address))?;
            let offset = (current_address - page_address) as usize;
            let available_bytes = (PAGE_SIZE - offset).min(data.len() - data_index);

            page_data[offset..offset + available_bytes].copy_from_slice(&data[data_index..data_index + available_bytes]);
            data_index += available_bytes;
            current_address += available_bytes as u32;
        }

        Ok(())
    }

    fn is_mapped(&self, address: u32, size: usize) -> bool {
        let page_start = address & !PAGE_MASK;
        let page_end = (address + size as u32 + PAGE_MASK) & !PAGE_MASK;

        if self.pages[page_start as usize / PAGE_SIZE].is_none() {
            return false;
        }

        for page in (page_start..page_end).step_by(PAGE_SIZE) {
            if self.pages[page as usize / PAGE_SIZE].is_none() {
                return false;
            }
        }

        true
    }
}

struct Arm32CpuMemory<'a> {
    emulated_memory: &'a mut EmulatedMemory,
    memory_error: RefCell<Option<u32>>,
}

impl<'a> Arm32CpuMemory<'a> {
    fn new(emulated_memory: &'a mut EmulatedMemory) -> Self {
        Self {
            emulated_memory,
            memory_error: RefCell::new(None),
        }
    }

    fn memory_error(&self) -> Option<u32> {
        *self.memory_error.borrow()
    }

    fn get_page(&mut self, addr: u32) -> Option<&mut [u8; PAGE_SIZE]> {
        let page_address = addr & !PAGE_MASK;
        let page_data = self.emulated_memory.pages[page_address as usize / PAGE_SIZE].as_mut();

        if let Some(x) = page_data {
            Some(x)
        } else {
            *self.memory_error.borrow_mut() = Some(addr);
            None
        }
    }
}

impl Memory for Arm32CpuMemory<'_> {
    fn r8(&mut self, addr: u32) -> u8 {
        let offset = addr & PAGE_MASK;

        let page = self.get_page(addr);
        if page.is_none() {
            return 0;
        }

        let data = page.unwrap();

        data[offset as usize]
    }

    fn r16(&mut self, addr: u32) -> u16 {
        let offset = addr & PAGE_MASK;

        let page = self.get_page(addr);
        if page.is_none() {
            return 0;
        }

        let data = page.unwrap();

        (data[offset as usize] as u16) | ((data[offset as usize + 1] as u16) << 8)
    }

    fn r32(&mut self, addr: u32) -> u32 {
        let offset = addr & PAGE_MASK;

        let page = self.get_page(addr);
        if page.is_none() {
            return 0;
        }

        let data = page.unwrap();
        (data[offset as usize] as u32)
            | ((data[offset as usize + 1] as u32) << 8)
            | ((data[offset as usize + 2] as u32) << 16)
            | ((data[offset as usize + 3] as u32) << 24)
    }

    fn w8(&mut self, addr: u32, val: u8) {
        let offset = addr & PAGE_MASK;

        let page = self.get_page(addr);
        if page.is_none() {
            return;
        }

        let data = page.unwrap();

        data[offset as usize] = val;
    }

    fn w16(&mut self, addr: u32, val: u16) {
        let offset = addr & PAGE_MASK;

        let page = self.get_page(addr);
        if page.is_none() {
            return;
        }

        let data = page.unwrap();

        data[offset as usize] = val as u8;
        data[offset as usize + 1] = (val >> 8) as u8;
    }

    fn w32(&mut self, addr: u32, val: u32) {
        let offset = addr & PAGE_MASK;

        let page = self.get_page(addr);
        if page.is_none() {
            return;
        }

        let data = page.unwrap();

        data[offset as usize] = val as u8;
        data[offset as usize + 1] = (val >> 8) as u8;
        data[offset as usize + 2] = (val >> 16) as u8;
        data[offset as usize + 3] = (val >> 24) as u8;
    }
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;
    use core::mem::size_of;

    use arm32_cpu::Memory;

    use super::EmulatedMemory;

    #[test]
    fn memory_fault_slot_preserves_last_fault_until_adapter_is_recreated() {
        let mut memory = EmulatedMemory::new();
        memory.map(0x10000, 0x10000);
        {
            let mut adapter = memory.as_arm32cpu_memory();
            assert_eq!(adapter.memory_error(), None);
            assert_eq!(adapter.r32(0x20000), 0);
            assert_eq!(adapter.memory_error(), Some(0x20000));
            adapter.w32(0x10000, 123);
            assert_eq!(adapter.r32(0x10000), 123);
            assert_eq!(adapter.memory_error(), Some(0x20000));
            adapter.w16(0x30000, 456);
            assert_eq!(adapter.memory_error(), Some(0x30000));
        }
        assert_eq!(memory.as_arm32cpu_memory().memory_error(), None);
    }

    #[test]
    fn page_table_is_heap_allocated() {
        assert_eq!(size_of::<EmulatedMemory>(), size_of::<Box<super::PageTable>>());
    }

    #[test]
    fn page_table_covers_both_ends_of_guest_address_space() {
        let mut memory = EmulatedMemory::new();
        assert_eq!(memory.pages.len(), 65536);
        memory.pages[0] = Some(Box::new([0; super::PAGE_SIZE]));
        memory.pages[65535] = Some(Box::new([0; super::PAGE_SIZE]));
        let mut adapter = memory.as_arm32cpu_memory();
        adapter.w32(0, 0x12345678);
        adapter.w32(0xffff_fffc, 0xabcdef01);
        assert_eq!(adapter.r32(0), 0x12345678);
        assert_eq!(adapter.r32(0xffff_fffc), 0xabcdef01);
        assert_eq!(adapter.r8(0xffff_ffff), 0xab);
        assert_eq!(adapter.memory_error(), None);
        assert_eq!(adapter.r16(0x0001_0000), 0);
        assert_eq!(adapter.memory_error(), Some(0x0001_0000));
    }

    #[test]
    fn word_read_matches_byte_reference_at_every_in_page_offset() {
        let mut memory = EmulatedMemory::new();
        memory.map(0x10000, super::PAGE_SIZE);
        let data: alloc::vec::Vec<u8> = (0..super::PAGE_SIZE).map(|i| (i ^ (i >> 8)) as u8).collect();
        memory.write_range(0x10000, &data).unwrap();
        let mut adapter = memory.as_arm32cpu_memory();
        for offset in 0..=super::PAGE_SIZE - 4 {
            let expected =
                (data[offset] as u32) | ((data[offset + 1] as u32) << 8) | ((data[offset + 2] as u32) << 16) | ((data[offset + 3] as u32) << 24);
            assert_eq!(adapter.r32(0x10000 + offset as u32), expected);
        }
        assert_eq!(adapter.memory_error(), None);
        assert_eq!(adapter.r32(0x30001), 0);
        assert_eq!(adapter.memory_error(), Some(0x30001));
    }

    #[test]
    #[should_panic]
    fn raw_word_read_still_rejects_crossing_a_mapped_page() {
        let mut memory = EmulatedMemory::new();
        memory.map(0x10000, super::PAGE_SIZE * 2);
        // CPU accesses are aligned by AlignmentWrapper. The private raw adapter
        // must retain its existing rejection of an unaligned cross-page read.
        memory.as_arm32cpu_memory().r32(0x1fffd);
    }

    #[test]
    fn test_memory_basic() {
        let mut memory = EmulatedMemory::new();

        memory.map(0x10000, 0x1000);
        memory.map(0x11000, 0x1000);
        memory.map(0x20000, 0x10000);

        memory.write_range(0x10000, &[123; 0x1000]).unwrap();

        let mut buf = [0; 0x1000];
        memory.read_range(0x10000, 0x1000, &mut buf).unwrap();
        assert_eq!(buf, [123; 0x1000]);

        memory.write_range(0x10900, &[100; 0x1000]).unwrap();

        memory.read_range(0x10900, 0x1000, &mut buf).unwrap();
        assert_eq!(buf, [100; 0x1000]);

        let mut arm32cpu_memory = memory.as_arm32cpu_memory();

        let r8 = arm32cpu_memory.r8(0x10000);
        assert_eq!(r8, 123);

        let r16 = arm32cpu_memory.r16(0x10000);
        assert_eq!(r16, 123 | (123 << 8));

        let r32 = arm32cpu_memory.r32(0x10000);
        assert_eq!(r32, 123 | (123 << 8) | (123 << 16) | (123 << 24));

        arm32cpu_memory.w8(0x10000, 12);
        let r8 = arm32cpu_memory.r8(0x10000);
        assert_eq!(r8, 12);

        arm32cpu_memory.w16(0x10000, 0x1234);
        let r16 = arm32cpu_memory.r16(0x10000);
        assert_eq!(r16, 0x1234);

        arm32cpu_memory.w32(0x10000, 0x12345678);
        let r32 = arm32cpu_memory.r32(0x10000);
        assert_eq!(r32, 0x12345678);
    }

    #[test]
    fn test_memory_unmapped_read() {
        let mut memory = EmulatedMemory::new();

        memory.map(0x10000, 0x10000);

        let mut buf = [0; 0x1000];
        assert!(memory.read_range(0x1f500, 0x1000, &mut buf).is_err());
    }

    #[test]
    fn test_memory_unmapped_write() {
        let mut memory = EmulatedMemory::new();

        memory.map(0x10000, 0x10000);

        assert!(memory.write_range(0x1f500, &[12; 0x1000]).is_err());
    }

    #[cfg(feature = "cpu-profiling")]
    mod profiling_regressions {
        use super::super::*;
        use core::sync::atomic::{AtomicU64, Ordering};

        static TICKS: AtomicU64 = AtomicU64::new(0);
        fn clock() -> u64 {
            TICKS.fetch_add(10, Ordering::Relaxed)
        }
        fn panic_clock() -> u64 {
            panic!("untimed profiling read the host clock")
        }

        #[derive(Clone, Copy, Debug)]
        enum Exit {
            End,
            Count,
            Svc,
            DataFault,
            FetchFault,
            Undefined,
            InvalidPc,
            ZeroCount,
            ImmediateEnd,
            InvalidSvc,
        }
        #[derive(Debug, PartialEq, Eq)]
        enum Outcome {
            End,
            Count,
            Svc(u32, u32, u32),
            Error(alloc::string::String),
        }
        fn outcome(result: Result<EngineRunResult>) -> Outcome {
            match result {
                Ok(EngineRunResult::End) => Outcome::End,
                Ok(EngineRunResult::CountExhausted) => Outcome::Count,
                Ok(EngineRunResult::Svc { category, lr, spsr }) => Outcome::Svc(category, lr, spsr),
                Err(error) => Outcome::Error(format!("{error:?}")),
            }
        }
        fn prepare(case: Exit, mode: ProfileMode) -> (Arm32CpuEngine, u32, u32) {
            let mut engine = Arm32CpuEngine::new();
            engine.mem_map(0x10000, PAGE_SIZE, MemoryPermission::ReadWriteExecute);
            engine.mem_write(0x10000, &[0x07, 0x20, 0x08, 0x60, 0x05, 0xdf]).unwrap(); // MOV r0,7; STR r0,[r1]; SVC 5
            engine.reg_write(ArmRegister::Cpsr, 0x30);
            engine.reg_write(ArmRegister::PC, 0x10001);
            engine.reg_write(ArmRegister::R1, 0x11000);
            let (end, count) = match case {
                Exit::End => (0x10004, 20),
                Exit::Count => (0x30000, 2),
                Exit::Svc => (0x30000, 20),
                Exit::DataFault => {
                    engine.reg_write(ArmRegister::R1, 0x20000);
                    (0x30000, 20)
                }
                Exit::FetchFault => {
                    engine.reg_write(ArmRegister::PC, 0x20001);
                    (0x30000, 20)
                }
                Exit::Undefined => {
                    engine.mem_write(0x10000, &[0x00, 0xe8]).unwrap();
                    (0x30000, 20)
                }
                Exit::InvalidPc => {
                    engine.reg_write(ArmRegister::PC, 0x901);
                    (0x30000, 20)
                }
                Exit::ZeroCount => (0x30000, 0),
                Exit::ImmediateEnd => (0x10000, 0),
                Exit::InvalidSvc => {
                    engine.cpu.reg_set(Mode::Supervisor, reg::LR, 0x10002);
                    engine.reg_write(ArmRegister::Cpsr, 0x13);
                    engine.reg_write(ArmRegister::PC, 8);
                    (0x30000, 20)
                }
            };
            engine.set_profiling(mode, 1, 11, if mode == ProfileMode::Sampled { clock } else { panic_clock });
            (engine, end, count)
        }

        #[test]
        fn all_run_exit_paths_preserve_architecture_and_report_exact_counts() {
            for case in [
                Exit::End,
                Exit::Count,
                Exit::Svc,
                Exit::DataFault,
                Exit::FetchFault,
                Exit::Undefined,
                Exit::InvalidPc,
                Exit::ZeroCount,
                Exit::ImmediateEnd,
                Exit::InvalidSvc,
            ] {
                let (mut reference, end, count) = prepare(case, ProfileMode::Off);
                let expected = outcome(reference.run(end, count));
                let (attempts, checks, returns, exhausted, svc, errors) = match case {
                    Exit::End => (2, 3, 1, 0, 0, 0),
                    Exit::Count => (2, 3, 0, 1, 0, 0),
                    Exit::Svc => (3, 4, 0, 0, 1, 0),
                    Exit::DataFault => (2, 2, 0, 0, 0, 1),
                    Exit::FetchFault | Exit::Undefined => (1, 1, 0, 0, 0, 1),
                    Exit::InvalidPc | Exit::InvalidSvc => (0, 1, 0, 0, 0, 1),
                    Exit::ZeroCount => (0, 1, 0, 1, 0, 0),
                    Exit::ImmediateEnd => (0, 1, 1, 0, 0, 0),
                };
                let off = reference.profiling_snapshot();
                assert_eq!(off.cpu.instruction_attempts, 0);
                assert_eq!(off.wrapper.run_calls, 0);
                assert_eq!(off.wrapper.clock_reads, 0);
                for mode in [ProfileMode::Counts, ProfileMode::Sampled] {
                    let (mut engine, end, count) = prepare(case, mode);
                    assert_eq!(outcome(engine.run(end, count)), expected, "{case:?}");
                    assert_eq!(engine.cpu, reference.cpu, "{case:?}");
                    assert_eq!(engine.mem.pages, reference.mem.pages, "{case:?}");
                    let snapshot = engine.profiling_snapshot();
                    assert_eq!(snapshot.cpu.instruction_attempts, attempts, "{case:?}");
                    let w = snapshot.wrapper;
                    assert_eq!(
                        (
                            w.run_calls,
                            w.boundary_checks.events,
                            w.function_returns,
                            w.count_exhaustions,
                            w.svc_exits,
                            w.errors
                        ),
                        (1, checks, returns, exhausted, svc, errors),
                        "{case:?}"
                    );
                    assert_eq!(w.run_calls, w.function_returns + w.count_exhaustions + w.svc_exits + w.errors);
                    if mode == ProfileMode::Counts {
                        assert_eq!((snapshot.cpu.clock_reads, w.clock_reads, w.run_inclusive_ns), (0, 0, 0));
                    } else {
                        assert!(w.clock_reads >= 2);
                        assert!(w.run_inclusive_ns > 0);
                        assert_eq!(w.boundary_checks.sampled_events, checks);
                    }
                }
            }
        }

        #[test]
        fn snapshot_and_reset_do_not_change_guest_state_or_read_the_clock() {
            let (mut engine, end, count) = prepare(Exit::Count, ProfileMode::Counts);
            assert!(matches!(engine.run(end, count).unwrap(), EngineRunResult::CountExhausted));
            let cpu = engine.cpu;
            let before = engine.profiling_snapshot();
            engine.reset_profiling();
            assert_eq!(engine.cpu, cpu);
            let reset = engine.profiling_snapshot();
            assert_eq!(reset.cpu.mode, before.cpu.mode);
            assert_eq!(reset.cpu.mean_interval, before.cpu.mean_interval);
            assert_eq!(reset.cpu.seed, before.cpu.seed);
            assert_eq!(reset.cpu.instruction_attempts, 0);
            assert_eq!(reset.wrapper.run_calls, 0);
            assert_eq!(reset.wrapper.clock_reads, 0);
        }
    }

    #[cfg(feature = "cpu-throughput")]
    mod throughput_regressions {
        use super::super::*;

        fn engine() -> Arm32CpuEngine {
            let mut engine = Arm32CpuEngine::new();
            engine.mem_map(0x10000, PAGE_SIZE, MemoryPermission::ReadWriteExecute);
            engine.reg_write(ArmRegister::Cpsr, 0x30);
            engine.reg_write(ArmRegister::PC, 0x10001);
            engine
        }

        fn check(engine: &Arm32CpuEngine, arm: u64, thumb: u64) {
            let snapshot = engine.throughput_snapshot();
            assert_eq!(snapshot.arm_instructions, arm);
            assert_eq!(snapshot.thumb_instructions, thumb);
            assert_eq!(snapshot.total_instructions, arm + thumb);
            assert_eq!(snapshot.full_profiling_compiled, cfg!(feature = "cpu-profiling"));
        }

        #[test]
        fn mixed_instruction_sets_flush_each_exit_and_reset_preserves_architecture() {
            let mut engine = engine();
            engine.reg_write(ArmRegister::Cpsr, 0x10);
            engine.reg_write(ArmRegister::PC, 0x10000);
            engine.reg_write(ArmRegister::R0, 0x10021);
            engine.reg_write(ArmRegister::R2, 0x11000);
            engine.reg_write(ArmRegister::R3, 0x10040);
            engine.reg_write(ArmRegister::R5, 0x20000);
            engine.mem_write(0x10000, &0xe12f_ff10u32.to_le_bytes()).unwrap(); // ARM BX r0
            engine.mem_write(0x10020, &[0x07, 0x21, 0x11, 0x60, 0x18, 0x47]).unwrap(); // Thumb MOV, STR, BX r3
            engine.mem_write(0x10040, &0xe284_4001u32.to_le_bytes()).unwrap(); // ARM ADD r4,r4,1
            engine.mem_write(0x10044, &0xe12f_ff15u32.to_le_bytes()).unwrap(); // ARM BX r5
            assert!(matches!(engine.run(0x20000, 2).unwrap(), EngineRunResult::CountExhausted));
            check(&engine, 1, 1);
            assert_eq!(engine.reg_read(ArmRegister::PC), 0x10022);
            assert!(matches!(engine.run(0x20000, 2).unwrap(), EngineRunResult::CountExhausted));
            check(&engine, 1, 3);
            assert_eq!(engine.reg_read(ArmRegister::PC), 0x10040);
            assert!(matches!(engine.run(0x20000, 10).unwrap(), EngineRunResult::End));
            check(&engine, 3, 3);
            assert_eq!(engine.reg_read(ArmRegister::R1), 7);
            assert_eq!(engine.reg_read(ArmRegister::R4), 1);
            assert!(!engine.cpu.thumb_mode());
            let mut stored = [0; 4];
            engine.mem_read(0x11000, 4, &mut stored).unwrap();
            assert_eq!(u32::from_le_bytes(stored), 7);
            let cpu = engine.cpu;
            engine.reset_throughput();
            assert_eq!(engine.cpu, cpu);
            check(&engine, 0, 0);
            assert!(matches!(engine.run(0x20000, 0).unwrap(), EngineRunResult::End));
            check(&engine, 0, 0);
            engine.mem_read(0x11000, 4, &mut stored).unwrap();
            assert_eq!(u32::from_le_bytes(stored), 7);
        }

        #[test]
        fn faults_software_exceptions_and_empty_exits_count_attempted_steps_once() {
            let mut svc = engine();
            svc.mem_write(0x10000, &[0x06, 0xdf]).unwrap();
            match svc.run(0x20000, 10).unwrap() {
                EngineRunResult::Svc { category, lr, spsr } => {
                    assert_eq!((category, lr), (6, 0x10002));
                    assert_ne!(spsr & 0x20, 0);
                }
                _ => panic!("expected SVC"),
            }
            check(&svc, 0, 1);
            assert!(!svc.cpu.thumb_mode());
            assert!(matches!(svc.run(0x20000, 10).unwrap(), EngineRunResult::Svc { .. }));
            check(&svc, 0, 1); // Existing vector boundary is not another step.
            svc.cpu.reg_set(Mode::Supervisor, reg::LR, 1);
            assert!(matches!(svc.run(0x20000, 10), Err(WieError::InvalidMemoryAccess(1))));
            check(&svc, 0, 1);

            let mut data = engine();
            data.reg_write(ArmRegister::R1, 0x20000);
            data.mem_write(0x10000, &[0x08, 0x68]).unwrap(); // LDR r0,[r1] unmapped
            assert!(matches!(data.run(0x30000, 10), Err(WieError::InvalidMemoryAccess(0x20000))));
            check(&data, 0, 1);
            assert_eq!(data.reg_read(ArmRegister::PC), 0x10002);

            let mut fetch = engine();
            fetch.reg_write(ArmRegister::PC, 0x20001);
            assert!(matches!(fetch.run(0x30000, 10), Err(WieError::InvalidMemoryAccess(0x20000))));
            check(&fetch, 0, 1);

            for (thumb, bytes) in [(true, &[0x00, 0xe8][..]), (false, &[0x00, 0x00, 0x00, 0xec][..])] {
                let mut invalid = engine();
                invalid.reg_write(ArmRegister::Cpsr, if thumb { 0x30 } else { 0x10 });
                invalid.mem_write(0x10000, bytes).unwrap();
                assert!(matches!(invalid.run(0x20000, 10), Err(WieError::FatalError(_))));
                check(&invalid, u64::from(!thumb), u64::from(thumb));
            }

            let mut empty = engine();
            assert!(matches!(empty.run(0x20000, 0).unwrap(), EngineRunResult::CountExhausted));
            check(&empty, 0, 0);
            assert!(matches!(empty.run(0x10000, 0).unwrap(), EngineRunResult::End));
            check(&empty, 0, 0);
            empty.reg_write(ArmRegister::PC, 0x901);
            assert!(matches!(empty.run(0x20000, 10), Err(WieError::InvalidMemoryAccess(0x900))));
            check(&empty, 0, 0);
        }
    }
}
