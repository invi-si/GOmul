use alloc::{boxed::Box, format, vec, vec::Vec};

use jvm::{
    Jvm,
    runtime::{JavaIoInputStream, JavaLangClassLoader},
};
use wipi_types::wipic::{WIPICIndirectPtr, WIPICWord};

use wie_backend::{AsyncCallable, Event, Instant, System};
use wie_core_arm::{Allocator, ArmCore};
use wie_util::{ByteRead, ByteWrite, Result, WieError, read_generic, write_generic};
use wie_wipi_c::{WIPICContext, WIPICMethodBody};

// mostly same as ktf's one, can we merge those?
#[derive(Clone)]
pub struct LgtWIPICContext {
    core: ArmCore,
    system: System,
    jvm: Jvm,
}

impl LgtWIPICContext {
    pub fn new(core: ArmCore, system: System, jvm: Jvm) -> Self {
        Self { core, system, jvm }
    }
}

#[async_trait::async_trait]
impl WIPICContext for LgtWIPICContext {
    async fn input_state(&mut self) -> Result<u32> {
        let address: i32 = match self.jvm.get_static_field("javax/microedition/lcdui/Display", "nativeInput", "I").await {
            Ok(p) => p,
            Err(e) => return Err(wie_jvm_support::JvmSupport::to_wie_err(&self.jvm, e).await),
        };
        if address != 0 {
            return Ok(address as u32);
        }
        let address = Allocator::alloc(&mut self.core, wie_wipi_c::api::input::STATE_SIZE)?;
        self.core
            .write_bytes(address, &alloc::vec![0; wie_wipi_c::api::input::STATE_SIZE as usize])?;
        if let Err(e) = self
            .jvm
            .put_static_field("javax/microedition/lcdui/Display", "nativeInput", "I", address as i32)
            .await
        {
            return Err(wie_jvm_support::JvmSupport::to_wie_err(&self.jvm, e).await);
        }
        Ok(address)
    }
    async fn input_selection(&mut self) -> Result<Option<(bool, i32)>> {
        let result = async {
            let epoch: i32 = self
                .jvm
                .get_static_field("javax/microedition/lcdui/Display", "inputModeEpoch", "I")
                .await?;
            let korean: bool = self.jvm.get_static_field("javax/microedition/lcdui/Display", "koreanInput", "Z").await?;
            Ok::<_, jvm::JavaError>(if epoch == 0 { None } else { Some((korean, epoch)) })
        }
        .await;
        match result {
            Ok(p) => Ok(p),
            Err(e) => Err(wie_jvm_support::JvmSupport::to_wie_err(&self.jvm, e).await),
        }
    }

    fn input_press_event(&self) -> u32 {
        502
    }

    #[cfg(feature = "cpu-transcript-capture")]
    fn transcript_begin(&mut self, callback: u64) {
        self.core.transcript_begin(callback);
    }
    #[cfg(feature = "cpu-transcript-capture")]
    fn transcript_end(&mut self, callback: u64, ok: bool) {
        self.core.transcript_end(callback, ok);
    }

    fn total_memory(&self) -> u32 {
        Allocator::total_memory()
    }
    fn free_memory(&self) -> Result<u32> {
        Allocator::free_memory(&self.core)
    }

    fn alloc_raw(&mut self, size: WIPICWord) -> Result<WIPICWord> {
        Allocator::alloc(&mut self.core, size)
    }

    fn alloc(&mut self, size: WIPICWord) -> Result<WIPICIndirectPtr> {
        let address = Allocator::alloc(&mut self.core, size + size_of::<WIPICWord>() as WIPICWord)?;
        write_generic(&mut self.core, address, size)?;

        Ok(WIPICIndirectPtr(address + size_of::<WIPICWord>() as WIPICWord))
    }

    fn free(&mut self, memory: WIPICIndirectPtr) -> Result<()> {
        let base_address = memory.0 - size_of::<WIPICWord>() as WIPICWord;

        let size: WIPICWord = read_generic(&self.core, base_address)?;
        Allocator::free(&mut self.core, base_address, size + size_of::<WIPICWord>() as WIPICWord)
    }

    fn free_raw(&mut self, address: WIPICWord, size: WIPICWord) -> Result<()> {
        Allocator::free(&mut self.core, address, size)?;

        Ok(())
    }

    fn data_ptr(&self, memory: WIPICIndirectPtr) -> Result<WIPICWord> {
        Ok(memory.0)
    }

    fn system(&mut self) -> &mut System {
        &mut self.system
    }

    async fn call_function(&mut self, address: WIPICWord, args: &[WIPICWord]) -> Result<WIPICWord> {
        self.core.run_function(address, args).await
    }

    fn spawn(&mut self, callback: WIPICMethodBody) -> Result<()> {
        struct SpawnProxy {
            context: LgtWIPICContext,
            callback: WIPICMethodBody,
        }

        impl AsyncCallable<Result<()>> for SpawnProxy {
            async fn call(mut self) -> Result<()> {
                self.context.jvm.attach_thread(None).await.unwrap();
                self.callback.call(&mut self.context, Box::new([])).await?;
                self.context.jvm.detach_thread().unwrap();

                Ok(())
            }
        }

        self.system.spawn(SpawnProxy {
            context: self.clone(),
            callback,
        });

        Ok(())
    }

    async fn get_resource_size(&self, name: &str) -> Result<Option<usize>> {
        let class_loader = JavaLangClassLoader::get_system_class_loader(&self.jvm).await.unwrap();
        let stream = JavaLangClassLoader::get_resource_as_stream(&self.jvm, &class_loader, name).await.unwrap();

        if let Some(stream) = stream {
            let available: i32 = self
                .jvm
                .invoke_virtual(&stream, "java/io/InputStream", "available", "()I", ())
                .await
                .unwrap();
            return Ok(Some(available as _));
        }
        self.jvm.collect_garbage().unwrap();

        Ok(self.system.filesystem().size(name).await)
    }

    async fn read_resource(&self, name: &str) -> Result<Vec<u8>> {
        let class_loader = JavaLangClassLoader::get_system_class_loader(&self.jvm).await.unwrap();
        let stream = JavaLangClassLoader::get_resource_as_stream(&self.jvm, &class_loader, name).await.unwrap();

        if let Some(stream) = stream {
            return Ok(JavaIoInputStream::read_until_end(&self.jvm, &stream).await.unwrap());
        }

        let Some(size) = self.system.filesystem().size(name).await else {
            return Err(WieError::FatalError(format!("Missing resource: {name}")));
        };
        let mut data = vec![0; size];
        let read = self.system.filesystem().read(name, 0, size, &mut data).await.unwrap_or(0);
        data.truncate(read);

        self.jvm.collect_garbage().unwrap();

        Ok(data)
    }

    fn set_timer(&mut self, due: Instant, registration: WIPICWord, callback: WIPICMethodBody) {
        let context = self.clone();

        self.system().event_queue().push(Event::timer_checked(due, move || {
            let mut context = context.clone();

            async move {
                // Consume immediately before entry, without an intervening await.
                // This also checks registrations deferred outside the backend queue.
                if !wie_wipi_c::timer::begin(&mut context, registration)? {
                    return Ok(false);
                }
                callback.call(&mut context, Box::new([])).await?;
                Ok(true)
            }
        }))
    }
}

impl ByteRead for LgtWIPICContext {
    fn read_bytes(&self, address: WIPICWord, result: &mut [u8]) -> wie_util::Result<usize> {
        self.core.read_bytes(address, result)
    }
}

impl ByteWrite for LgtWIPICContext {
    fn write_bytes(&mut self, address: WIPICWord, data: &[u8]) -> wie_util::Result<()> {
        self.core.write_bytes(address, data)
    }
}
