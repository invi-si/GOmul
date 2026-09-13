use image::RgbaImage;
use wie_util::{Result, WieError};

/// Observed phone BMP extension: reserved DWORD 1 marks an indexed transparency
/// key stored in biClrImportant. Apply only the evidenced 8-bit BI_RGB variant.
/// Ordinary BMPs retain the standard decoder's semantics, including opaque green.
pub(super) fn apply_palette_mask(data: &[u8], rgba: &mut RgbaImage) -> Result<()> {
    if data.len() < 54 || &data[..2] != b"BM" {
        return Ok(());
    }
    let word = |offset| u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
    if word(6) != 1 || word(14) != 40 || data[26..30] != [1, 0, 8, 0] || word(30) != 0 {
        return Ok(());
    }
    let count = match word(46) {
        0 => 256,
        count => count,
    };
    let key = word(50);
    // Some otherwise valid phone BMPs use values outside the palette here.
    // Their meaning is not established; retain the standard decoder's pixels.
    if key >= count {
        return Ok(());
    }
    let offset = u64::from(word(10));
    let stride = (u64::from(rgba.width()) + 3) & !3;
    if count > 256 || offset < 54 + u64::from(count) * 4 || offset + stride * u64::from(rgba.height()) > data.len() as u64 {
        return Err(WieError::FatalError("Invalid indexed BMP transparency data".into()));
    }
    let top_down = (word(22) as i32) < 0;
    for y in 0..rgba.height() {
        let row = if top_down { y } else { rgba.height() - 1 - y };
        let start = (offset + u64::from(row) * stride) as usize;
        for x in 0..rgba.width() {
            // Match the palette index, not RGB: another palette entry may have
            // the same color and still be intentionally opaque.
            if u32::from(data[start + x as usize]) == key {
                rgba.get_pixel_mut(x, y).0[3] = 0;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use alloc::{vec, vec::Vec};

    use crate::canvas::decode_image;

    fn fixture(top_down: bool, flag: u32) -> Vec<u8> {
        let mut data = vec![0; 78];
        data[..2].copy_from_slice(b"BM");
        for (offset, value) in [
            (2, 78u32),
            (6, flag),
            (10, 70),
            (14, 40),
            (18, 3),
            (22, if top_down { (-2i32) as u32 } else { 2 }),
            (34, 8),
            (46, 4),
            (50, 1),
        ] {
            data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        data[26..30].copy_from_slice(&[1, 0, 8, 0]);
        data[54..70].copy_from_slice(&[0, 0, 255, 0, 0, 255, 0, 0, 0, 255, 0, 0, 255, 0, 0, 0]);
        // Top row: red, masked green, opaque green. Bottom: masked green,
        // opaque green, blue. Padding deliberately equals the mask index.
        let top = [0, 1, 2, 1];
        let bottom = [1, 2, 3, 1];
        data[70..74].copy_from_slice(if top_down { &top } else { &bottom });
        data[74..78].copy_from_slice(if top_down { &bottom } else { &top });
        data
    }

    #[test]
    fn flagged_palette_mask_preserves_orientation_padding_and_duplicate_colors() {
        for top_down in [false, true] {
            let data = fixture(top_down, 1);
            let original = data.clone();
            let image = decode_image(&data).unwrap();
            assert_eq!((image.width(), image.height()), (3, 2));
            let colors = image.colors();
            let rgba: Vec<_> = colors.iter().map(|c| [c.r, c.g, c.b, c.a]).collect();
            assert_eq!(
                rgba,
                [
                    [255, 0, 0, 255],
                    [0, 255, 0, 0],
                    [0, 255, 0, 255],
                    [0, 255, 0, 0],
                    [0, 255, 0, 255],
                    [0, 0, 255, 255]
                ]
            );
            assert_eq!(data, original);
        }
    }

    #[test]
    fn ordinary_and_other_reserved_values_remain_opaque() {
        for flag in [0, 2, 0x10001] {
            let image = decode_image(&fixture(false, flag)).unwrap();
            assert!(image.colors().iter().all(|c| c.a == 255));
        }
    }

    #[test]
    fn unsupported_mask_hint_keeps_standard_decoding() {
        let mut data = fixture(false, 1);
        for key in [4u32, 304, u32::MAX] {
            data[50..54].copy_from_slice(&key.to_le_bytes());
            let image = decode_image(&data).unwrap();
            assert!(image.colors().iter().all(|c| c.a == 255));
            assert_eq!(image.colors()[0].r, 255);
        }
    }

    #[test]
    fn truncated_rows_are_rejected() {
        let mut data = fixture(false, 1);
        data.truncate(76);
        assert!(decode_image(&data).is_err());
    }
}
