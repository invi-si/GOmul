use alloc::{boxed::Box, string::ToString, vec};

mod context;
pub(super) mod graphics;

use jvm::{Jvm, Result as JvmResult, runtime::JavaLangString};
use wipi_types::lgt::CletFunctions;

use wie_backend::System;
use wie_core_arm::{ArmCore, EmulatedFunction, EmulatedFunctionParam, ResultWriter, SvcId};
use wie_jvm_support::JvmSupport;
use wie_util::{Result, read_generic, write_generic, write_null_terminated_string_bytes};
use wie_wipi_c::{
    MethodImpl, WIPICContext, WIPICMethodBody, WIPICResult,
    api::{database, graphics as shared_graphics, kernel, media, misc, net, uic},
};

use context::LgtWIPICContext;

use crate::runtime::{SVC_CATEGORY_WIPIC, svc_ids::WIPICSvcId};

// LGT's native 0x322 call passes the address of the application-context
// handle (observed stack slot), unlike the shared WIPI by-value contract.
async fn create_component(context: &mut dyn WIPICContext, pac_ptr: u32, cls: u32) -> Result<wipi_types::wipic::WIPICIndirectPtr> {
    let pac = if pac_ptr == 0 { 0 } else { read_generic::<u32, _>(context, pac_ptr)? };
    uic::create(context, pac, cls).await
}

// Native consumers walk a NUL-separated, empty-string-terminated list and
// inspect names such as SMSDATA, MMSDATA and CALLHISTORY. The original API
// name is unconfirmed. The emulated phone has no personal-data stores.
async fn list_phone_data_stores(context: &mut dyn WIPICContext, buffer: u32, capacity: i32) -> Result<i32> {
    if buffer == 0 || capacity <= 0 {
        return Ok(-22);
    }
    write_generic(context, buffer, 0u8)?;
    Ok(0)
}

fn directory_names(entries: &[alloc::string::String], directory: &str) -> alloc::collections::BTreeSet<alloc::string::String> {
    let prefix = alloc::format!("{}/", directory.trim_matches('/'));
    entries
        .iter()
        .filter_map(|entry| {
            let rest = entry.strip_prefix(&prefix)?;
            let (name, _) = rest.split_once('/')?;
            (!name.is_empty()).then(|| name.to_string())
        })
        .collect()
}

async fn list_resource_directories(
    core: &mut ArmCore,
    (system, jvm): &mut (System, Jvm),
    directory: u32,
    output: u32,
    capacity: u32,
    flags: u32,
) -> Result<u32> {
    use wie_util::{ByteWrite, read_null_terminated_string_bytes};
    if flags != 1 || output == 0 || capacity == 0 {
        return Ok((-22i32) as u32);
    }
    let directory = alloc::string::String::from_utf8(read_null_terminated_string_bytes(core, directory)?)
        .map_err(|e| wie_util::WieError::FatalError(e.to_string()))?;
    let key = JavaLangString::from_rust_string(jvm, "java.class.path")
        .await
        .map_err(|e| wie_util::WieError::FatalError(alloc::format!("Resource directory query: {e:?}")))?;
    let paths = jvm
        .invoke_static("java/lang/System", "getProperty", "(Ljava/lang/String;)Ljava/lang/String;", (key,))
        .await
        .map_err(|e| wie_util::WieError::FatalError(alloc::format!("Resource directory query: {e:?}")))?;
    let paths = JavaLangString::to_rust_string(jvm, &paths)
        .await
        .map_err(|e| wie_util::WieError::FatalError(alloc::format!("Resource directory query: {e:?}")))?;
    let mut names = alloc::collections::BTreeSet::new();
    for path in paths.split(if cfg!(windows) { ';' } else { ':' }) {
        let Some(size) = system.filesystem().size(path).await else {
            continue;
        };
        let mut bytes = vec![0; size];
        if system.filesystem().read(path, 0, size, &mut bytes).await != Some(size) {
            continue;
        }
        names.extend(directory_names(&wie_backend::zip_entry_names(&bytes)?, &directory));
    }
    let mut bytes = alloc::vec::Vec::new();
    for name in names {
        bytes.extend_from_slice(name.as_bytes());
        bytes.push(0);
    }
    bytes.push(0);
    if bytes.len() > capacity as usize {
        return Ok((-22i32) as u32);
    }
    core.write_bytes(output, &bytes)?;
    Ok(0)
}

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
    context: LgtWIPICContext,
    body: WIPICMethodBody,
}

async fn handle_wipic_svc(core: &mut ArmCore, (system, jvm): &mut (System, Jvm), id: SvcId) -> Result<()> {
    let wipic_context = LgtWIPICContext::new(core.clone(), system.clone(), jvm.clone());
    let (_, lr) = core.read_pc_lr()?;
    let method = match WIPICSvcId::try_from(id)? {
        WIPICSvcId::ListResourceDirectories => {
            return EmulatedFunction::call(&list_resource_directories, core, &mut (system.clone(), jvm.clone()))
                .await?
                .write(core, lr);
        }
        WIPICSvcId::CletRegister => {
            return EmulatedFunction::call(&clet_register, core, &mut (system.clone(), jvm.clone()))
                .await?
                .write(core, lr);
        }
        WIPICSvcId::GetFramebufferPointer => graphics::get_framebuffer_pointer.into_body(),
        WIPICSvcId::GetFramebufferWidth => graphics::get_framebuffer_width.into_body(),
        WIPICSvcId::GetFramebufferHeight => graphics::get_framebuffer_height.into_body(),
        WIPICSvcId::GetFramebufferBpl => graphics::get_framebuffer_bpl.into_body(),
        WIPICSvcId::GetFramebufferBpp => graphics::get_framebuffer_bpp.into_body(),
        WIPICSvcId::ListPhoneDataStores => list_phone_data_stores.into_body(),
        WIPICSvcId::Printk => kernel::printk.into_body(),
        WIPICSvcId::Sprintk => kernel::sprintk.into_body(),
        // Native callers use this slot after saving settings for a restart and
        // when the main loop stops. The original API name is not confirmed.
        WIPICSvcId::RequestExit => kernel::exit.into_body(),
        WIPICSvcId::Unk1 => unk1.into_body(),
        WIPICSvcId::Exit => kernel::exit.into_body(),
        WIPICSvcId::GetProgramName => kernel::get_program_name.into_body(),
        WIPICSvcId::Alloc => kernel::alloc.into_body(),
        WIPICSvcId::Calloc => kernel::calloc.into_body(),
        WIPICSvcId::Free => kernel::free.into_body(),
        WIPICSvcId::GetTotalMemory => kernel::get_total_memory.into_body(),
        WIPICSvcId::GetFreeMemory => kernel::get_free_memory.into_body(),
        WIPICSvcId::DefTimer => kernel::def_timer.into_body(),
        WIPICSvcId::SetTimer => kernel::set_timer.into_body(),
        WIPICSvcId::UnsetTimer => kernel::unset_timer.into_body(),
        WIPICSvcId::CurrentTime => kernel::current_time.into_body(),
        WIPICSvcId::GetSystemProperty => kernel::get_system_property.into_body(),
        WIPICSvcId::SetSystemProperty => kernel::set_system_property.into_body(),
        WIPICSvcId::GetResourceId => kernel::get_resource_id.into_body(),
        WIPICSvcId::GetResource => kernel::get_resource.into_body(),
        WIPICSvcId::Unk2 => unk2.into_body(),
        WIPICSvcId::GetImageProperty => graphics::get_image_property.into_body(),
        WIPICSvcId::GetImageFramebuffer => graphics::get_image_framebuffer.into_body(),
        WIPICSvcId::GetScreenFramebuffer => graphics::get_screen_framebuffer.into_body(),
        WIPICSvcId::DestroyOffscreenFramebuffer => graphics::destroy_offscreen_framebuffer.into_body(),
        WIPICSvcId::CreateOffscreenFramebuffer => graphics::create_offscreen_framebuffer.into_body(),
        WIPICSvcId::InitContext => graphics::init_context.into_body(),
        WIPICSvcId::SetContext => graphics::set_context.into_body(),
        WIPICSvcId::GetContext => graphics::get_context.into_body(),
        WIPICSvcId::PutPixel => graphics::put_pixel.into_body(),
        WIPICSvcId::DrawLine => graphics::draw_line.into_body(),
        WIPICSvcId::DrawRect => graphics::draw_rect.into_body(),
        WIPICSvcId::FillRect => graphics::fill_rect.into_body(),
        WIPICSvcId::CopyFrameBuffer => graphics::copy_frame_buffer.into_body(),
        WIPICSvcId::DrawImage => graphics::draw_image.into_body(),
        WIPICSvcId::CopyArea => graphics::copy_area.into_body(),
        WIPICSvcId::DrawArc => graphics::draw_arc.into_body(),
        WIPICSvcId::FillArc => graphics::fill_arc.into_body(),
        WIPICSvcId::DrawString => graphics::draw_string.into_body(),
        WIPICSvcId::GetRgbPixels => graphics::get_rgb_pixels.into_body(),
        WIPICSvcId::SetRgbPixels => graphics::set_rgb_pixels.into_body(),
        WIPICSvcId::FlushLcd => graphics::flush_lcd.into_body(),
        WIPICSvcId::GetPixelFromRgb => shared_graphics::get_pixel_from_rgb.into_body(),
        WIPICSvcId::GetRgbFromPixel => shared_graphics::get_rgb_from_pixel.into_body(),
        WIPICSvcId::GetDisplayInfo => graphics::get_display_info.into_body(),
        WIPICSvcId::Repaint => shared_graphics::repaint.into_body(),
        WIPICSvcId::GetFont => shared_graphics::get_font.into_body(),
        WIPICSvcId::GetFontHeight => shared_graphics::get_font_height.into_body(),
        WIPICSvcId::GetFontAscent => shared_graphics::get_font_ascent.into_body(),
        WIPICSvcId::GetFontDescent => shared_graphics::get_font_descent.into_body(),
        WIPICSvcId::GetStringWidth => shared_graphics::get_string_width.into_body(),
        WIPICSvcId::CreateImage => graphics::create_image.into_body(),
        WIPICSvcId::Unk0 => unk0.into_body(),
        WIPICSvcId::FillPolygon => graphics::fill_polygon.into_body(),
        WIPICSvcId::Unk11 => unk11.into_body(),
        WIPICSvcId::InputModeCount => wie_wipi_c::api::input::mode_count.into_body(),
        WIPICSvcId::InputModes => wie_wipi_c::api::input::modes.into_body(),
        WIPICSvcId::SetInputMode => wie_wipi_c::api::input::set_mode.into_body(),
        WIPICSvcId::GetInputMode => wie_wipi_c::api::input::get_mode.into_body(),
        WIPICSvcId::HandleInput => wie_wipi_c::api::input::handle_input.into_body(),
        WIPICSvcId::UicConfigure => uic::configure.into_body(),
        WIPICSvcId::UicGetGeometry => uic::get_geometry.into_body(),
        WIPICSvcId::UicSetEnable => uic::set_enable.into_body(),
        WIPICSvcId::UicInsertText => uic::insert_text.into_body(),
        WIPICSvcId::UicDeleteText => uic::delete_text.into_body(),
        WIPICSvcId::UicGetMaxTextSize => uic::get_max_text_size.into_body(),
        WIPICSvcId::UicSetMaxTextSize => uic::set_max_text_size.into_body(),
        WIPICSvcId::UicGetTextSize => uic::get_text_size.into_body(),
        WIPICSvcId::UicGetText => uic::get_text.into_body(),
        WIPICSvcId::UicCreateApplicationContext => uic::create_application_context.into_body(),
        WIPICSvcId::UicGetClass => uic::get_class.into_body(),
        WIPICSvcId::UicCreate => create_component.into_body(),
        WIPICSvcId::UicDestroy => uic::destroy.into_body(),
        WIPICSvcId::UicGetTime => uic::get_time.into_body(),
        WIPICSvcId::OpenDatabase => database::open_database.into_body(),
        WIPICSvcId::ReadRecordSingle => database::stream_read.into_body(),
        WIPICSvcId::WriteRecordSingle => database::stream_write.into_body(),
        WIPICSvcId::CloseDatabase => database::close_database.into_body(),
        WIPICSvcId::Unk12 => database::seek_record_single.into_body(),
        WIPICSvcId::Unk9 => database::list_record_info.into_body(),
        WIPICSvcId::DeleteRecord => database::delete_database.into_body(),
        WIPICSvcId::ListRecord => database::list_record.into_body(),
        WIPICSvcId::UpdateRecord => database::update_record.into_body(),
        WIPICSvcId::AvailableDatabaseStorage => database::available_storage_lgt.into_body(),
        WIPICSvcId::SelectRecord => database::select_record.into_body(),
        WIPICSvcId::Unk8 => database::exists_database.into_body(),
        WIPICSvcId::Connect => net::connect.into_body(),
        WIPICSvcId::Close => net::close.into_body(),
        WIPICSvcId::Socket => net::socket.into_body(),
        WIPICSvcId::InetAddrInt => inet_addr_int.into_body(),
        WIPICSvcId::HostToNetworkShort => host_to_network_short.into_body(),
        WIPICSvcId::SocketWrite => net::socket_write.into_body(),
        WIPICSvcId::SocketClose => net::socket_close.into_body(),
        WIPICSvcId::ClipCreate => media::clip_create.into_body(),
        WIPICSvcId::ClipFree => media::clip_free.into_body(),
        WIPICSvcId::ClipPutData => media::clip_put_data.into_body(),
        WIPICSvcId::Unk15 => unk15.into_body(),
        WIPICSvcId::ClipGetVolume => media::clip_get_volume.into_body(),
        WIPICSvcId::ClipSetVolume => media::clip_set_volume.into_body(),
        WIPICSvcId::Play => media::play.into_body(),
        WIPICSvcId::Stop => media::stop.into_body(),
        WIPICSvcId::Unk5 => unk5.into_body(),
        WIPICSvcId::Vibrator => media::vibrator.into_body(),
        WIPICSvcId::Unk14 => unk14.into_body(),
        WIPICSvcId::ClipAllocPlayer => media::clip_alloc_player.into_body(),
        WIPICSvcId::ClipFreePlayer => media::clip_free_player.into_body(),
        WIPICSvcId::Unk10 => unk10.into_body(),
        WIPICSvcId::SetMuteState => media::set_mute_state.into_body(),
        WIPICSvcId::GetMuteState => media::get_mute_state.into_body(),
        WIPICSvcId::BackLight => misc::back_light.into_body(),
        WIPICSvcId::Unk16 => unk16.into_body(),
    };

    EmulatedFunction::call(
        &CMethodProxy {
            context: wipic_context,
            body: method,
        },
        core,
        &mut (),
    )
    .await?
    .write(core, lr)
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

pub fn register_wipic_svc_handler(core: &mut ArmCore, system: &System, jvm: &Jvm) -> Result<()> {
    core.register_svc_handler(SVC_CATEGORY_WIPIC, handle_wipic_svc, &(system.clone(), jvm.clone()))
}

async fn clet_register(core: &mut ArmCore, (system, jvm): &mut (System, Jvm), function_table: u32, a1: u32) -> Result<()> {
    tracing::debug!("clet_register({function_table:#x}, {a1:#x})");

    let (screen_width, screen_height) = {
        let screen = system.platform().screen();
        (screen.width(), screen.height())
    };
    graphics::init_process_state(core, screen_width, screen_height)?;
    graphics::set_use_annunciator(core, a1)?;
    if let Some((width, height, full)) = system.platform().screen().display_override() {
        graphics::set_display_property(core, &mut (), 0, 0x64, width, 0).await?;
        graphics::set_display_property(core, &mut (), 0, 0x65, height, 0).await?;
        if full {
            graphics::set_use_annunciator(core, 0)?;
        }
    }
    let functions: CletFunctions = read_generic(core, function_table)?;

    jvm.put_static_field("net/wie/CletWrapper", "startClet", "I", functions.start_clet as i32)
        .await
        .unwrap();
    jvm.put_static_field("net/wie/CletWrapper", "pauseClet", "I", functions.pause_clet as i32)
        .await
        .unwrap();
    jvm.put_static_field("net/wie/CletWrapper", "resumeClet", "I", functions.resume_clet as i32)
        .await
        .unwrap();
    jvm.put_static_field("net/wie/CletWrapper", "destroyClet", "I", functions.destroy_clet as i32)
        .await
        .unwrap();
    jvm.put_static_field("net/wie/CletWrapper", "paintClet", "I", functions.paint_clet as i32)
        .await
        .unwrap();
    jvm.put_static_field("net/wie/CletWrapper", "handleCletEvent", "I", functions.handle_clet_event as i32)
        .await
        .unwrap();

    let main_class_name = JavaLangString::from_rust_string(jvm, "net/wie/CletWrapper").await.unwrap();
    let mut args_array = jvm.instantiate_array("Ljava/lang/String;", 1).await.unwrap();
    jvm.store_array(&mut args_array, 0, vec![main_class_name]).await.unwrap();

    let result: JvmResult<()> = jvm
        .invoke_static("org/kwis/msp/lcdui/Main", "main", "([Ljava/lang/String;)V", (args_array,))
        .await;

    if let Err(x) = result {
        return Err(JvmSupport::to_wie_err(jvm, x).await);
    }

    Ok(())
}

async fn unk0(_context: &mut dyn WIPICContext, a0: u32, a1: u32, a2: u32, a3: u32) -> Result<u32> {
    tracing::warn!("stub unk0({a0:#x}, {a1:#x}, {a2:#x}, {a3:#x})");

    // graphics

    Ok(0)
}

async fn unk1(_context: &mut dyn WIPICContext, a0: u32, a1: u32, a2: u32, a3: u32) -> Result<u32> {
    tracing::warn!("stub unk1({a0:#x}, {a1:#x}, {a2:#x}, {a3:#x})");

    // kernel

    Ok(0)
}

async fn unk2(context: &mut dyn WIPICContext) -> Result<u32> {
    tracing::warn!("stub unk2");

    // OEMC_knlGetProgramInfo? get app id
    let app_id = context.system().aid().to_string();
    let result = context.alloc_raw((app_id.len() + 1) as u32)?;
    write_null_terminated_string_bytes(context, result, app_id.as_bytes())?;

    Ok(result)
}

// ARM guests are little-endian; network byte order is big-endian.
async fn host_to_network_short(_context: &mut dyn WIPICContext, value: u32) -> Result<u32> {
    Ok((value as u16).swap_bytes() as u32)
}

async fn inet_addr_int(context: &mut dyn WIPICContext, address: u32) -> Result<u32> {
    let bytes = wie_util::read_null_terminated_string_bytes(context, address)?;
    let ip = core::str::from_utf8(&bytes)
        .ok()
        .and_then(|text| text.parse::<core::net::Ipv4Addr>().ok());
    // Network-order octets in the little-endian ARM guest word.
    Ok(ip.map_or(u32::MAX, |ip| u32::from_le_bytes(ip.octets())))
}

async fn unk5(_context: &mut dyn WIPICContext, a0: u32, a1: u32, a2: u32, a3: u32) -> Result<u32> {
    tracing::warn!("stub unk5({a0:#x}, {a1:#x}, {a2:#x}, {a3:#x})");

    // media

    Ok(0)
}

async fn unk10(_context: &mut dyn WIPICContext, a0: u32, a1: u32, a2: u32, a3: u32) -> Result<u32> {
    tracing::warn!("stub unk10({a0:#x}, {a1:#x}, {a2:#x}, {a3:#x})");

    Ok(0)
}

async fn unk11(_context: &mut dyn WIPICContext, a0: u32, a1: u32, a2: u32, a3: u32) -> Result<u32> {
    tracing::warn!("stub unk11({a0:#x}, {a1:#x}, {a2:#x}, {a3:#x})");

    Ok(0)
}

async fn unk14(_context: &mut dyn WIPICContext, a0: u32, a1: u32, a2: u32, a3: u32) -> Result<u32> {
    tracing::warn!("stub unk14({a0:#x}, {a1:#x}, {a2:#x}, {a3:#x})");

    // media

    Ok(0)
}

async fn unk15(_context: &mut dyn WIPICContext, a0: u32, a1: u32, a2: u32, a3: u32) -> Result<u32> {
    tracing::warn!("stub unk15({a0:#x}, {a1:#x}, {a2:#x}, {a3:#x})");

    // media

    Ok(0)
}

async fn unk16(_context: &mut dyn WIPICContext, a0: u32, a1: u32, a2: u32, a3: u32) -> Result<u32> {
    tracing::warn!("stub unk16({a0:#x}, {a1:#x}, {a2:#x}, {a3:#x})");

    // misc

    Ok(0)
}

#[cfg(test)]
mod tests {
    use alloc::{boxed::Box, sync::Arc};
    use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    use test_utils::{TestPlatform, TestPlatformEvent};
    use wie_backend::{DefaultTaskRunner, System};
    use wie_core_arm::{Allocator, ArmCore};
    use wie_util::Result;

    use super::register_wipic_svc_handler;
    use crate::runtime::{LgtJvmSupport, SVC_CATEGORY_WIPIC};

    #[test]
    fn resource_directory_listing_uses_immediate_unique_children() {
        let entries = ["tbl/B/001/a", "tbl/B/001/b", "tbl/B/002/", "tbl/B/file", "tbl/B-other/003/a"];
        let entries: alloc::vec::Vec<_> = entries.iter().map(|s| alloc::string::String::from(*s)).collect();
        assert_eq!(
            super::directory_names(&entries, "/tbl/B/").into_iter().collect::<alloc::vec::Vec<_>>(),
            ["001", "002"]
        );
        assert!(super::directory_names(&entries, "missing").is_empty());
    }

    #[test]
    fn component_context_pointer_and_date_time_use_native_svc_contract() -> Result<()> {
        let mut system = System::new(Box::new(TestPlatform::new()), "", "", DefaultTaskRunner);
        let system_clone = system.clone();
        let done = Arc::new(AtomicBool::new(false));
        let done_clone = done.clone();
        system.spawn(async move || {
            let mut core = ArmCore::new(false, None)?;
            Allocator::init(&mut core)?;
            let mut registers = core.save_context();
            registers.sp = Allocator::alloc(&mut core, 0x100)? + 0x100;
            core.restore_context(&registers);
            let jvm = LgtJvmSupport::init(&mut core, &system_clone, None).await?;
            register_wipic_svc_handler(&mut core, &system_clone, &jvm)?;
            let application = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x320u32)?;
            let class = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x321u32)?;
            let create = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x322u32)?;
            let destroy = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x323u32)?;
            let time = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x338u32)?;
            let memory = Allocator::alloc(&mut core, 128)?;
            wie_util::write_null_terminated_string_bytes(&mut core, memory + 4, b"DateTimeComponent")?;
            let cls = core.run_function::<u32>(class, &[memory + 4]).await?;
            let pac = core.run_function::<u32>(application, &[]).await?;
            wie_util::write_generic(&mut core, memory, pac)?;
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

    #[test]
    fn offscreen_cleanup_accepts_null_and_preserves_handle_validation() -> Result<()> {
        let mut system = System::new(Box::new(TestPlatform::new()), "", "", DefaultTaskRunner);
        let system_clone = system.clone();
        let done = Arc::new(AtomicBool::new(false));
        let done_clone = done.clone();
        system.spawn(async move || {
            let mut core = ArmCore::new(false, None)?;
            Allocator::init(&mut core)?;
            let mut registers = core.save_context();
            registers.sp = Allocator::alloc(&mut core, 0x100)? + 0x100;
            core.restore_context(&registers);
            let jvm = LgtJvmSupport::init(&mut core, &system_clone, None).await?;
            register_wipic_svc_handler(&mut core, &system_clone, &jvm)?;
            let destroy = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0xcbu32)?;
            let create = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0xccu32)?;
            for _ in 0..2 {
                core.run_function::<()>(destroy, &[0]).await?;
                let handle = core.run_function::<u32>(create, &[8, 8]).await?;
                assert_ne!(handle, 0);
                core.run_function::<()>(destroy, &[handle]).await?;
            }
            let invalid = Allocator::alloc(&mut core, 16)?;
            wie_util::write_generic(&mut core, invalid, [0u32; 4])?;
            assert!(core.run_function::<()>(destroy, &[invalid]).await.is_err());
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

    #[test]
    fn request_exit_svc_notifies_platform() -> Result<()> {
        let exit_count = Arc::new(AtomicU32::new(0));
        let observed_exits = exit_count.clone();
        let platform = TestPlatform::with_event_handler(move |event| {
            if matches!(event, TestPlatformEvent::Exit) {
                observed_exits.fetch_add(1, Ordering::Relaxed);
            }
        });
        let mut system = System::new(Box::new(platform), "", "", DefaultTaskRunner);
        let system_clone = system.clone();
        let done = Arc::new(AtomicBool::new(false));
        let done_clone = done.clone();

        system.spawn(async move || {
            let mut core = ArmCore::new(false, None)?;
            Allocator::init(&mut core)?;
            let mut registers = core.save_context();
            registers.sp = Allocator::alloc(&mut core, 0x100)? + 0x100;
            core.restore_context(&registers);

            let jvm = LgtJvmSupport::init(&mut core, &system_clone, None).await?;
            register_wipic_svc_handler(&mut core, &system_clone, &jvm)?;
            let request_exit = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x68u32)?;
            let _: () = core.run_function(request_exit, &[0]).await?;
            done_clone.store(true, Ordering::Relaxed);
            Ok(())
        });

        for _ in 0..1000 {
            system.tick()?;
            if done.load(Ordering::Relaxed) {
                break;
            }
        }
        assert!(done.load(Ordering::Relaxed), "exit request did not finish");
        assert_eq!(exit_count.load(Ordering::Relaxed), 1);
        Ok(())
    }
    #[test]
    fn input_mode_and_storage_svcs_use_guest_memory() -> Result<()> {
        let mut system = System::new(Box::new(TestPlatform::new()), "", "", DefaultTaskRunner);
        let system_clone = system.clone();
        let done = Arc::new(AtomicBool::new(false));
        let done_clone = done.clone();
        system.spawn(async move || {
            use wie_util::{read_generic, read_null_terminated_string_bytes};
            let mut core = ArmCore::new(false, None)?;
            Allocator::init(&mut core)?;
            let mut registers = core.save_context();
            registers.sp = Allocator::alloc(&mut core, 0x100)? + 0x100;
            core.restore_context(&registers);
            let jvm = LgtJvmSupport::init(&mut core, &system_clone, None).await?;
            register_wipic_svc_handler(&mut core, &system_clone, &jvm)?;
            let names = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x44cu32)?;
            let output = Allocator::alloc(&mut core, 8)?;
            wie_util::write_generic(&mut core, output, [0xa5u8; 8])?;
            assert_eq!(core.run_function::<u32>(names, &[output + 2, 4]).await?, 0);
            assert_eq!(read_generic::<[u8; 8], _>(&core, output)?, [0xa5, 0xa5, 0, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5]);
            for (buffer, capacity) in [(0, 4), (output, 0), (output, u32::MAX)] {
                assert_eq!(core.run_function::<u32>(names, &[buffer, capacity]).await? as i32, -22);
            }
            assert_eq!(read_generic::<u8, _>(&core, output)?, 0xa5);
            // Input-mode discovery must not overwrite display properties.
            wie_util::write_generic(&mut core, 0x7fff1010, [240u32, 320, 1])?;
            let count = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x12cu32)?;
            let modes = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x12du32)?;
            let set = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x12eu32)?;
            let get = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x12fu32)?;
            let storage = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x19cu32)?;
            assert_eq!(core.run_function::<u32>(count, &[]).await?, 4);
            let table = core.run_function::<u32>(modes, &[]).await?;
            let first: u32 = read_generic(&core, table)?;
            let second: u32 = read_generic(&core, table + 4)?;
            assert_eq!(read_null_terminated_string_bytes(&core, first)?, b"EN/L");
            assert_eq!(read_null_terminated_string_bytes(&core, second)?, b"EN/S");
            assert_eq!(core.run_function::<u32>(set, &[1]).await?, 1);
            assert_eq!(core.run_function::<u32>(get, &[]).await?, 1);
            assert_eq!(core.run_function::<u32>(set, &[4]).await? as i32, 0);
            assert_eq!(core.run_function::<u32>(modes, &[]).await?, table);
            assert_eq!(core.run_function::<u32>(get, &[]).await?, 1);
            assert_eq!(core.run_function::<u32>(storage, &[]).await?, 1024 * 1024);
            // Exercise all six ABI arguments, including the two stack arguments.
            let input = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x130u32)?;
            let buffers = Allocator::alloc(&mut core, 32)?;
            for (key, event, expected1, expected2) in [
                (b'2' as u32, 502, None, Some(b'a')),
                (b'2' as u32, 503, None, None),
                (b'2' as u32, 502, None, Some(b'b')),
                (b'3' as u32, 502, Some(b'b'), Some(b'd')),
                ((-99i32) as u32, 502, Some(b'd'), None),
                ((-99i32) as u32, 502, None, None),
            ] {
                wie_util::write_generic(&mut core, buffers, [0xccccccccu32, 1, 0xcccccccc, 1])?;
                assert_eq!(
                    core.run_function::<u32>(input, &[key, event, buffers, buffers + 4, buffers + 8, buffers + 12])
                        .await?,
                    0
                );
                assert_eq!(read_generic::<u32, _>(&core, buffers + 4)?, u32::from(expected1.is_some()));
                assert_eq!(read_generic::<u32, _>(&core, buffers + 12)?, u32::from(expected2.is_some()));
                assert_eq!(
                    read_generic::<u32, _>(&core, buffers)?,
                    expected1.map_or(0xcccccccc, |b| 0xcccccc00 | b as u32)
                );
                assert_eq!(
                    read_generic::<u32, _>(&core, buffers + 8)?,
                    expected2.map_or(0xcccccccc, |b| 0xcccccc00 | b as u32)
                );
            }
            // Insufficient capacity cannot consume the first key of a composition.
            wie_util::write_generic(&mut core, buffers, [0xccccccccu32, 0, 0xcccccccc, 0])?;
            assert_eq!(
                core.run_function::<u32>(input, &[50, 502, buffers, buffers + 4, buffers + 8, buffers + 12])
                    .await? as i32,
                -22
            );
            assert_eq!(core.run_function::<u32>(set, &[0]).await?, 1);
            wie_util::write_generic(&mut core, buffers + 12, 1u32)?;
            assert_eq!(
                core.run_function::<u32>(input, &[50, 502, buffers, buffers + 4, buffers + 8, buffers + 12])
                    .await?,
                0
            );
            assert_eq!(read_generic::<u8, _>(&core, buffers + 8)?, b'A');
            // Offline writes return an error without dereferencing guest buffers.
            let socket_write = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x25cu32)?;
            for args in [[0, 0, 1], [u32::MAX, 0xfffffff0, 64], [7, 0, 0]] {
                assert_eq!(core.run_function::<u32>(socket_write, &args).await? as i32, -1);
            }
            let htons = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x385u32)?;
            for (input, expected) in [(0, 0), (0x3b1d, 0x1d3b), (0xffff0001, 0x100)] {
                assert_eq!(core.run_function::<u32>(htons, &[input]).await?, expected);
            }
            let connect = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x258u32)?;
            assert_eq!(core.run_function::<u32>(connect, &[0, 123]).await? as i32, -1);
            let socket = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x7d0u32)?;
            for args in [[2, 1], [2, 2], [u32::MAX, 0]] {
                assert_eq!(core.run_function::<u32>(socket, &args).await? as i32, -1);
            }
            let inet = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x388u32)?;
            let ip_text = Allocator::alloc(&mut core, 64)?;
            for (text, expected) in [
                ("127.0.0.1", 0x0100007f),
                ("192.168.1.25", 0x1901a8c0),
                ("0.0.0.0", 0),
                ("255.255.255.255", u32::MAX),
                ("256.1.2.3", u32::MAX),
                ("not-an-address", u32::MAX),
                ("", u32::MAX),
            ] {
                wie_util::write_null_terminated_string_bytes(&mut core, ip_text, text.as_bytes())?;
                assert_eq!(core.run_function::<u32>(inet, &[ip_text]).await?, expected);
            }
            assert_eq!(read_generic::<[u32; 3], _>(&core, 0x7fff1010)?, [240, 320, 1]);
            let init = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0xcdu32)?;
            let set = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0xceu32)?;
            let get = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0xcfu32)?;
            let ctx = Allocator::alloc(&mut core, 56)?;
            let out = Allocator::alloc(&mut core, 24)?;
            core.run_function::<()>(init, &[ctx]).await?;
            core.run_function::<()>(get, &[ctx, 4, out]).await?;
            assert_eq!(read_generic::<u32, _>(&core, out)?, 255);
            for (op, value) in [(1, 0x123456), (2, 0x987654), (4, 127), (5, 2), (6, 128), (7, 3), (8, 5), (9, 1), (9, 0)] {
                core.run_function::<()>(set, &[ctx, op, value]).await?;
                core.run_function::<()>(get, &[ctx, op, out]).await?;
                assert_eq!(read_generic::<u32, _>(&core, out)?, value);
            }
            for (op, values) in [(0, [-7i32, 3, 200, 240]), (10, [-5, 8, 0, 0])] {
                wie_util::write_generic(&mut core, out, values)?;
                core.run_function::<()>(set, &[ctx, op, out]).await?;
                wie_util::write_generic(&mut core, out, [0i32; 4])?;
                core.run_function::<()>(get, &[ctx, op, out]).await?;
                assert_eq!(read_generic::<[i32; 4], _>(&core, out)?, values);
            }
            assert!(core.run_function::<()>(get, &[ctx, 4, 0]).await.is_err());
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
