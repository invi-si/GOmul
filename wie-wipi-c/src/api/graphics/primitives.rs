#![allow(clippy::too_many_arguments)]

use alloc::{string::String, vec};

use wie_backend::canvas::{Clip, Color, Image, PixelType, Rgb8Pixel};
use wie_util::{Result, read_null_terminated_string_bytes};

use crate::{WIPICContext, api::graphics::FrameBuffer};

pub fn read_text(context: &dyn WIPICContext, address: u32, length: i32) -> Result<Option<String>> {
    read_text_impl(context, address, length, false)
}

pub fn read_text_for_measurement(context: &dyn WIPICContext, address: u32, length: i32) -> Result<Option<String>> {
    read_text_impl(context, address, length, true)
}

fn read_text_impl(context: &dyn WIPICContext, address: u32, length: i32, measuring: bool) -> Result<Option<String>> {
    let bytes = if length == -1 {
        read_null_terminated_string_bytes(context, address)?
    } else if length >= 0 {
        let mut bytes = vec![0; length as usize];
        context.read_bytes(address, &mut bytes)?;
        bytes
    } else {
        return Ok(None);
    };

    let mut text = encoding_rs::EUC_KR.decode(&bytes).0.into_owned();
    if measuring && length >= 0 && text.ends_with('\u{fffd}') {
        // Byte-by-byte width probes can end at a Korean lead byte. Reserve its
        // full cell now, so wrapping does not accept half a character. Decode
        // without EOF to distinguish an incomplete pair from malformed input.
        let mut decoder = encoding_rs::EUC_KR.new_decoder_without_bom_handling();
        let mut prefix = String::with_capacity(decoder.max_utf8_buffer_length(bytes.len()).unwrap());
        let (_, consumed, _) = decoder.decode_to_string(&bytes, &mut prefix, false);
        if consumed == bytes.len() && text.strip_suffix('\u{fffd}') == Some(prefix.as_str()) {
            text.pop();
            text.push('\u{3000}');
        }
    }
    Ok(Some(text))
}

fn write_canvas<F>(context: &mut dyn WIPICContext, framebuffer: &FrameBuffer, operation: F) -> Result<()>
where
    F: FnOnce(&mut dyn wie_backend::canvas::Canvas),
{
    let mut canvas = framebuffer.canvas(context)?;
    operation(&mut **canvas);
    canvas.flush()
}

pub fn put_pixel(context: &mut dyn WIPICContext, framebuffer: &FrameBuffer, x: i32, y: i32, color: Color, clip: Clip) -> Result<()> {
    write_canvas(context, framebuffer, |canvas| canvas.put_pixel(x, y, color, clip))
}

pub fn fill_rect(
    context: &mut dyn WIPICContext,
    framebuffer: &FrameBuffer,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    color: Color,
    clip: Clip,
) -> Result<()> {
    write_canvas(context, framebuffer, |canvas| canvas.fill_rect(x, y, width, height, color, clip))
}

/// Even-odd polygon fill, sampling pixel centers and excluding upper edge endpoints.
pub fn fill_polygon(context: &mut dyn WIPICContext, framebuffer: &FrameBuffer, points: &[(i32, i32)], color: Color, clip: Clip) -> Result<()> {
    if points.len() < 3 {
        return Ok(());
    }
    let bounds = Clip {
        x: 0,
        y: 0,
        width: framebuffer.0.width,
        height: framebuffer.0.height,
    };
    let clip = clip.intersect(&bounds);
    let mut intersections = alloc::vec::Vec::with_capacity(points.len());
    write_canvas(context, framebuffer, |canvas| {
        for y in i64::from(clip.y)..i64::from(clip.y) + i64::from(clip.height) {
            intersections.clear();
            for i in 0..points.len() {
                let (mut x1, mut y1) = points[i];
                let (mut x2, mut y2) = points[(i + 1) % points.len()];
                if y1 > y2 {
                    core::mem::swap(&mut x1, &mut x2);
                    core::mem::swap(&mut y1, &mut y2);
                }
                if y < i64::from(y1) || y >= i64::from(y2) {
                    continue;
                }
                let dy = i128::from(y2) - i128::from(y1);
                let denominator = 2 * dy;
                let numerator = i128::from(x1) * denominator + (2 * i128::from(y) + 1 - 2 * i128::from(y1)) * (i128::from(x2) - i128::from(x1));
                // ceil(intersection - 0.5): first pixel center on/right of the edge.
                let edge = -(-(numerator - dy)).div_euclid(denominator);
                intersections.push(edge);
            }
            intersections.sort_unstable();
            for pair in intersections.chunks_exact(2) {
                let left = pair[0].max(i128::from(clip.x));
                let right = pair[1].min(i128::from(clip.x) + i128::from(clip.width));
                if right > left {
                    canvas.fill_rect(left as i32, y as i32, (right - left) as u32, 1, color, clip);
                }
            }
        }
    })
}

pub fn draw_line(
    context: &mut dyn WIPICContext,
    framebuffer: &FrameBuffer,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    color: Color,
    clip: Clip,
) -> Result<()> {
    write_canvas(context, framebuffer, |canvas| canvas.draw_line(x1, y1, x2, y2, color, clip))
}

pub fn draw_rect(
    context: &mut dyn WIPICContext,
    framebuffer: &FrameBuffer,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    color: Color,
    clip: Clip,
) -> Result<()> {
    write_canvas(context, framebuffer, |canvas| canvas.draw_rect(x, y, width, height, color, clip))
}

pub fn draw_arc(
    context: &mut dyn WIPICContext,
    framebuffer: &FrameBuffer,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    start_angle: i32,
    arc_angle: i32,
    color: Color,
    clip: Clip,
) -> Result<()> {
    write_canvas(context, framebuffer, |canvas| {
        canvas.draw_arc(x, y, width, height, start_angle, arc_angle, color, clip)
    })
}

pub fn fill_arc(
    context: &mut dyn WIPICContext,
    framebuffer: &FrameBuffer,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    start_angle: i32,
    arc_angle: i32,
    color: Color,
    clip: Clip,
) -> Result<()> {
    write_canvas(context, framebuffer, |canvas| {
        canvas.fill_arc(x, y, width, height, start_angle, arc_angle, color, clip)
    })
}

/// Invoke the guest pixel operation for each visible pixel. The ABI passes the
/// existing framebuffer pixel first, the incoming pixel second, then param1.
/// Keep writes in guest memory between callbacks: callbacks may read that memory
/// or fail, and earlier completed pixels must remain observable.
pub async fn draw_image_with_pixel_op(
    context: &mut dyn WIPICContext,
    framebuffer: &FrameBuffer,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    image: alloc::boxed::Box<dyn Image>,
    source_x: i32,
    source_y: i32,
    clip: Clip,
    pixel_op: u32,
    param: u32,
) -> Result<()> {
    if pixel_op == 0 {
        return draw_image(context, framebuffer, x, y, width, height, &*image, source_x, source_y, clip);
    }
    use wie_backend::canvas::{ArgbPixel, Rgb565Pixel};
    use wie_util::{WieError, read_generic, write_generic};
    let bytes_per_pixel = match framebuffer.0.bpp {
        16 => 2u32,
        32 => 4u32,
        _ => return Err(WieError::FatalError("Unsupported pixel operation framebuffer depth".into())),
    };
    let row_bytes = framebuffer.0.width.checked_mul(bytes_per_pixel).ok_or(WieError::AllocationFailure)?;
    if framebuffer.0.bpl < row_bytes {
        return Err(WieError::FatalError("Invalid pixel operation framebuffer stride".into()));
    }
    let base = context.data_ptr(framebuffer.0.buf)?;
    let size = framebuffer.0.bpl.checked_mul(framebuffer.0.height).ok_or(WieError::AllocationFailure)?;
    base.checked_add(size).ok_or(WieError::InvalidMemoryAccess(base))?;
    let left = 0i64.max(-(x as i64)).max(-(source_x as i64)).max(clip.x as i64 - x as i64);
    let top = 0i64.max(-(y as i64)).max(-(source_y as i64)).max(clip.y as i64 - y as i64);
    let right = (width as i64)
        .min(framebuffer.0.width as i64 - x as i64)
        .min(image.width() as i64 - source_x as i64)
        .min(clip.x as i64 + clip.width as i64 - x as i64);
    let bottom = (height as i64)
        .min(framebuffer.0.height as i64 - y as i64)
        .min(image.height() as i64 - source_y as i64)
        .min(clip.y as i64 + clip.height as i64 - y as i64);
    for row in top..bottom {
        for column in left..right {
            let address = base + (y as i64 + row) as u32 * framebuffer.0.bpl + (x as i64 + column) as u32 * bytes_per_pixel;
            let color = image.get_pixel((source_x as i64 + column) as i32, (source_y as i64 + row) as i32);
            let (old, incoming) = if bytes_per_pixel == 2 {
                (read_generic::<u16, _>(context, address)? as u32, Rgb565Pixel::from_color(color) as u32)
            } else {
                (read_generic::<u32, _>(context, address)?, ArgbPixel::from_color(color))
            };
            let result = context.call_function(pixel_op, &[old, incoming, param]).await?;
            if bytes_per_pixel == 2 {
                write_generic(context, address, result as u16)?;
            } else {
                write_generic(context, address, result)?;
            }
        }
    }
    Ok(())
}

pub fn draw_image(
    context: &mut dyn WIPICContext,
    framebuffer: &FrameBuffer,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    image: &dyn Image,
    source_x: i32,
    source_y: i32,
    clip: Clip,
) -> Result<()> {
    if framebuffer.try_draw_image(context, x, y, width, height, image, source_x, source_y, clip)? {
        return Ok(());
    }
    write_canvas(context, framebuffer, |canvas| {
        canvas.draw(x, y, width, height, image, source_x, source_y, clip)
    })
}

pub fn copy_area(
    context: &mut dyn WIPICContext,
    framebuffer: &FrameBuffer,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    source_x: i32,
    source_y: i32,
    clip: Clip,
) -> Result<()> {
    let image = framebuffer.image(context)?;
    draw_image(context, framebuffer, x, y, width, height, &*image, source_x, source_y, clip)
}

pub fn copy_framebuffer(
    context: &mut dyn WIPICContext,
    destination: &FrameBuffer,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    source: &FrameBuffer,
    source_x: i32,
    source_y: i32,
    clip: Clip,
) -> Result<()> {
    let image = source.image(context)?;
    draw_image(context, destination, x, y, width, height, &*image, source_x, source_y, clip)
}

pub fn draw_text(context: &mut dyn WIPICContext, framebuffer: &FrameBuffer, string: &str, x: i32, y: i32, color: Color, clip: Clip) -> Result<()> {
    let font = context.system().platform().font().clone();
    write_canvas(context, framebuffer, |canvas| {
        canvas.draw_text(&font, string, x, y, wie_backend::canvas::TextAlignment::Left, color, clip)
    })
}

fn rgb_row_bytes(width: i32, stride: i32) -> Option<usize> {
    if width <= 0 || stride <= 0 {
        return None;
    }

    let row_bytes = (width as usize).checked_mul(4)?;
    (stride as usize >= row_bytes).then_some(row_bytes)
}

pub fn get_rgb_pixels(
    context: &mut dyn WIPICContext,
    image: &dyn Image,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    destination: u32,
    destination_bpl: i32,
) -> Result<()> {
    if height <= 0 {
        return Ok(());
    }
    let Some(row_bytes) = rgb_row_bytes(width, destination_bpl) else {
        return Ok(());
    };

    let mut row = vec![0; row_bytes];
    for row_index in 0..height {
        for column in 0..width {
            let source_x = x.wrapping_add(column);
            let source_y = y.wrapping_add(row_index);
            let color = if source_x < 0 || source_y < 0 || source_x >= image.width() as i32 || source_y >= image.height() as i32 {
                Color { a: 0, r: 0, g: 0, b: 0 }
            } else {
                image.get_pixel(source_x, source_y)
            };
            let offset = column as usize * 4;
            row[offset..offset + 4].copy_from_slice(&Rgb8Pixel::from_color(color).to_le_bytes());
        }
        let address = destination
            .checked_add(
                (row_index as u32)
                    .checked_mul(destination_bpl as u32)
                    .ok_or(wie_util::WieError::AllocationFailure)?,
            )
            .ok_or(wie_util::WieError::AllocationFailure)?;
        context.write_bytes(address, &row)?;
    }
    Ok(())
}

pub fn set_rgb_pixels(
    context: &mut dyn WIPICContext,
    framebuffer: &FrameBuffer,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    source: u32,
    source_bpl: i32,
    clip: Clip,
) -> Result<()> {
    if height <= 0 {
        return Ok(());
    }
    let Some(row_bytes) = rgb_row_bytes(width, source_bpl) else {
        return Ok(());
    };
    let Some(total_bytes) = row_bytes.checked_mul(height as usize) else {
        return Ok(());
    };

    let mut pixels = vec![0; total_bytes];
    for row_index in 0..height {
        let address = source
            .checked_add(
                (row_index as u32)
                    .checked_mul(source_bpl as u32)
                    .ok_or(wie_util::WieError::AllocationFailure)?,
            )
            .ok_or(wie_util::WieError::AllocationFailure)?;
        let offset = row_index as usize * row_bytes;
        context.read_bytes(address, &mut pixels[offset..offset + row_bytes])?;
    }

    write_canvas(context, framebuffer, |canvas| {
        for row_index in 0..height {
            let row_offset = row_index as usize * row_bytes;
            let row = &pixels[row_offset..row_offset + row_bytes];
            for column in 0..width {
                let offset = column as usize * 4;
                let rgb = u32::from_le_bytes([row[offset], row[offset + 1], row[offset + 2], row[offset + 3]]);
                canvas.put_pixel(x.wrapping_add(column), y.wrapping_add(row_index), Rgb8Pixel::to_color(rgb), clip);
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use wie_backend::canvas::{Clip, Color};
    use wie_util::{ByteRead, ByteWrite, Result};

    use crate::{
        api::graphics::{
            FrameBuffer,
            primitives::{fill_rect, get_rgb_pixels, put_pixel, set_rgb_pixels},
        },
        context::{WIPICContext, test::TestContext},
    };

    #[futures_test::test]
    async fn pixel_callback_preserves_key_and_receives_native_pixels_after_clipping() -> Result<()> {
        use alloc::vec;
        use wie_backend::canvas::{Rgb565Pixel, VecImageBuffer};
        use wie_util::write_generic;
        let mut context = TestContext::new();
        context.pixel_callback = Some(|_, address, args| {
            assert_eq!(address, 0x101);
            Ok(if args[1] == args[2] { args[0] } else { args[1] })
        });
        let fb = FrameBuffer::new(&mut context, 4, 1, 16)?;
        for i in 0..4 {
            write_generic(&mut context, fb.0.buf.0 + i * 2, 0x07e0u16)?;
        }
        let source = VecImageBuffer::<Rgb565Pixel>::from_raw(4, 1, vec![0xf800, 0xf81f, 0x001f, 0xffff]);
        super::draw_image_with_pixel_op(
            &mut context,
            &fb,
            -1,
            0,
            4,
            1,
            alloc::boxed::Box::new(source),
            0,
            0,
            Clip {
                x: 0,
                y: 0,
                width: 2,
                height: 1,
            },
            0x101,
            0xf81f,
        )
        .await?;
        assert_eq!(
            context.calls,
            vec![(0x101, vec![0x07e0, 0xf81f, 0xf81f]), (0x101, vec![0x07e0, 0x001f, 0xf81f])]
        );
        assert_eq!(fb.data(&context)?, vec![0xe0, 7, 0x1f, 0, 0xe0, 7, 0xe0, 7]);
        // No callback means ordinary magenta artwork must remain visible.
        super::draw_image_with_pixel_op(
            &mut context,
            &fb,
            0,
            0,
            1,
            1,
            alloc::boxed::Box::new(VecImageBuffer::<Rgb565Pixel>::from_raw(1, 1, vec![0xf81f])),
            0,
            0,
            Clip {
                x: 0,
                y: 0,
                width: 4,
                height: 1,
            },
            0,
            0,
        )
        .await?;
        assert_eq!(&fb.data(&context)?[..2], &[0x1f, 0xf8]);
        Ok(())
    }

    #[futures_test::test]
    async fn pixel_callback_writes_are_visible_and_faults_preserve_completed_pixels() -> Result<()> {
        use alloc::vec;
        use wie_backend::canvas::{ArgbPixel, VecImageBuffer};
        use wie_util::{WieError, read_generic};
        let mut context = TestContext::new();
        let fb = FrameBuffer::new(&mut context, 3, 1, 32)?;
        context.pixel_callback = Some(|context, _, args| {
            if context.calls.len() == 2 {
                // Test allocator starts at 0x10000; no extra allocations occur.
                assert_eq!(read_generic::<u32, _>(context, 0x10000)?, 0xff123456);
                return Err(WieError::InvalidMemoryAccess(0xdead));
            }
            Ok(args[1])
        });
        let source = VecImageBuffer::<ArgbPixel>::from_raw(3, 1, vec![0xff123456, 0xffabcdef, 0xffffffff]);
        let result = super::draw_image_with_pixel_op(
            &mut context,
            &fb,
            0,
            0,
            3,
            1,
            alloc::boxed::Box::new(source),
            0,
            0,
            Clip {
                x: 0,
                y: 0,
                width: 3,
                height: 1,
            },
            0x101,
            0,
        )
        .await;
        assert!(matches!(result, Err(WieError::InvalidMemoryAccess(0xdead))));
        assert_eq!(context.calls.len(), 2);
        assert_eq!(read_generic::<u32, _>(&context, fb.0.buf.0)?, 0xff123456);
        assert_eq!(read_generic::<u32, _>(&context, fb.0.buf.0 + 4)?, 0);
        Ok(())
    }

    #[test]
    fn polygon_fill_handles_concavity_winding_clipping_and_extreme_coordinates() -> Result<()> {
        let red = Color { a: 255, r: 255, g: 0, b: 0 };
        let clip = Clip {
            x: 0,
            y: 0,
            width: 4,
            height: 4,
        };
        for bpp in [16, 32] {
            let mut points = alloc::vec![(0, 0), (4, 0), (4, 2), (2, 2), (2, 4), (0, 4)];
            for _ in 0..2 {
                let mut context = TestContext::new();
                let framebuffer = FrameBuffer::new(&mut context, 4, 4, bpp)?;
                super::fill_polygon(&mut context, &framebuffer, &points, red, clip)?;
                let image = framebuffer.image(&mut context)?;
                for y in 0..4 {
                    for x in 0..4 {
                        assert_eq!(image.get_pixel(x, y).r, if x < 2 || y < 2 { 255 } else { 0 });
                    }
                }
                points.reverse();
            }
            let mut context = TestContext::new();
            let framebuffer = FrameBuffer::new(&mut context, 4, 4, bpp)?;
            let huge = [(i32::MIN, i32::MIN), (i32::MAX, i32::MIN), (i32::MAX, i32::MAX), (i32::MIN, i32::MAX)];
            super::fill_polygon(
                &mut context,
                &framebuffer,
                &huge,
                red,
                Clip {
                    x: 1,
                    y: 1,
                    width: 2,
                    height: 2,
                },
            )?;
            super::fill_polygon(&mut context, &framebuffer, &[(0, 0), (3, 3)], red, clip)?;
            let image = framebuffer.image(&mut context)?;
            for y in 0..4 {
                for x in 0..4 {
                    assert_eq!(image.get_pixel(x, y).r, if (1..3).contains(&x) && (1..3).contains(&y) { 255 } else { 0 });
                }
            }
        }
        Ok(())
    }

    #[test]
    fn drawing_primitives_write_the_guest_framebuffer() -> Result<()> {
        let mut context = TestContext::new();
        let framebuffer = FrameBuffer::new(&mut context, 4, 4, 16)?;
        let clip = Clip {
            x: 0,
            y: 0,
            width: 4,
            height: 4,
        };
        let red = Color {
            a: 0xff,
            r: 0xff,
            g: 0,
            b: 0,
        };

        fill_rect(&mut context, &framebuffer, 1, 1, 2, 2, red, clip)?;
        put_pixel(&mut context, &framebuffer, 0, 0, red, clip)?;

        let image = framebuffer.image(&mut context)?;
        assert_eq!(image.get_pixel(0, 0).r, 255);
        assert_eq!(image.get_pixel(1, 1).r, 255);
        assert_eq!(image.get_pixel(3, 3).r, 0);
        Ok(())
    }

    #[test]
    fn overlapping_copy_uses_original_source_pixels_in_both_directions() -> Result<()> {
        let clip = Clip {
            x: 0,
            y: 0,
            width: 12,
            height: 10,
        };
        for bpp in [16, 32] {
            let mut context = TestContext::new();
            let framebuffer = FrameBuffer::new(&mut context, 12, 10, bpp)?;
            let mut reference_context = TestContext::new();
            let reference = FrameBuffer::new(&mut reference_context, 12, 10, bpp)?;
            let initial = (0..12 * 10 * (bpp / 8)).map(|i| (i * 31) as u8).collect::<alloc::vec::Vec<_>>();
            framebuffer.write(&mut context, &initial)?;
            reference.write(&mut reference_context, &initial)?;
            for (x, y, sx, sy) in [(2, 1, 0, 0), (0, 0, 2, 1), (2, 0, 0, 1), (0, 1, 2, 0)] {
                let source = reference.image(&mut reference_context)?;
                let mut canvas = reference.canvas(&mut reference_context)?;
                canvas.draw(x, y, 10, 9, &*source, sx, sy, clip);
                canvas.flush()?;
                super::copy_area(&mut context, &framebuffer, x, y, 10, 9, sx, sy, clip)?;
                assert_eq!(
                    framebuffer.data(&context)?,
                    reference.data(&reference_context)?,
                    "bpp={bpp}, {x},{y} <- {sx},{sy}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn rgb_pixels_use_the_wipi_little_endian_layout() -> Result<()> {
        let mut context = TestContext::new();
        let framebuffer = FrameBuffer::new(&mut context, 1, 1, 16)?;
        put_pixel(
            &mut context,
            &framebuffer,
            0,
            0,
            Color {
                a: 0xff,
                r: 0xff,
                g: 0,
                b: 0,
            },
            Clip {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
        )?;
        let image = framebuffer.image(&mut context)?;

        get_rgb_pixels(&mut context, &*image, 0, 0, 1, 1, 0x1000, 4)?;
        let mut bytes = [0; 4];
        context.read_bytes(0x1000, &mut bytes)?;
        assert_eq!(u32::from_le_bytes(bytes), 0x00ff_0000);
        Ok(())
    }

    #[test]
    fn rgb_row_bytes_rejects_overflow_and_short_strides() {
        assert_eq!(super::rgb_row_bytes(i32::MAX, i32::MAX), None);
        assert_eq!(super::rgb_row_bytes(4, 15), None);
        assert_eq!(super::rgb_row_bytes(4, 16), Some(16));
    }

    #[test]
    fn set_rgb_pixels_wraps_guest_coordinates() -> Result<()> {
        let mut context = TestContext::new();
        let framebuffer = FrameBuffer::new(&mut context, 1, 1, 16)?;
        let source = context.alloc(8)?;
        context.write_bytes(context.data_ptr(source)?, &[0, 0, 0, 0, 0, 0, 0, 0])?;

        set_rgb_pixels(
            &mut context,
            &framebuffer,
            i32::MAX,
            0,
            2,
            1,
            source.0,
            8,
            Clip {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
        )
    }
}
