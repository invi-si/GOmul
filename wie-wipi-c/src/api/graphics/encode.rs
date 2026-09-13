use alloc::vec;
use wie_backend::canvas::{ArgbPixel, PixelType, Rgb565Pixel};
use wie_util::{Result, WieError, read_generic, write_generic};
use wipi_types::wipic::{WIPICFramebuffer, WIPICIndirectPtr};

use crate::context::WIPICContext;

/// MC_grpEncodeImage returns an owned BMP memory ID, or NULL on failure.
/// The source remains unchanged. BMP rows are bottom-up BGR with DWORD padding.
pub async fn encode_image(
    context: &mut dyn WIPICContext,
    source: WIPICIndirectPtr,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    length: u32,
) -> Result<WIPICIndirectPtr> {
    if source.0 == 0 || length == 0 || x < 0 || y < 0 || width <= 0 || height <= 0 {
        return Ok(WIPICIndirectPtr(0));
    }
    let source: WIPICFramebuffer = read_generic(context, context.data_ptr(source)?)?;
    let (x, y, width, height) = (x as u32, y as u32, width as u32, height as u32);
    let bytes = source.bpp / 8;
    let stride = (u64::from(width) * 3 + 3) & !3;
    let size = 54 + stride * u64::from(height);
    if !matches!(source.bpp, 16 | 32)
        || u64::from(x) + u64::from(width) > u64::from(source.width)
        || u64::from(y) + u64::from(height) > u64::from(source.height)
        || u64::from(source.bpl) < u64::from(source.width) * u64::from(bytes)
        || size > u64::from(context.total_memory()).min(i32::MAX as u64)
    {
        return Ok(WIPICIndirectPtr(0));
    }
    let base = context.data_ptr(source.buf)?;
    let source_end = u64::from(base) + u64::from(source.bpl) * u64::from(source.height);
    if source_end > u64::from(u32::MAX) {
        return Err(WieError::InvalidMemoryAccess(base));
    }
    let output = match context.alloc(size as u32) {
        Ok(memory) => memory,
        Err(WieError::AllocationFailure) => return Ok(WIPICIndirectPtr(0)),
        Err(error) => return Err(error),
    };
    let result = (|| -> Result<()> {
        let destination = context.data_ptr(output)?;
        let mut header = [0u8; 54];
        header[..2].copy_from_slice(b"BM");
        header[2..6].copy_from_slice(&(size as u32).to_le_bytes());
        header[10..14].copy_from_slice(&54u32.to_le_bytes());
        header[14..18].copy_from_slice(&40u32.to_le_bytes());
        header[18..22].copy_from_slice(&width.to_le_bytes());
        header[22..26].copy_from_slice(&height.to_le_bytes());
        header[26..28].copy_from_slice(&1u16.to_le_bytes());
        header[28..30].copy_from_slice(&24u16.to_le_bytes());
        header[34..38].copy_from_slice(&((size - 54) as u32).to_le_bytes());
        context.write_bytes(destination, &header)?;
        let mut input = vec![0u8; (width * bytes) as usize];
        let mut row = vec![0u8; stride as usize];
        for row_index in 0..height {
            let address = base + (y + height - 1 - row_index) * source.bpl + x * bytes;
            context.read_bytes(address, &mut input)?;
            for (pixel, target) in input.chunks_exact(bytes as usize).zip(row.as_chunks_mut::<3>().0.iter_mut()) {
                let color = if bytes == 2 {
                    Rgb565Pixel::to_color(u16::from_le_bytes(pixel.try_into().unwrap()))
                } else {
                    ArgbPixel::to_color(u32::from_le_bytes(pixel.try_into().unwrap()))
                };
                target.copy_from_slice(&[color.b, color.g, color.r]);
            }
            context.write_bytes(destination + 54 + row_index * stride as u32, &row)?;
        }
        write_generic(context, length, size as u32)
    })();
    if let Err(error) = result {
        context.free(output)?;
        return Err(error);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::test::TestContext;
    use wie_backend::canvas::{Color, decode_image};
    use wie_util::{ByteRead, ByteWrite};

    #[futures_test::test]
    async fn bmp_crop_roundtrips_both_formats_with_padded_source_rows() -> Result<()> {
        for bpp in [16, 32] {
            let mut context = TestContext::new();
            let pixels = context.alloc(64)?;
            let buffer = context.alloc(20)?;
            let length = context.alloc_raw(4)?;
            let stride = 3 * (bpp / 8) + 4;
            let mut original = [0xa5; 64];
            let mut expected = vec![];
            for y in 0..2 {
                for x in 0..3 {
                    let color = Color {
                        a: 255,
                        r: (x * 80) as u8,
                        g: (y * 160) as u8,
                        b: 80,
                    };
                    let offset = (y * stride + x * (bpp / 8)) as usize;
                    if bpp == 16 {
                        let pixel = Rgb565Pixel::from_color(color);
                        original[offset..offset + 2].copy_from_slice(&pixel.to_le_bytes());
                        if x == 1 {
                            expected.push(Rgb565Pixel::to_color(pixel));
                        }
                    } else {
                        original[offset..offset + 4].copy_from_slice(&ArgbPixel::from_color(color).to_le_bytes());
                        if x == 1 {
                            expected.push(color);
                        }
                    }
                }
            }
            context.write_bytes(pixels.0, &original)?;
            write_generic(
                &mut context,
                buffer.0,
                WIPICFramebuffer {
                    width: 3,
                    height: 2,
                    bpl: stride,
                    bpp,
                    buf: pixels,
                },
            )?;
            let output = encode_image(&mut context, buffer, 1, 0, 1, 2, length).await?;
            let size: u32 = read_generic(&context, length)?;
            assert_eq!(size, 62);
            let mut bmp = vec![0; size as usize];
            context.read_bytes(output.0, &mut bmp)?;
            assert_eq!((bmp[57], bmp[61]), (0, 0));
            let decoded = decode_image(&bmp)?;
            assert_eq!((decoded.width(), decoded.height()), (1, 2));
            for y in 0..2 {
                assert_eq!(
                    ArgbPixel::from_color(decoded.get_pixel(0, y)),
                    ArgbPixel::from_color(expected[y as usize])
                );
            }
            let mut unchanged = [0; 64];
            context.read_bytes(pixels.0, &mut unchanged)?;
            assert_eq!(original, unchanged);
            write_generic(&mut context, length, 0x12345678u32)?;
            for (x, w) in [(-1, 1), (3, 1), (0, 0), (0, i32::MAX)] {
                assert_eq!(encode_image(&mut context, buffer, x, 0, w, 2, length).await?.0, 0);
                assert_eq!(read_generic::<u32, _>(&context, length)?, 0x12345678);
            }
            let available = context.free_memory()?;
            context.alloc(available)?;
            assert_eq!(encode_image(&mut context, buffer, 1, 0, 1, 2, length).await?.0, 0);
            assert_eq!(read_generic::<u32, _>(&context, length)?, 0x12345678);
        }
        Ok(())
    }
}
