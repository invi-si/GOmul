//! Native WIPI input method. All persistent state and mode strings are guest-owned.
use crate::WIPICContext;
use alloc::{string::String, vec::Vec};
use bytemuck::{Pod, Zeroable};
use wie_backend::hangul;
use wie_util::{Result, read_generic, write_generic};

// Keep existing EN/L and EN/S indices; advertise Korean and numeric after them.
const LANGUAGES: &[u8] = b"EN/L\0EN/S\0KO\0N123\0";
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct State {
    mode: u32,
    epoch: i32,
    key: u32,
    index: u32,
    latin: u32,
    count: u32,
    tokens: [u16; 32],
}
pub const STATE_SIZE: u32 = core::mem::size_of::<State>() as u32 + 16 + LANGUAGES.len() as u32;

pub async fn mode_count(_: &mut dyn WIPICContext) -> Result<u32> {
    Ok(4)
}
pub async fn modes(context: &mut dyn WIPICContext) -> Result<u32> {
    let base = context.input_state().await? + core::mem::size_of::<State>() as u32;
    for (i, offset) in [0, 5, 10, 13].into_iter().enumerate() {
        write_generic(context, base + i as u32 * 4, base + 16 + offset)?;
    }
    context.write_bytes(base + 16, LANGUAGES)?;
    Ok(base)
}
pub async fn set_mode(context: &mut dyn WIPICContext, mode: i32) -> Result<i32> {
    if !(0..4).contains(&mode) {
        return Ok(0);
    }
    let address = context.input_state().await?;
    let old: State = read_generic(context, address)?;
    let mut state = State::zeroed();
    state.mode = mode as u32;
    state.epoch = old.epoch;
    write_generic(context, address, state)?;
    Ok(1)
}
pub async fn get_mode(context: &mut dyn WIPICContext) -> Result<u32> {
    let address = context.input_state().await?;
    let mut state: State = read_generic(context, address)?;
    selection(context, &mut state).await?;
    Ok(state.mode)
}
async fn selection(context: &mut dyn WIPICContext, state: &mut State) -> Result<()> {
    if let Some((korean, epoch)) = context.input_selection().await? {
        if state.epoch != epoch {
            // A selection becomes effective at the next IME call. Do not discard
            // pending text: handle_input commits it before processing a new mode.
            state.epoch = epoch;
            state.mode = if korean { 2 } else { 1 };
        }
    }
    Ok(())
}
fn letters(key: u8) -> &'static [u8] {
    match key {
        b'0' => b" 0",
        b'1' => b".,?!1",
        b'2' => b"abc2",
        b'3' => b"def3",
        b'4' => b"ghi4",
        b'5' => b"jkl5",
        b'6' => b"mno6",
        b'7' => b"pqrs7",
        b'8' => b"tuv8",
        b'9' => b"wxyz9",
        _ => b"",
    }
}
fn encode(chars: &[u16]) -> Vec<u8> {
    encoding_rs::EUC_KR.encode(&String::from_utf16_lossy(chars)).0.into_owned()
}
#[allow(clippy::too_many_arguments)]
pub async fn handle_input(
    context: &mut dyn WIPICContext,
    key: u32,
    event: u32,
    committed: u32,
    committed_size: u32,
    composing: u32,
    composing_size: u32,
) -> Result<i32> {
    let capacity1: i32 = read_generic(context, committed_size)?;
    let capacity2: i32 = read_generic(context, composing_size)?;
    tracing::debug!(key, event, capacity1, capacity2, "native input request");
    if event != context.input_press_event() && key as u8 as i8 != -99 {
        write_generic(context, committed_size, 0i32)?;
        write_generic(context, composing_size, 0i32)?;
        return Ok(0);
    }
    let address = context.input_state().await?;
    let mut state: State = read_generic(context, address)?;
    let mut tokens = state.tokens[..(state.count as usize).min(32)].to_vec();
    let mut complete = Vec::new();
    let old_mode = state.mode;
    selection(context, &mut state).await?;
    if old_mode != state.mode {
        complete.extend(hangul::compose(&tokens));
        tokens.clear();
        if state.latin != 0 {
            complete.push(state.latin as u16);
            state.latin = 0;
        }
        state.key = 0;
    }
    let key = key as u8;
    if key as i8 == -99 || (state.mode == 2 && key == b'#' && event == context.input_press_event()) {
        complete.extend(hangul::compose(&tokens));
        tokens.clear();
        if state.latin != 0 {
            complete.push(state.latin as u16);
            state.latin = 0;
        }
        state.key = 0;
    } else if event == context.input_press_event() {
        if state.mode == 2 {
            if key.is_ascii_digit() {
                hangul::push(&mut tokens, (key - b'0') as u16, state.key == key as u32);
                let text = hangul::compose(&tokens);
                if text.len() > 1 {
                    complete.extend_from_slice(&text[..text.len() - 1]);
                    let last = &text[text.len() - 1..];
                    let start = (1..tokens.len())
                        .find(|&i| hangul::compose(&tokens[i..]) == last)
                        .ok_or_else(|| wie_util::WieError::FatalError("Invalid Hangul composition boundary".into()))?;
                    tokens = tokens[start..].to_vec();
                }
                state.key = key as u32;
            } else if key as i8 == -16 || key as i8 == -8 {
                tokens.pop();
                state.key = 0;
            } else if key == b'*' {
                complete.extend(hangul::compose(&tokens));
                complete.push(32);
                tokens.clear();
                state.key = 0;
            }
        } else if state.mode == 3 {
            if key.is_ascii_digit() {
                complete.push(key as u16);
            }
        } else {
            let choices = letters(key);
            if !choices.is_empty() {
                if state.key == key as u32 && state.latin != 0 {
                    state.index = (state.index + 1) % choices.len() as u32;
                } else {
                    if state.latin != 0 {
                        complete.push(state.latin as u16);
                    }
                    state.index = 0;
                }
                let c = choices[state.index as usize];
                state.latin = if state.mode == 0 { c.to_ascii_uppercase() } else { c } as u32;
                state.key = key as u32;
            } else if key as i8 == -16 || key as i8 == -8 {
                state.latin = 0;
                state.key = 0;
            }
        }
    }
    let mut pending = hangul::compose(&tokens);
    if state.latin != 0 {
        pending.push(state.latin as u16);
    }
    let complete = encode(&complete);
    let pending = encode(&pending);
    tracing::debug!(
        mode = state.mode,
        committed = complete.len(),
        composing = pending.len(),
        "native input result"
    );
    if capacity1 < complete.len() as i32 || capacity2 < pending.len() as i32 {
        return Ok(-22);
    }
    if !complete.is_empty() {
        context.write_bytes(committed, &complete)?;
    }
    if !pending.is_empty() {
        context.write_bytes(composing, &pending)?;
    }
    write_generic(context, committed_size, complete.len() as i32)?;
    write_generic(context, composing_size, pending.len() as i32)?;
    state.count = tokens.len() as u32;
    state.tokens.fill(0);
    state.tokens[..tokens.len()].copy_from_slice(&tokens);
    write_generic(context, address, state)?;
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::test::TestContext;
    use wie_util::{ByteRead, ByteWrite};
    async fn press(c: &mut TestContext, key: i32, event: u32, cap: i32) -> Result<(String, String)> {
        c.write_bytes(0x100, &[0xcc; 64])?;
        c.write_bytes(0x200, &[0xcc; 64])?;
        write_generic(c, 0x300, cap)?;
        write_generic(c, 0x304, cap)?;
        assert_eq!(handle_input(c, key as u32, event, 0x100, 0x300, 0x200, 0x304).await?, 0);
        let mut outputs = Vec::new();
        for (address, size) in [(0x100, 0x300), (0x200, 0x304)] {
            let n: i32 = read_generic(c, size)?;
            let mut b = alloc::vec![0;n as usize];
            c.read_bytes(address, &mut b)?;
            assert_eq!(read_generic::<u8, _>(c, address + n as u32)?, 0xcc);
            outputs.push(encoding_rs::EUC_KR.decode(&b).0.into_owned());
        }
        Ok((outputs.remove(0), outputs.remove(0)))
    }
    #[futures_test::test]
    async fn korean_stream_flush_delete_capacity_and_guest_restore() -> Result<()> {
        let mut c = TestContext::new();
        assert_eq!(mode_count(&mut c).await?, 4);
        set_mode(&mut c, 2).await?;
        let mut text = String::new();
        let mut pending = String::new();
        for key in b"881254355" {
            let (a, b) = press(&mut c, *key as i32, 2, 32).await?;
            text.push_str(&a);
            pending = b;
        }
        assert_eq!(text.clone() + &pending, "한글");
        assert_eq!(press(&mut c, 51, 3, 32).await?, (String::new(), String::new()));
        let p = c.input_state().await?;
        let mut snapshot = alloc::vec![0;STATE_SIZE as usize];
        c.read_bytes(p, &mut snapshot)?;
        let flushed = press(&mut c, -99, 2, 32).await?;
        assert_eq!(flushed.0, "글");
        assert!(flushed.1.is_empty());
        c.write_bytes(p, &snapshot)?;
        assert_eq!(press(&mut c, -99, 2, 32).await?, flushed);
        set_mode(&mut c, 2).await?;
        press(&mut c, 52, 2, 32).await?;
        press(&mut c, 49, 2, 32).await?;
        write_generic(&mut c, 0x300, 0i32)?;
        write_generic(&mut c, 0x304, 1i32)?;
        let before: State = read_generic(&c, p)?;
        assert_eq!(handle_input(&mut c, 50, 2, 0x100, 0x300, 0x200, 0x304).await?, -22);
        let after: State = read_generic(&c, p)?;
        assert_eq!(bytemuck::bytes_of(&before), bytemuck::bytes_of(&after));
        assert_eq!(press(&mut c, 50, 2, 32).await?.1, "가");
        assert_eq!(press(&mut c, -16, 2, 32).await?.1, "기");
        assert_eq!(press(&mut c, -16, 2, 32).await?.1, "ㄱ");
        Ok(())
    }
    #[futures_test::test]
    async fn korean_stream_boundaries_cover_long_key_sequences() -> Result<()> {
        let mut c = TestContext::new();
        set_mode(&mut c, 2).await?;
        let mut seed = 7u32;
        for _ in 0..10000 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            press(&mut c, 48 + (seed % 10) as i32, 2, 32).await?;
        }
        Ok(())
    }
}
