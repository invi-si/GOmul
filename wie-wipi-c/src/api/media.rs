use alloc::{boxed::Box, vec, vec::Vec};

use bytemuck::{Pod, Zeroable};

use wipi_types::wipic::WIPICWord;

use wie_util::{Result, WieError, read_generic, write_generic};

use crate::{WIPICResult, context::WIPICContext, method::MethodBody};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MdaClip {
    clip_id: i32,
    h_proc: i32,
    r#type: u8,
    in_use: u8, // bool
    _padding1: [u8; 2],
    dev_id: i32,

    x: i32,
    y: i32,
    w: i32,
    h: i32,
    mute: u8, // bool
    _padding2: [u8; 3],
    watermark: i32,
    position: i32,
    quality: i32,
    mode: i32,
    state: i32,
    penpot: i32,
    num_slave: i32,

    clip_save: WIPICWord, // MC_MdaClip**

    audio_tone_saved_len: i32,
    audio_tone_len: i32,
    audio_tone: WIPICWord,          // MC_MdaToneType*
    audio_tone_duration: WIPICWord, // M_Int32 *

    audio_freq_saved_len: i32,
    audio_freq_len: i32,
    audio_hi_freq: WIPICWord,       // M_Int32 *
    audio_low_freq: WIPICWord,      // M_Int32 *
    audio_freq_duration: WIPICWord, // M_Int32 *

    sound_data_saved_len: i32,
    sound_data_len: i32,
    sound_data: WIPICWord, // M_Byte *

    original_volume: i32,

    pos: i8,
    _padding3: [u8; 3],
    codec_config_data_size: i32,
    codec_config_data: WIPICWord, // M_Byte *
    tick_duration: i32,

    b_control: u8, // bool
    _padding4: [u8; 3],

    movie_record_size_width: i32,
    movie_record_size_height: i32,
    max_record_length: i32,

    temp_record_space: WIPICWord, // M_Byte *
    temp_record_space_size: i32,
    temp_record_size: i32,

    next_ptr: WIPICWord, // MC_MdaClip*

    mda_id: i32,
    device_info: i32,

    // not in sdk, for internal usage
    handle: u32,
    generation: u32,
    pending_callbacks: u32,
    closed: u32,
}

pub async fn clip_create(context: &mut dyn WIPICContext, ptr_type: WIPICWord, buf_size: WIPICWord, callback: WIPICWord) -> Result<WIPICWord> {
    tracing::debug!("MC_mdaClipCreate({ptr_type:#x}, {buf_size:#x}, {callback:#x})");

    let clip = context.alloc_raw(size_of::<MdaClip>() as u32)?;
    write_generic(
        context,
        clip,
        MdaClip {
            h_proc: callback as i32,
            handle: u32::MAX,
            ..MdaClip::zeroed()
        },
    )?;

    Ok(clip)
}

pub async fn clip_free(context: &mut dyn WIPICContext, clip: WIPICWord) -> Result<WIPICWord> {
    tracing::debug!("MC_mdaClipFree({clip:#x})");

    // some app call clip free with null clip...
    if clip == 0 {
        return Ok(0);
    }

    let mut data: MdaClip = read_generic(context, clip)?;
    if data.closed != 0 {
        return Ok(0);
    }
    data.closed = 1;
    let _ = context.system().audio().close(data.handle);
    write_generic(context, clip, data)?;
    if data.pending_callbacks == 0 {
        context.free_raw(clip, size_of::<MdaClip>() as u32)?;
    }

    Ok(0)
}

pub async fn clip_get_type(_context: &mut dyn WIPICContext, clip: WIPICWord, buf: WIPICWord, buf_size: WIPICWord) -> Result<WIPICWord> {
    tracing::warn!("stub MC_mdaClipGetType({clip:#x}, {buf:#x}, {buf_size:#x})");

    Ok(0)
}

pub async fn get_mute_state(_context: &mut dyn WIPICContext, source: WIPICWord) -> Result<WIPICWord> {
    tracing::warn!("stub MC_mdaGetMuteState({source:#x})");

    Ok(0)
}

pub async fn clip_get_info(
    _context: &mut dyn WIPICContext,
    clip: WIPICWord,
    command: WIPICWord,
    buf: WIPICWord,
    buf_size: WIPICWord,
) -> Result<WIPICWord> {
    tracing::warn!("stub OEMC_mdaClipGetInfo({clip:#x}, {command:#x}, {buf:#x}, {buf_size:#x})");

    Ok(0)
}

pub async fn clip_put_data(context: &mut dyn WIPICContext, ptr_clip: WIPICWord, buf: WIPICWord, buf_size: WIPICWord) -> Result<i32> {
    tracing::debug!("MC_mdaClipPutData({ptr_clip:#x}, {buf:#x}, {buf_size:#x})");

    if ptr_clip == 0 {
        return Ok(-1);
    }

    let mut data = vec![0; buf_size as _];
    context.read_bytes(buf, &mut data)?;

    let handle = context.system().audio().load_smaf(&data);
    if let Err(x) = handle {
        tracing::error!("Failed to load audio: {x:?}");
        return Ok(0);
    }

    let handle = handle.unwrap();

    let mut clip: MdaClip = read_generic(context, ptr_clip)?;
    let _ = context.system().audio().close(clip.handle);
    clip.generation = next_generation(clip.generation)?;
    clip.handle = handle;
    write_generic(context, ptr_clip, clip)?;

    Ok(buf_size as _)
}

pub async fn clip_get_data(_context: &mut dyn WIPICContext, clip: WIPICWord, buf: WIPICWord, buf_size: WIPICWord) -> Result<WIPICWord> {
    tracing::warn!("stub MC_mdaClipGetData({clip:#x}, {buf:#x}, {buf_size:#x})");

    Ok(0)
}

pub async fn clip_set_position(_context: &mut dyn WIPICContext, clip: WIPICWord, ms: WIPICWord) -> Result<WIPICWord> {
    tracing::warn!("stub MC_mdaClipSetPosition({clip:#x}, {ms:#x})");

    Ok(0)
}

pub async fn clip_get_volume(_context: &mut dyn WIPICContext, clip: WIPICWord) -> Result<WIPICWord> {
    tracing::warn!("stub MC_mdaClipGetVolume({clip:#x})");

    Ok(0)
}

pub async fn clip_set_volume(_context: &mut dyn WIPICContext, clip: WIPICWord, volume: WIPICWord) -> Result<WIPICWord> {
    tracing::warn!("stub MC_mdaClipSetVolume({clip:#x}, {volume:#x})");

    Ok(0)
}

pub async fn get_volume(_context: &mut dyn WIPICContext) -> Result<WIPICWord> {
    tracing::warn!("stub MC_mdaGetVolume");

    Ok(0)
}

pub async fn set_volume(_context: &mut dyn WIPICContext, volume: i32) -> Result<()> {
    tracing::warn!("stub MC_mdaSetVolume({volume})");

    Ok(())
}

pub async fn play(context: &mut dyn WIPICContext, ptr_clip: WIPICWord, repeat: WIPICWord) -> Result<i32> {
    tracing::debug!("MC_mdaPlay({ptr_clip:#x}, {repeat})");

    if ptr_clip == 0 {
        return Ok(0);
    }

    let mut clip: MdaClip = read_generic(context, ptr_clip)?;
    if clip.closed != 0 {
        return Ok(0);
    }
    let generation = next_generation(clip.generation)?;
    let result = context.system().audio().play(clip.handle, repeat != 0);

    if let Err(x) = result {
        tracing::error!("Failed to load audio: {x:?}");
    } else {
        clip.generation = generation;
        if clip.h_proc != 0 {
            clip.pending_callbacks = clip.pending_callbacks.checked_add(1).ok_or(WieError::AllocationFailure)?;
            write_generic(context, ptr_clip, clip)?;
            if let Err(error) = context.spawn(Box::new(PlaybackStarted { clip: ptr_clip, generation })) {
                clip.pending_callbacks -= 1;
                write_generic(context, ptr_clip, clip)?;
                return Err(error);
            }
        } else {
            write_generic(context, ptr_clip, clip)?;
        }
    }

    Ok(0)
}

pub async fn clip_alloc_player(_context: &mut dyn WIPICContext, clip: WIPICWord, param: WIPICWord) -> Result<WIPICWord> {
    tracing::warn!("stub MC_mdaClipAllocPlayer({clip:#x}, {param:#x})");

    Ok(0)
}

pub async fn clip_free_player(_context: &mut dyn WIPICContext, clip: WIPICWord) -> Result<WIPICWord> {
    tracing::warn!("stub MC_mdaClipFreePlayer({clip:#x})");

    Ok(0)
}

pub async fn vibrator(context: &mut dyn WIPICContext, level: i32, timeout: i32) -> Result<WIPICWord> {
    tracing::debug!("MC_mdaVibrator({level}, {timeout})");

    let duration_ms = timeout.max(0) as u64;
    let intensity = (level.clamp(0, 10) * 10) as u8;
    context.system().platform().vibrate(duration_ms, intensity);

    Ok(0)
}

pub async fn set_mute_state(_context: &mut dyn WIPICContext, source: i32, b_mute: i32) -> Result<WIPICWord> {
    tracing::warn!("stub MC_mdaSetMuteState({source:#x}, {b_mute})");

    Ok(0)
}

pub async fn pause(_context: &mut dyn WIPICContext, clip: WIPICWord) -> Result<WIPICWord> {
    tracing::warn!("stub MC_mdaPause({clip:#x})");

    Ok(0)
}

pub async fn resume(_context: &mut dyn WIPICContext, clip: WIPICWord) -> Result<WIPICWord> {
    tracing::warn!("stub MC_mdaResume({clip:#x})");

    Ok(0)
}

pub async fn stop(context: &mut dyn WIPICContext, ptr_clip: WIPICWord) -> Result<WIPICWord> {
    tracing::debug!("MC_mdaStop({ptr_clip:#x})");

    if ptr_clip == 0 {
        return Ok(0);
    }

    let mut clip: MdaClip = read_generic(context, ptr_clip)?;
    clip.generation = next_generation(clip.generation)?;
    write_generic(context, ptr_clip, clip)?;

    let system = context.system();

    system.audio().stop(clip.handle);

    Ok(0)
}

pub async fn record(_context: &mut dyn WIPICContext, clip: WIPICWord) -> Result<WIPICWord> {
    tracing::warn!("stub MC_mdaRecord({clip:#x})");

    Ok(0)
}

pub async fn unk7(_context: &mut dyn WIPICContext, clip: WIPICWord) -> Result<WIPICWord> {
    tracing::warn!("stub MC_mdaUnk7({clip:#x})");

    Ok(0)
}

pub async fn unk17(_context: &mut dyn WIPICContext, clip: WIPICWord) -> Result<WIPICWord> {
    tracing::warn!("stub MC_mdaUnk17({clip:#x})");

    Ok(0)
}

pub async fn unk18(_context: &mut dyn WIPICContext, clip: WIPICWord) -> Result<WIPICWord> {
    tracing::warn!("stub MC_mdaUnk18({clip:#x})");

    Ok(0)
}

// Keep callback lifetime and cancellation in guest-owned clip storage. A queued
// notification retains the allocation, so a freed/reused address cannot receive it.
fn next_generation(value: u32) -> Result<u32> {
    value
        .checked_add(1)
        .ok_or_else(|| WieError::FatalError("Media generation exhausted".into()))
}
struct PlaybackStarted {
    clip: u32,
    generation: u32,
}
#[async_trait::async_trait]
impl MethodBody<WieError> for PlaybackStarted {
    async fn call(&self, context: &mut dyn WIPICContext, _: Box<[u32]>) -> Result<WIPICResult> {
        let clip: MdaClip = read_generic(context, self.clip)?;
        let result = if clip.closed == 0 && clip.generation == self.generation {
            // Event 2 initializes the observed WIPI playback clock. Completion,
            // pause/resume and stop notifications are separate contracts.
            context.call_function(clip.h_proc as u32, &[self.clip, 2]).await.map(|_| ())
        } else {
            Ok(())
        };
        // The callback itself can stop, replay, or free the clip.
        let mut clip: MdaClip = read_generic(context, self.clip)?;
        clip.pending_callbacks -= 1;
        write_generic(context, self.clip, clip)?;
        if clip.closed != 0 && clip.pending_callbacks == 0 {
            context.free_raw(self.clip, size_of::<MdaClip>() as u32)?;
        }
        result?;
        Ok(WIPICResult { results: Vec::new() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::test::TestContext;
    use wie_util::ByteWrite;
    fn context() -> TestContext {
        TestContext::with_system(wie_backend::System::new(
            Box::new(test_utils::TestPlatform::new()),
            "media",
            "media",
            wie_backend::DefaultTaskRunner,
        ))
    }
    async fn put_valid(c: &mut TestContext, clip: u32) {
        let data = b"MMMD\0\0\0\x02\0\0";
        let ptr = c.alloc_raw(data.len() as u32).unwrap();
        c.write_bytes(ptr, data).unwrap();
        assert_eq!(clip_put_data(c, clip, ptr, data.len() as u32).await.unwrap(), data.len() as i32);
        c.free_raw(ptr, data.len() as u32).unwrap();
    }
    async fn loaded(c: &mut TestContext, callback: u32) -> u32 {
        let clip = clip_create(c, 0, 0, callback).await.unwrap();
        put_valid(c, clip).await;
        clip
    }
    async fn drain(c: &mut TestContext) {
        while !c.spawned.is_empty() {
            let callback = c.spawned.remove(0);
            callback.call(c, Box::new([])).await.unwrap();
        }
    }
    #[futures_test::test]
    async fn invalid_clip_data_preserves_the_current_clip() {
        let mut c = context();
        let clip = loaded(&mut c, 0x101).await;
        let before: MdaClip = read_generic(&mut c, clip).unwrap();
        assert_eq!(clip_put_data(&mut c, clip, 0, 0).await.unwrap(), 0);
        let after: MdaClip = read_generic(&mut c, clip).unwrap();
        assert_eq!(before.handle, after.handle);
        assert_eq!(before.generation, after.generation);
        play(&mut c, clip, 0).await.unwrap();
        drain(&mut c).await;
        assert_eq!(c.calls, vec![(0x101, vec![clip, 2])]);
    }
    #[futures_test::test]
    async fn playback_start_is_deferred_and_requires_loaded_audio_and_listener() {
        let mut c = context();
        let invalid = clip_create(&mut c, 0, 0, 0x101).await.unwrap();
        play(&mut c, invalid, 0).await.unwrap();
        assert!(c.spawned.is_empty());
        let silent = loaded(&mut c, 0).await;
        play(&mut c, silent, 0).await.unwrap();
        assert!(c.spawned.is_empty());
        let clip = loaded(&mut c, 0x101).await;
        play(&mut c, clip, 0).await.unwrap();
        assert!(c.calls.is_empty());
        assert_eq!(c.spawned.len(), 1);
        drain(&mut c).await;
        assert_eq!(c.calls, vec![(0x101, vec![clip, 2])]);
    }
    #[futures_test::test]
    async fn stop_replay_and_data_replacement_cancel_stale_starts() {
        let mut c = context();
        let clip = loaded(&mut c, 0x101).await;
        play(&mut c, clip, 0).await.unwrap();
        stop(&mut c, clip).await.unwrap();
        drain(&mut c).await;
        assert!(c.calls.is_empty());
        play(&mut c, clip, 0).await.unwrap();
        play(&mut c, clip, 0).await.unwrap();
        drain(&mut c).await;
        assert_eq!(c.calls.len(), 1);
        c.calls.clear();
        play(&mut c, clip, 0).await.unwrap();
        put_valid(&mut c, clip).await;
        drain(&mut c).await;
        assert!(c.calls.is_empty());
    }
    #[futures_test::test]
    async fn free_retains_allocation_until_queued_start_is_discarded() {
        let mut c = context();
        let clip = loaded(&mut c, 0x101).await;
        c.raw_freed.clear(); // Start observing frees after fixture setup.
        play(&mut c, clip, 0).await.unwrap();
        clip_free(&mut c, clip).await.unwrap();
        assert!(c.raw_freed.is_empty());
        drain(&mut c).await;
        assert!(c.calls.is_empty());
        assert_eq!(c.raw_freed, vec![clip]);
    }
    #[futures_test::test]
    async fn callback_closing_clip_releases_after_return() {
        let mut c = context();
        c.pixel_callback = Some(|c, _, args| {
            let mut clip: MdaClip = read_generic(c, args[0])?;
            assert_eq!(clip.pending_callbacks, 1);
            clip.closed = 1;
            write_generic(c, args[0], clip)?;
            Ok(0)
        });
        let clip = loaded(&mut c, 0x101).await;
        c.raw_freed.clear(); // Start observing frees after fixture setup.
        play(&mut c, clip, 0).await.unwrap();
        drain(&mut c).await;
        assert_eq!(c.calls.len(), 1);
        assert_eq!(c.raw_freed, vec![clip]);
    }
}
