use alloc::{boxed::Box, vec, vec::Vec};

use wipi_types::wipic::{WIPICIndirectPtr, WIPICWord};

use wie_backend::{Instant, System};
use wie_util::{ByteRead, ByteWrite, Result};

use crate::{
    WIPICMethodBody,
    method::{ParamConverter, ResultConverter},
};

#[async_trait::async_trait]
pub trait WIPICContext: ByteRead + ByteWrite + Send + Sync {
    #[cfg(feature = "cpu-transcript-capture")]
    fn transcript_begin(&mut self, _callback: u64) {}
    #[cfg(feature = "cpu-transcript-capture")]
    fn transcript_end(&mut self, _callback: u64, _ok: bool) {}
    /// Carrier ABI: the image prefix contains framebuffer memory IDs instead
    /// of embedding framebuffer descriptors. The pointed-to data stays guest-owned.
    fn indirect_image_framebuffers(&self) -> bool {
        false
    }
    /// Guest-owned native IME storage and optional frontend mode selection.
    async fn input_state(&mut self) -> Result<u32> {
        Err(wie_util::WieError::Unimplemented("Native input state".into()))
    }
    async fn input_selection(&mut self) -> Result<Option<(bool, i32)>> {
        Ok(None)
    }
    fn input_press_event(&self) -> u32 {
        2
    }
    fn total_memory(&self) -> u32;
    fn free_memory(&self) -> Result<u32>;
    fn alloc_raw(&mut self, size: WIPICWord) -> Result<WIPICWord>;
    fn alloc(&mut self, size: WIPICWord) -> Result<WIPICIndirectPtr>;
    fn free(&mut self, memory: WIPICIndirectPtr) -> Result<()>;
    fn free_raw(&mut self, address: WIPICWord, size: WIPICWord) -> Result<()>;
    fn data_ptr(&self, memory: WIPICIndirectPtr) -> Result<WIPICWord>;
    async fn call_function(&mut self, address: WIPICWord, args: &[WIPICWord]) -> Result<WIPICWord>;
    fn system(&mut self) -> &mut System;
    fn spawn(&mut self, callback: WIPICMethodBody) -> Result<()>;
    async fn get_resource_size(&self, name: &str) -> Result<Option<usize>>;
    async fn read_resource(&self, name: &str) -> Result<Vec<u8>>;
    fn set_timer(&mut self, due: Instant, registration: WIPICWord, callback: WIPICMethodBody);
}

pub struct WIPICResult {
    pub results: Vec<WIPICWord>,
}

impl ParamConverter<WIPICWord> for WIPICWord {
    fn convert(_: &mut dyn WIPICContext, raw: WIPICWord) -> WIPICWord {
        raw
    }
}

impl ParamConverter<WIPICIndirectPtr> for WIPICIndirectPtr {
    fn convert(_: &mut dyn WIPICContext, raw: WIPICWord) -> WIPICIndirectPtr {
        WIPICIndirectPtr(raw)
    }
}

impl ParamConverter<i32> for i32 {
    fn convert(_: &mut dyn WIPICContext, raw: WIPICWord) -> i32 {
        raw as _
    }
}

impl ResultConverter<u64> for u64 {
    fn convert(_: &mut dyn WIPICContext, result: u64) -> WIPICResult {
        WIPICResult {
            results: vec![result as u32, (result >> 32) as u32],
        }
    }
}

impl ResultConverter<WIPICWord> for WIPICWord {
    fn convert(_: &mut dyn WIPICContext, result: WIPICWord) -> WIPICResult {
        WIPICResult { results: vec![result] }
    }
}

impl ResultConverter<WIPICIndirectPtr> for WIPICIndirectPtr {
    fn convert(_: &mut dyn WIPICContext, result: WIPICIndirectPtr) -> WIPICResult {
        WIPICResult { results: vec![result.0] }
    }
}

impl ResultConverter<i32> for i32 {
    fn convert(_: &mut dyn WIPICContext, result: i32) -> WIPICResult {
        WIPICResult { results: vec![result as _] }
    }
}

impl ResultConverter<()> for () {
    fn convert(_: &mut dyn WIPICContext, _: ()) -> WIPICResult {
        WIPICResult { results: Vec::new() }
    }
}

#[cfg(test)]
pub mod test {
    use alloc::{boxed::Box, format, string::String, vec::Vec};

    use wipi_types::wipic::{WIPICIndirectPtr, WIPICWord};

    use wie_backend::{Instant, System};
    use wie_util::{ByteRead, ByteWrite, Result, WieError};

    use super::{WIPICContext, WIPICMethodBody};

    const TEST_MEMORY_SIZE: usize = 0x20000;
    const TEST_ALLOC_START: usize = 0x10000;

    pub struct TestContext {
        memory: [u8; TEST_MEMORY_SIZE],
        bytes_read: core::sync::atomic::AtomicUsize,
        bytes_written: usize,
        last_alloc: usize,
        system: Option<System>,
        resources: Vec<(String, Vec<u8>)>,
        pub timers: alloc::collections::VecDeque<(Instant, u32, WIPICMethodBody)>,
        pub raw_freed: Vec<u32>,
        pub calls: Vec<(u32, Vec<u32>)>,
        pub spawned: Vec<WIPICMethodBody>,
        pub freed: Vec<u32>,
        pub indirect_images: bool,
        screen_framebuffer: [u8; 4],
        pub pixel_callback: Option<fn(&mut Self, u32, &[u32]) -> Result<u32>>,
    }

    impl TestContext {
        #[allow(clippy::new_without_default)]
        pub fn new() -> Self {
            Self {
                memory: [0; TEST_MEMORY_SIZE],
                bytes_read: core::sync::atomic::AtomicUsize::new(0),
                bytes_written: 0,
                last_alloc: TEST_ALLOC_START,
                system: None,
                resources: Vec::new(),
                timers: alloc::collections::VecDeque::new(),
                raw_freed: Vec::new(),
                calls: Vec::new(),
                spawned: Vec::new(),
                freed: Vec::new(),
                indirect_images: false,
                screen_framebuffer: [0; 4],
                pixel_callback: None,
            }
        }

        pub fn with_system(system: System) -> Self {
            Self {
                system: Some(system),
                ..Self::new()
            }
        }

        pub fn with_resource(mut self, name: &str, data: &[u8]) -> Self {
            self.resources.push((String::from(name), data.to_vec()));
            self
        }

        pub fn reset_io_counts(&mut self) {
            self.bytes_read.store(0, core::sync::atomic::Ordering::Relaxed);
            self.bytes_written = 0;
        }

        pub fn io_counts(&self) -> (usize, usize) {
            (self.bytes_read.load(core::sync::atomic::Ordering::Relaxed), self.bytes_written)
        }
    }

    #[async_trait::async_trait]
    impl WIPICContext for TestContext {
        async fn input_state(&mut self) -> Result<u32> {
            Ok(0x8000)
        }

        fn indirect_image_framebuffers(&self) -> bool {
            self.indirect_images
        }
        fn total_memory(&self) -> u32 {
            (TEST_MEMORY_SIZE - TEST_ALLOC_START) as u32
        }
        fn free_memory(&self) -> Result<u32> {
            Ok(TEST_MEMORY_SIZE.saturating_sub(self.last_alloc) as u32)
        }

        fn alloc_raw(&mut self, size: WIPICWord) -> Result<WIPICWord> {
            let address = self.last_alloc;
            let end = address
                .checked_add(size as usize)
                .filter(|&end| end <= TEST_MEMORY_SIZE)
                .ok_or(WieError::AllocationFailure)?;
            self.last_alloc = end;

            Ok(address as WIPICWord)
        }

        fn alloc(&mut self, size: WIPICWord) -> Result<WIPICIndirectPtr> {
            Ok(WIPICIndirectPtr(Self::alloc_raw(self, size)?))
        }

        fn free(&mut self, memory: WIPICIndirectPtr) -> Result<()> {
            self.freed.push(memory.0);
            Ok(())
        }

        fn free_raw(&mut self, address: WIPICWord, _size: WIPICWord) -> Result<()> {
            self.raw_freed.push(address);
            Ok(())
        }

        fn data_ptr(&self, memory: WIPICIndirectPtr) -> Result<WIPICWord> {
            Ok(memory.0)
        }

        async fn call_function(&mut self, address: WIPICWord, args: &[WIPICWord]) -> Result<WIPICWord> {
            self.calls.push((address, args.to_vec()));
            if let Some(callback) = self.pixel_callback {
                return callback(self, address, args);
            }
            Ok(0)
        }

        fn system(&mut self) -> &mut System {
            self.system.as_mut().unwrap()
        }

        fn spawn(&mut self, callback: WIPICMethodBody) -> Result<()> {
            self.spawned.push(callback);
            Ok(())
        }

        async fn get_resource_size(&self, name: &str) -> Result<Option<usize>> {
            Ok(self.resources.iter().find(|(x, _)| x == name).map(|(_, data)| data.len()))
        }

        async fn read_resource(&self, name: &str) -> Result<Vec<u8>> {
            self.resources
                .iter()
                .find(|(x, _)| x == name)
                .map(|(_, data)| data.clone())
                .ok_or_else(|| WieError::FatalError(format!("Missing test resource: {name}")))
        }

        fn set_timer(&mut self, due: Instant, registration: WIPICWord, callback: WIPICMethodBody) {
            self.timers.push_back((due, registration, callback));
        }
    }

    impl ByteWrite for TestContext {
        fn write_bytes(&mut self, address: u32, data: &[u8]) -> wie_util::Result<()> {
            self.bytes_written += data.len();
            if address == 0x7fff1000 && data.len() == 4 {
                self.screen_framebuffer.copy_from_slice(data);
                return Ok(());
            }
            self.memory[address as usize..(address + data.len() as u32) as usize].copy_from_slice(data);

            Ok(())
        }
    }

    impl ByteRead for TestContext {
        fn read_bytes(&self, address: u32, result: &mut [u8]) -> wie_util::Result<usize> {
            self.bytes_read.fetch_add(result.len(), core::sync::atomic::Ordering::Relaxed);
            if address == 0x7fff1000 && result.len() == 4 {
                result.copy_from_slice(&self.screen_framebuffer);
                return Ok(4);
            }
            result.copy_from_slice(&self.memory[address as usize..(address as usize + result.len())]);

            Ok(result.len())
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::{ResultConverter, test::TestContext};

    #[test]
    fn convert_u64_splits_low_and_high_words() {
        let mut context = TestContext::new();

        let result = <u64 as ResultConverter<u64>>::convert(&mut context, 0x1122_3344_5566_7788);

        assert_eq!(result.results, vec![0x5566_7788, 0x1122_3344]);
    }
}
