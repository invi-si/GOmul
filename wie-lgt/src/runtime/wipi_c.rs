use alloc::{boxed::Box, string::ToString, vec};

mod context;
pub(super) mod graphics;

use jvm::{Jvm, Result as JvmResult, runtime::JavaLangString};
use wipi_types::lgt::CletFunctions;
use wipi_types::wipic::WIPICIndirectPtr;

use wie_backend::System;
use wie_core_arm::{ArmCore, EmulatedFunction, EmulatedFunctionParam, ResultWriter, SvcId};
use wie_jvm_support::JvmSupport;
use wie_util::{Result, read_generic, write_generic, write_null_terminated_string_bytes};
use wie_wipi_c::{
    MethodImpl, WIPICContext, WIPICMethodBody, WIPICResult,
    api::{database, graphics as shared_graphics, kernel, media, misc, net},
};

use context::LgtWIPICContext;

use crate::runtime::{SVC_CATEGORY_WIPIC, svc_ids::WIPICSvcId};

const TIME_VALUE_PTR: u32 = 0x7fff1004;

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
        WIPICSvcId::Unk11 => unk11.into_body(),
        WIPICSvcId::InputModeCount => input_mode_count.into_body(),
        WIPICSvcId::InputModes => input_modes.into_body(),
        WIPICSvcId::SetInputMode => set_input_mode.into_body(),
        WIPICSvcId::GetInputMode => get_input_mode.into_body(),
        WIPICSvcId::TimeNow => time_now.into_body(),
        WIPICSvcId::TimeComponent => time_component.into_body(),
        WIPICSvcId::TimeConvert => time_convert.into_body(),
        WIPICSvcId::TimeToTm => time_to_tm.into_body(),
        WIPICSvcId::DateTimeToTm => time_to_tm.into_body(),
        WIPICSvcId::OpenDatabase => database::open_database.into_body(),
        WIPICSvcId::ReadRecordSingle => database::stream_read.into_body(),
        WIPICSvcId::WriteRecordSingle => database::stream_write.into_body(),
        WIPICSvcId::CloseDatabase => database::close_database.into_body(),
        WIPICSvcId::Unk12 => database::seek_record_single.into_body(),
        WIPICSvcId::Unk9 => database::list_record_info.into_body(),
        WIPICSvcId::DeleteRecord => database::delete_database.into_body(),
        WIPICSvcId::ListRecord => database::list_record.into_body(),
        WIPICSvcId::UpdateRecord => database::update_record.into_body(),
        WIPICSvcId::AvailableDatabaseStorage => database::list_databases.into_body(),
        WIPICSvcId::SelectRecord => database::select_record.into_body(),
        WIPICSvcId::Unk8 => database::exists_database.into_body(),
        WIPICSvcId::Connect => net::connect.into_body(),
        WIPICSvcId::Close => net::close.into_body(),
        WIPICSvcId::HostToNetworkShort => host_to_network_short.into_body(),
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

// WIPI mode discovery returns a count and a char** table, not a handle.
// The table and current selection live in guest global memory.
// 0x7fff1010..0x7fff101c belongs to the display properties record.
const INPUT_MODE_TABLE: u32 = 0x7fff1040;
const INPUT_MODE_STRINGS: u32 = INPUT_MODE_TABLE + 8;
const INPUT_MODE_CURRENT: u32 = INPUT_MODE_TABLE + 20;

// ARM guests are little-endian; network byte order is big-endian.
async fn host_to_network_short(_context: &mut dyn WIPICContext, value: u32) -> Result<u32> {
    Ok((value as u16).swap_bytes() as u32)
}

async fn input_mode_count(_context: &mut dyn WIPICContext) -> Result<u32> {
    Ok(2)
}

async fn input_modes(context: &mut dyn WIPICContext) -> Result<u32> {
    write_generic(context, INPUT_MODE_TABLE, INPUT_MODE_STRINGS)?;
    write_generic(context, INPUT_MODE_TABLE + 4, INPUT_MODE_STRINGS + 5)?;
    context.write_bytes(INPUT_MODE_STRINGS, b"EN/L\0EN/S\0")?;
    Ok(INPUT_MODE_TABLE)
}

async fn set_input_mode(context: &mut dyn WIPICContext, mode: i32) -> Result<i32> {
    if !(0..2).contains(&mode) {
        return Ok(-22);
    }
    write_generic(context, INPUT_MODE_CURRENT, mode)?;
    Ok(0)
}

async fn get_input_mode(context: &mut dyn WIPICContext) -> Result<i32> {
    read_generic(context, INPUT_MODE_CURRENT)
}

async fn unk5(_context: &mut dyn WIPICContext, a0: u32, a1: u32, a2: u32, a3: u32) -> Result<u32> {
    tracing::warn!("stub unk5({a0:#x}, {a1:#x}, {a2:#x}, {a3:#x})");

    // media

    Ok(0)
}

async fn time_now(context: &mut dyn WIPICContext, component_class: u32) -> Result<u32> {
    let epoch_seconds = context.system().platform().now().raw() / 1000;
    tracing::debug!("LGT_timeNow({component_class:#x}) -> {epoch_seconds}");

    write_time_value(context, epoch_seconds as u32)
}

async fn time_component(_context: &mut dyn WIPICContext, name: u32) -> Result<u32> {
    tracing::debug!("LGT_timeComponent({name:#x})");

    Ok(name)
}

async fn time_convert(context: &mut dyn WIPICContext, date_time: u32, component: u32) -> Result<u32> {
    tracing::debug!("LGT_timeConvert({date_time:#x}, {component:#x})");

    let timestamp = read_time_value(context, date_time)?;
    write_time_value(context, timestamp)
}

async fn time_to_tm(context: &mut dyn WIPICContext, time_value: u32, out_ptr: u32) -> Result<i32> {
    tracing::debug!("LGT_timeToTm({time_value:#x}, {out_ptr:#x})");

    let timestamp = read_time_value(context, time_value)?;
    let (year, month, day, hour, minute, second) = unix_seconds_to_utc(timestamp as i64);
    write_generic(context, out_ptr, second)?;
    write_generic(context, out_ptr + 4, minute)?;
    write_generic(context, out_ptr + 8, hour)?;
    write_generic(context, out_ptr + 12, day)?;
    write_generic(context, out_ptr + 16, month - 1)?;
    write_generic(context, out_ptr + 20, year - 1900)?;

    Ok(0)
}

fn write_time_value(context: &mut dyn WIPICContext, timestamp: u32) -> Result<u32> {
    let time_value_ptr: u32 = read_generic(context, TIME_VALUE_PTR)?;
    let memory = if time_value_ptr != 0 {
        WIPICIndirectPtr(time_value_ptr)
    } else {
        let memory = context.alloc(4)?;
        write_generic(context, TIME_VALUE_PTR, memory.0)?;
        memory
    };
    write_generic(context, context.data_ptr(memory)?, timestamp)?;
    Ok(memory.0)
}

fn read_time_value(context: &mut dyn WIPICContext, handle: u32) -> Result<u32> {
    read_generic(context, context.data_ptr(WIPICIndirectPtr(handle))?)
}

fn unix_seconds_to_utc(timestamp: i64) -> (i32, i32, i32, i32, i32, i32) {
    let days = timestamp.div_euclid(86_400);
    let seconds_of_day = timestamp.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = (seconds_of_day / 3600) as i32;
    let minute = ((seconds_of_day % 3600) / 60) as i32;
    let second = (seconds_of_day % 60) as i32;

    (year, month, day, hour, minute, second)
}

fn civil_from_days(days: i64) -> (i32, i32, i32) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_param = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_param + 2) / 5 + 1;
    let month = month_param + if month_param < 10 { 3 } else { -9 };
    year += if month <= 2 { 1 } else { 0 };

    (year as i32, month as i32, day as i32)
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
            // Input-mode discovery must not overwrite display properties.
            wie_util::write_generic(&mut core, 0x7fff1010, [240u32, 320, 1])?;
            let count = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x12cu32)?;
            let modes = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x12du32)?;
            let set = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x12eu32)?;
            let get = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x12fu32)?;
            let storage = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x19cu32)?;
            assert_eq!(core.run_function::<u32>(count, &[]).await?, 2);
            let table = core.run_function::<u32>(modes, &[]).await?;
            let first: u32 = read_generic(&core, table)?;
            let second: u32 = read_generic(&core, table + 4)?;
            assert_eq!(read_null_terminated_string_bytes(&core, first)?, b"EN/L");
            assert_eq!(read_null_terminated_string_bytes(&core, second)?, b"EN/S");
            assert_eq!(core.run_function::<u32>(set, &[1]).await?, 0);
            assert_eq!(core.run_function::<u32>(get, &[]).await?, 1);
            assert_eq!(core.run_function::<u32>(set, &[2]).await? as i32, -22);
            assert_eq!(core.run_function::<u32>(modes, &[]).await?, table);
            assert_eq!(core.run_function::<u32>(get, &[]).await?, 1);
            assert_eq!(core.run_function::<u32>(storage, &[]).await?, 1024 * 1024);
            let htons = core.make_svc_stub(SVC_CATEGORY_WIPIC, 0x385u32)?;
            for (input, expected) in [(0, 0), (0x3b1d, 0x1d3b), (0xffff0001, 0x100)] {
                assert_eq!(core.run_function::<u32>(htons, &[input]).await?, expected);
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
