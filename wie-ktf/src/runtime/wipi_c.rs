use alloc::{boxed::Box, format, vec};

use jvm::Jvm;
use wie_backend::System;
use wie_core_arm::{ArmCore, EmulatedFunction, EmulatedFunctionParam, ResultWriter, SvcId};
use wie_util::{Result, WieError};
use wie_wipi_c::{WIPICMethodBody, WIPICResult};

use crate::runtime::SVC_CATEGORY_WIPIC;
use crate::runtime::svc_ids::{WIPICGraphicsMethodId, WIPICKernelMethodId, WIPICTableId};

mod context;
pub mod interface;
mod method_table;
mod mx_memory;

pub(crate) use context::KtfWIPICContext;

struct WIPICMethodResult {
    result: WIPICResult,
}

impl ResultWriter<WIPICMethodResult> for WIPICMethodResult {
    fn write(self, core: &mut ArmCore, next_pc: u32) -> Result<()> {
        core.write_return_value(&self.result.results)?;
        core.set_next_pc(next_pc)?;

        Ok(())
    }
}

struct CMethodProxy {
    context: KtfWIPICContext,
    body: WIPICMethodBody,
}

#[async_trait::async_trait]
impl EmulatedFunction<(), WIPICMethodResult, ()> for CMethodProxy {
    async fn call(&self, core: &mut ArmCore, _: &mut ()) -> Result<WIPICMethodResult> {
        let a0 = u32::get(core, 0);
        let a1 = u32::get(core, 1);
        let a2 = u32::get(core, 2);
        let a3 = u32::get(core, 3);
        let a4 = u32::get(core, 4);
        let a5 = u32::get(core, 5);
        let a6 = u32::get(core, 6);
        let a7 = u32::get(core, 7);
        let a8 = u32::get(core, 8);

        let result = self
            .body
            .call(&mut self.context.clone(), vec![a0, a1, a2, a3, a4, a5, a6, a7, a8].into_boxed_slice())
            .await?;

        Ok(WIPICMethodResult { result })
    }
}

async fn handle_wipic_svc(core: &mut ArmCore, (system, jvm): &mut (System, Jvm), id: SvcId) -> Result<()> {
    let table_id = WIPICTableId::try_from(id.0 >> 16)?;
    let function_id = id.0 as u16;
    let (_, lr) = core.read_pc_lr()?;
    if table_id == WIPICTableId::Net && function_id == 34 {
        // Observed callers select a constant carrier endpoint and ignore the
        // result. Treat this as configuration; actual connections remain offline.
        let address = core.read_param(0)?;
        crate::runtime::KtfJvmSupport::set_network_endpoint(core, address)?;
        return core.set_next_pc(lr);
    }
    if table_id == WIPICTableId::Kernel && function_id == WIPICKernelMethodId::Reserved4 as u16 {
        let name = core.read_param(0)?;
        return mx_memory::query(core, name)?.write(core, lr);
    }
    if table_id == WIPICTableId::Kernel && function_id == WIPICKernelMethodId::Reserved1 as u16 {
        return interface::get_wipic_interfaces(core, &mut KtfWIPICContext::new(core.clone(), system.clone(), jvm.clone()))
            .await?
            .write(core, lr);
    }

    let body = method_table::get_method_body(table_id, function_id)
        .ok_or_else(|| WieError::FatalError(alloc::format!("Unknown KTF WIPIC SVC id {:#x}", id.0)))?;

    let args = [u32::get(core, 0), u32::get(core, 1), u32::get(core, 2), u32::get(core, 3)];
    if table_id == WIPICTableId::Kernel && function_id == WIPICKernelMethodId::Exit as u16 {
        tracing::debug!(target: "wie_frames", "KTF guest exit: caller={lr:#x}, code={:#x}\n{}", args[0], core.dump_reg_stack(crate::emulator::IMAGE_BASE));
    }
    if table_id == WIPICTableId::Graphics && function_id == WIPICGraphicsMethodId::Repaint as u16 {
        // C redraw notifications share the Java event queue. Mark the request
        // before posting it, even before the first native framebuffer flush.
        let context = KtfWIPICContext::new(core.clone(), system.clone(), jvm.clone());
        let framebuffer = wie_wipi_c::api::graphics::screen_framebuffer(&context)?;
        if let Err(error) = crate::runtime::KtfJvmSupport::mark_repaint_pending(jvm, framebuffer.map(|fb| fb.0)).await {
            return Err(wie_jvm_support::JvmSupport::to_wie_err(jvm, error).await);
        }
    }
    let result = EmulatedFunction::call(
        &CMethodProxy {
            context: KtfWIPICContext::new(core.clone(), system.clone(), jvm.clone()),
            body,
        },
        core,
        &mut (),
    )
    .await
    .inspect_err(|error| {
        tracing::error!(
            "KTF WIPIC failure: id={:#x}, caller={lr:#x}, args=[{:#x}, {:#x}, {:#x}, {:#x}]: {error}",
            id.0,
            args[0],
            args[1],
            args[2],
            args[3],
        );
    })?;
    if table_id == WIPICTableId::Graphics && function_id == WIPICGraphicsMethodId::FlushLcd as u16 {
        // Select direct framebuffer presentation from an actual successful flush,
        // not a main-class name. Some Clet wrappers render entirely through Java.
        if let Err(error) = crate::runtime::KtfJvmSupport::disable_midp_paint(jvm).await {
            return Err(wie_jvm_support::JvmSupport::to_wie_err(jvm, error).await);
        }
    }
    tracing::trace!(target: "ktf_svc", id = id.0, caller = lr, ?args, results = ?result.result.results, "KTF native return");
    result.write(core, lr)
}

pub fn register_wipic_svc_handler(core: &mut ArmCore, system: &System, jvm: &Jvm) -> Result<()> {
    mx_memory::register(core)?;
    core.register_svc_handler(SVC_CATEGORY_WIPIC, handle_wipic_svc, &(system.clone(), jvm.clone()))
}

/// Installed program access metadata is distinct from whether an emulated
/// service can currently connect. It does not grant access to host resources.
async fn get_access_level(context: &mut dyn wie_wipi_c::WIPICContext) -> Result<i32> {
    let filesystem = context.system().filesystem();
    let Some(size) = filesystem.size("__adf__").await else {
        return Ok(0);
    };
    let mut data = vec![0; size];
    let Some(count) = filesystem.read("__adf__", 0, size, &mut data).await else {
        return Ok(0);
    };
    Ok(crate::adf::KtfAdf::parse(&data[..count]).access_level.unwrap_or(0) as i32)
}

/// Each System is an isolated single-application installation. Enumerate its
/// real package, never the launcher's unrelated host-side game library.
async fn get_exec_names(
    context: &mut dyn wie_wipi_c::WIPICContext,
    program: u32,
    version: u32,
    vendor: u32,
    output: u32,
    capacity: i32,
) -> Result<i32> {
    let filesystem = context.system().filesystem().clone();
    let Some(size) = filesystem.size("__adf__").await else {
        return Ok(0);
    };
    let mut data = vec![0; size];
    let Some(count) = filesystem.read("__adf__", 0, size, &mut data).await else {
        return Ok(0);
    };
    let metadata = crate::adf::KtfAdf::parse(&data[..count]);
    let jar = format!("{}.jar", metadata.aid);
    if metadata.aid.is_empty() || !filesystem.exists(&jar).await {
        return Ok(0);
    }
    for (filter, installed) in [(program, &metadata.aid), (version, &metadata.version), (vendor, &metadata.vendor)] {
        if filter != 0 {
            let bytes = wie_util::read_null_terminated_string_bytes(context, filter)?;
            let requested = encoding_rs::EUC_KR.decode(&bytes).0;
            tracing::debug!(target: "wie_frames", "KTF GetExecNames filter: requested={requested:?}, installed={installed:?}");
            if requested != installed.as_str() {
                return Ok(0);
            }
        }
    }
    // KTF uses the installation AID as the program name, not its display title.
    // The executable identifier contains its application directory and JAR name.
    // It is an opaque launch identifier; this is not a host filesystem path.
    let executable = format!("/{}/{}", metadata.aid, jar);
    if capacity < 0 || (capacity as usize) < executable.len() + 1 {
        return Ok(-18); // M_E_SHORTBUF
    }
    tracing::debug!(target: "wie_frames", "KTF GetExecNames result: executable={executable:?}, capacity={capacity}");
    wie_util::write_null_terminated_string_bytes(context, output, executable.as_bytes())?;
    Ok(1)
}

#[cfg(test)]
mod component_tests {
    use super::register_wipic_svc_handler;
    use crate::runtime::{
        SVC_CATEGORY_WIPIC,
        java::jvm_support::{KtfJvmSupport, KtfJvmThreadContext},
    };
    use alloc::{boxed::Box, sync::Arc};
    use bytemuck::Zeroable;
    use core::sync::atomic::{AtomicBool, Ordering};
    use test_utils::TestPlatform;
    use wie_backend::{DefaultTaskRunner, System};
    use wie_core_arm::{Allocator, ArmCore};
    use wie_util::Result;
    #[test]
    fn component_context_pointer_and_date_time_use_native_svc_contract() -> Result<()> {
        let mut system = System::new(Box::new(TestPlatform::new()), "", "", DefaultTaskRunner);
        let mut system_clone = system.clone();
        let done = Arc::new(AtomicBool::new(false));
        let done_clone = done.clone();
        system.spawn(async move || {
            let mut core = ArmCore::new(false, None)?;
            Allocator::init(&mut core)?;
            let mut registers = core.save_context();
            registers.sp = Allocator::alloc(&mut core, 0x100)? + 0x100;
            core.restore_context(&registers);
            let thread = Allocator::alloc(&mut core, core::mem::size_of::<KtfJvmThreadContext>() as u32)?;
            wie_util::write_generic(&mut core, thread, KtfJvmThreadContext::zeroed())?;
            KtfJvmSupport::set_current_thread_context(&mut core, thread)?;
            let (jvm, _) = KtfJvmSupport::init(&mut core, &mut system_clone, None).await?;
            register_wipic_svc_handler(&mut core, &system_clone, &jvm)?;
            let application = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x90000u32)?;
            let class = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x90001u32)?;
            let create = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x90002u32)?;
            let destroy = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x90003u32)?;
            let time = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x9001au32)?;
            let memory = Allocator::alloc(&mut core, 128)?;
            wie_util::write_null_terminated_string_bytes(&mut core, memory + 4, b"DateTimeComponent")?;
            let cls = core.run_function::<u32>(class, &[memory + 4]).await?;
            let pac = core.run_function::<u32>(application, &[]).await?;
            wie_util::write_generic(&mut core, memory, pac)?;
            let direct = core.run_function::<u32>(create, &[pac, cls]).await?;
            assert_ne!(direct, u32::MAX);
            core.run_function::<()>(destroy, &[direct]).await?;
            let first = core.run_function::<u32>(create, &[memory, cls]).await?;
            let second = core.run_function::<u32>(create, &[memory, cls]).await?;
            assert_ne!(first, u32::MAX);
            assert_ne!(first, second);
            // Reusing the caller's context slot must not invalidate an instance.
            wie_util::write_generic(&mut core, memory, 0u32)?;
            assert_eq!(core.run_function::<u32>(create, &[memory, cls]).await?, u32::MAX);
            assert_eq!(core.run_function::<u32>(create, &[0, cls]).await?, u32::MAX);
            wie_util::write_generic(&mut core, memory + 32, [0x55555555u32; 11])?;
            core.run_function::<()>(time, &[first, memory + 36]).await?;
            let fields: [i32; 11] = wie_util::read_generic(&core, memory + 32)?;
            assert_eq!(fields[0], 0x55555555);
            assert_eq!(fields[10], 0x55555555);
            assert!((0..60).contains(&fields[1]));
            assert!((0..60).contains(&fields[2]));
            assert!((0..24).contains(&fields[3]));
            assert!((1..32).contains(&fields[4]));
            assert!((0..12).contains(&fields[5]));
            assert!((0..7).contains(&fields[7]));
            assert!((0..366).contains(&fields[8]));
            assert_eq!(fields[9], 0);
            core.run_function::<()>(destroy, &[first]).await?;
            core.run_function::<()>(time, &[second, memory + 36]).await?;
            assert_eq!(wie_util::read_generic::<[i32; 11], _>(&core, memory + 32)?, fields);
            core.run_function::<()>(destroy, &[second]).await?;
            done_clone.store(true, Ordering::Relaxed);
            Ok(())
        });
        for _ in 0..1000 {
            system.tick()?;
            if done.load(Ordering::Relaxed) {
                break;
            }
        }
        assert!(done.load(Ordering::Relaxed));
        Ok(())
    }
}
