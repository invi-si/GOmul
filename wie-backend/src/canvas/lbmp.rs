use alloc::{boxed::Box, format, vec};

use bytemuck::{Pod, Zeroable, pod_collect_to_vec, pod_read_unaligned};

use wie_util::{Result, WieError};

use crate::canvas::{ArgbPixel, Image, Rgb332Pixel, Rgb565Pixel, VecImageBuffer};

// lcd bitmap file format for skvm

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LbmpHeader {
    descriptor: u32,
    r#type: u32,
    width: u32,
    height: u32,
    size: u32,
    mask: u32,
}

pub fn decode_lbmp(data: &[u8]) -> Result<Box<dyn Image>> {
    let header: LbmpHeader = pod_read_unaligned(data.get(..24).ok_or_else(|| WieError::FatalError("Truncated LBMP header".into()))?);
    let data = &data[24..];

    if header.r#type == 2 {
        // SK-VM grayscale planes pack eight vertical pixels per byte, LSB first.
        // A trailing mask plane marks transparent pixels with set bits.
        let stride = header.width as usize;
        let plane = stride
            .checked_mul((header.height as usize).div_ceil(8))
            .ok_or_else(|| WieError::FatalError("LBMP dimensions overflow".into()))?;
        let planes = header.r#type as usize;
        let required = plane
            .checked_mul(planes + usize::from(header.mask != 0))
            .ok_or_else(|| WieError::FatalError("LBMP plane size overflow".into()))?;
        if plane != header.size as usize || data.len() < required {
            return Err(WieError::FatalError("Invalid LBMP grayscale planes".into()));
        }
        let mut pixels = vec![0u32; stride * header.height as usize];
        for y in 0..header.height as usize {
            for x in 0..stride {
                let offset = (y / 8) * stride + x;
                let bit = 1 << (y % 8);
                if header.mask != 0 && data[planes * plane + offset] & bit != 0 {
                    continue;
                }
                let mut level = 0;
                for channel in 0..planes {
                    level |= u32::from(data[channel * plane + offset] & bit != 0) << channel;
                }
                // LCD grayscale levels encode darkness: zero is white.
                let gray = 255 - level * 255 / ((1 << planes) - 1);
                pixels[y * stride + x] = 0xff000000 | gray * 0x010101;
            }
        }
        return Ok(Box::new(VecImageBuffer::<ArgbPixel>::from_raw(header.width, header.height, pixels)));
    }

    let bytes_per_pixel = match header.r#type {
        8 => 1usize,
        16 => 2usize,
        _ => return Err(WieError::Unimplemented(format!("Unsupported type {}", header.r#type))),
    };
    let expected = (header.width as usize)
        .checked_mul(header.height as usize)
        .and_then(|n| n.checked_mul(bytes_per_pixel))
        .ok_or_else(|| WieError::FatalError("LBMP dimensions overflow".into()))?;
    if header.size as usize != expected || data.len() < expected {
        return Err(WieError::FatalError("Invalid LBMP pixel data".into()));
    }
    let image: Box<dyn Image> = if header.r#type == 8 {
        Box::new(VecImageBuffer::<Rgb332Pixel>::from_raw(header.width, header.height, data.to_vec()))
    } else if header.r#type == 16 {
        Box::new(VecImageBuffer::<Rgb565Pixel>::from_raw(
            header.width,
            header.height,
            pod_collect_to_vec(data),
        ))
    } else {
        return Err(WieError::Unimplemented(format!("Unsupported type {}", header.r#type)));
    };
    if header.mask == 0 {
        return Ok(image);
    }
    let stride = header.width as usize;
    let mask_length = stride * (header.height as usize).div_ceil(8);
    let mask = data
        .get(header.size as usize..)
        .and_then(|tail| tail.get(..mask_length))
        .ok_or_else(|| WieError::FatalError("Truncated LBMP transparency mask".into()))?;
    let mut pixels = vec![0u32; stride * header.height as usize];
    for y in 0..header.height as usize {
        for x in 0..stride {
            if mask[(y / 8) * stride + x] & (1 << (y % 8)) != 0 {
                continue;
            }
            let c = image.get_pixel(x as i32, y as i32);
            pixels[y * stride + x] = 0xff000000 | ((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32;
        }
    }
    Ok(Box::new(VecImageBuffer::<ArgbPixel>::from_raw(header.width, header.height, pixels)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn color_mask_is_separate_from_pixel_values() {
        let header = LbmpHeader {
            descriptor: u32::from_le_bytes(*b"LBMP"),
            r#type: 8,
            width: 2,
            height: 1,
            size: 2,
            mask: 1,
        };
        let mut bytes = bytemuck::bytes_of(&header).to_vec();
        bytes.extend_from_slice(&[0, 0, 0, 1]);
        let image = decode_lbmp(&bytes).unwrap();
        assert_eq!(image.get_pixel(0, 0).a, 255);
        assert_eq!(image.get_pixel(0, 0).r, 0);
        assert_eq!(image.get_pixel(1, 0).a, 0);
    }
    #[test]
    fn grayscale_vertical_planes_mask_and_partial_last_row() {
        let header = LbmpHeader {
            descriptor: u32::from_le_bytes(*b"LBMP"),
            r#type: 2,
            width: 2,
            height: 9,
            size: 4,
            mask: 1,
        };
        let mut bytes = bytemuck::bytes_of(&header).to_vec();
        bytes.extend_from_slice(&[0b10, 0b10, 1, 0, 0, 0b10, 0, 1, 1, 0, 0, 0]);
        let image = decode_lbmp(&bytes).unwrap();
        assert_eq!(image.get_pixel(0, 0).a, 0);
        assert_eq!(image.get_pixel(1, 0).r, 255);
        assert_eq!(image.get_pixel(0, 1).r, 170);
        assert_eq!(image.get_pixel(1, 1).r, 0);
        assert_eq!(image.get_pixel(0, 8).r, 170);
        assert_eq!(image.get_pixel(1, 8).r, 85);
        assert!(decode_lbmp(&bytes[..bytes.len() - 1]).is_err());
        assert!(decode_lbmp(b"LBMP").is_err());
    }

    #[test]
    fn grayscale_white_and_black_glyphs_preserve_the_same_transparency() {
        let header = LbmpHeader {
            descriptor: u32::from_le_bytes(*b"LBMP"),
            r#type: 2,
            width: 2,
            height: 1,
            size: 2,
            mask: 1,
        };
        for (ink, expected) in [(0, 255), (1, 0)] {
            let mut bytes = bytemuck::bytes_of(&header).to_vec();
            bytes.extend_from_slice(&[ink, 0, ink, 0, 0, 1]);
            let image = decode_lbmp(&bytes).unwrap();
            let visible = image.get_pixel(0, 0);
            assert_eq!((visible.a, visible.r, visible.g, visible.b), (255, expected, expected, expected));
            assert_eq!(image.get_pixel(1, 0).a, 0);
        }
    }
}
