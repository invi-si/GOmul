use wie_util::{ByteRead, ByteWrite, Result};

use crate::ArmCore;

const COPY_CHUNK: usize = 4096;
const STR_SCAN_CHUNK: usize = 256;

pub async fn memcpy(core: &mut ArmCore, _: &mut (), ptr_dst: u32, ptr_src: u32, len: u32) -> Result<()> {
    let mut buf = [0u8; COPY_CHUNK];
    let mut offset: u32 = 0;
    while offset < len {
        let chunk = ((len - offset) as usize).min(COPY_CHUNK);
        core.read_bytes(ptr_src.wrapping_add(offset), &mut buf[..chunk])?;
        core.write_bytes(ptr_dst.wrapping_add(offset), &buf[..chunk])?;
        offset = offset.wrapping_add(chunk as u32);
    }
    Ok(())
}

/// Copy in the direction that preserves unread source bytes when ranges overlap.
pub async fn memmove(core: &mut ArmCore, _: &mut (), ptr_dst: u32, ptr_src: u32, len: u32) -> Result<u32> {
    if ptr_dst == ptr_src || len == 0 {
        return Ok(ptr_dst);
    }
    if ptr_dst < ptr_src || ptr_dst - ptr_src >= len {
        memcpy(core, &mut (), ptr_dst, ptr_src, len).await?;
    } else {
        let mut buf = [0; COPY_CHUNK];
        let mut remaining = len;
        while remaining > 0 {
            let chunk = remaining.min(COPY_CHUNK as u32);
            remaining -= chunk;
            core.read_bytes(ptr_src.wrapping_add(remaining), &mut buf[..chunk as usize])?;
            core.write_bytes(ptr_dst.wrapping_add(remaining), &buf[..chunk as usize])?;
        }
    }
    Ok(ptr_dst)
}

pub async fn memset(core: &mut ArmCore, _: &mut (), ptr_dst: u32, value: u32, len: u32) -> Result<()> {
    let buf = [value as u8; COPY_CHUNK];
    let mut offset: u32 = 0;
    while offset < len {
        let chunk = ((len - offset) as usize).min(COPY_CHUNK);
        core.write_bytes(ptr_dst.wrapping_add(offset), &buf[..chunk])?;
        offset = offset.wrapping_add(chunk as u32);
    }
    Ok(())
}

/// Reads in chunks because guest allocations are page-aligned. R0 already
/// holds the original `ptr_dst` for ARM ABI return, so the function returns
/// `()` and leaves R0 untouched.
pub async fn strcpy(core: &mut ArmCore, _: &mut (), ptr_dst: u32, ptr_src: u32) -> Result<()> {
    let mut buf = [0u8; STR_SCAN_CHUNK];
    let mut offset: u32 = 0;
    loop {
        core.read_bytes(ptr_src.wrapping_add(offset), &mut buf)?;
        if let Some(pos) = buf.iter().position(|&b| b == 0) {
            core.write_bytes(ptr_dst.wrapping_add(offset), &buf[..=pos])?;
            return Ok(());
        }
        core.write_bytes(ptr_dst.wrapping_add(offset), &buf)?;
        offset = offset.wrapping_add(STR_SCAN_CHUNK as u32);
    }
}

/// Return the first matching byte, including the terminating NUL when requested.
pub async fn strchr(core: &mut ArmCore, _: &mut (), ptr_str: u32, character: u32) -> Result<u32> {
    let mut cursor = ptr_str;
    loop {
        let mut byte = [0];
        core.read_bytes(cursor, &mut byte)?;
        if byte[0] == character as u8 {
            return Ok(cursor);
        }
        if byte[0] == 0 {
            return Ok(0);
        }
        cursor = cursor.wrapping_add(1);
    }
}

pub async fn strlen(core: &mut ArmCore, _: &mut (), ptr_str: u32) -> Result<u32> {
    let mut buf = [0u8; STR_SCAN_CHUNK];
    let mut len: u32 = 0;
    loop {
        core.read_bytes(ptr_str.wrapping_add(len), &mut buf)?;
        if let Some(pos) = buf.iter().position(|&b| b == 0) {
            return Ok(len.wrapping_add(pos as u32));
        }
        len = len.wrapping_add(STR_SCAN_CHUNK as u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    #[futures_test::test]
    async fn memmove_preserves_overlaps_across_copy_chunks() -> Result<()> {
        for (src, dst, len) in [(0, 3, 8193), (3, 0, 8193), (0, 9000, 7000), (3, 3, 8193), (0, 9000, 0)] {
            let mut core = ArmCore::new(false, None)?;
            core.map(0x10000, 0x4000)?;
            let initial: Vec<u8> = (0..0x4000).map(|i| (i % 251) as u8).collect();
            core.write_bytes(0x10000, &initial)?;
            let mut expected = initial.clone();
            expected.copy_within(src..src + len, dst);
            assert_eq!(
                memmove(&mut core, &mut (), 0x10000 + dst as u32, 0x10000 + src as u32, len as u32).await?,
                0x10000 + dst as u32
            );
            let mut actual = initial;
            core.read_bytes(0x10000, &mut actual)?;
            assert_eq!(actual, expected);
        }
        Ok(())
    }

    #[futures_test::test]
    async fn memmove_propagates_unmapped_memory_faults() -> Result<()> {
        let mut core = ArmCore::new(false, None)?;
        core.map(0x10000, 0x1000)?;
        assert!(memmove(&mut core, &mut (), 0x10000, 0x20000, 4).await.is_err());
        assert!(memmove(&mut core, &mut (), 0x20000, 0x10000, 4).await.is_err());
        Ok(())
    }
    #[futures_test::test]
    async fn strchr_handles_first_match_nul_high_bytes_and_faults() -> Result<()> {
        let mut core = ArmCore::new(false, None)?;
        core.map(0x10000, 0x1000)?;
        core.write_bytes(0x10ff8, b"a.b.c\xff\0")?;
        for (character, expected) in [(b'.' as u32, 0x10ff9), (0, 0x10ffe), (0x1ff, 0x10ffd), (b'z' as u32, 0)] {
            assert_eq!(strchr(&mut core, &mut (), 0x10ff8, character).await?, expected);
        }
        assert!(strchr(&mut core, &mut (), 0x20000, 0).await.is_err());
        Ok(())
    }
}
