use alloc::{boxed::Box, vec, vec::Vec};
use core::ops::{Deref, DerefMut};

use bytemuck::pod_collect_to_vec;

use wipi_types::wipic::{WIPICFramebuffer, WIPICIndirectPtr, WIPICWord};

use wie_backend::canvas::{ArgbPixel, Canvas, Clip, Color, Image, ImageBufferCanvas, PixelType, Rgb8Pixel, Rgb565Pixel, VecImageBuffer};
use wie_util::{Result, WieError};

use crate::context::WIPICContext;

// same 256MB as wie_core_arm's HEAP_SIZE; not referenced directly to avoid the dependency
const MAX_FRAMEBUFFER_BYTES: u32 = 0x1000_0000;

fn buffer_size(width: u32, height: u32, bytes_per_pixel: u32) -> Result<(u32, u32)> {
    let bpl = width.checked_mul(bytes_per_pixel).ok_or(WieError::AllocationFailure)?;
    let size = bpl.checked_mul(height).ok_or(WieError::AllocationFailure)?;
    if size > MAX_FRAMEBUFFER_BYTES {
        return Err(WieError::AllocationFailure);
    }

    Ok((size, bpl))
}

pub struct FrameBuffer(pub WIPICFramebuffer);

impl FrameBuffer {
    pub fn empty() -> Self {
        Self(WIPICFramebuffer {
            width: 0,
            height: 0,
            bpl: 0,
            bpp: 0,
            buf: WIPICIndirectPtr(0),
        })
    }

    pub fn new(context: &mut dyn WIPICContext, width: WIPICWord, height: WIPICWord, bpp: WIPICWord) -> Result<Self> {
        let bytes_per_pixel = bpp / 8;

        let (size, bpl) = buffer_size(width, height, bytes_per_pixel)?;
        let buf = context.alloc(size)?;

        Ok(Self(WIPICFramebuffer {
            width,
            height,
            bpl,
            bpp: bytes_per_pixel * 8,
            buf,
        }))
    }

    pub fn from_image(context: &mut dyn WIPICContext, image: &dyn Image) -> Result<Self> {
        let (size, bpl) = buffer_size(image.width(), image.height(), image.bytes_per_pixel())?;
        let buf = context.alloc(size)?;

        context.write_bytes(context.data_ptr(buf)?, &image.raw())?;

        Ok(Self(WIPICFramebuffer {
            width: image.width(),
            height: image.height(),
            bpl,
            bpp: image.bytes_per_pixel() * 8,
            buf,
        }))
    }

    pub fn data(&self, context: &dyn WIPICContext) -> Result<Vec<u8>> {
        let (size, _) = buffer_size(self.0.width, self.0.height, self.0.bpp / 8)?;
        let mut buf = vec![0; size as _];
        context.read_bytes(context.data_ptr(self.0.buf)?, &mut buf)?;

        Ok(buf)
    }

    pub fn image(&self, context: &mut dyn WIPICContext) -> Result<Box<dyn Image>> {
        let data = self.data(context)?;

        Ok(match self.0.bpp {
            16 => Box::new(VecImageBuffer::<Rgb565Pixel>::from_raw(
                self.0.width as _,
                self.0.height as _,
                pod_collect_to_vec(&data),
            )),
            32 => Box::new(VecImageBuffer::<ArgbPixel>::from_raw(
                self.0.width as _,
                self.0.height as _,
                pod_collect_to_vec(&data),
            )),
            _ => unimplemented!("Unsupported pixel format: {}", self.0.bpp),
        })
    }

    pub fn canvas<'a>(&'a self, context: &'a mut dyn WIPICContext) -> Result<FramebufferCanvas<'a>> {
        let data = self.data(context)?;

        let canvas: Box<dyn Canvas> = match self.0.bpp {
            16 => Box::new(ImageBufferCanvas::new(VecImageBuffer::<Rgb565Pixel>::from_raw(
                self.0.width as _,
                self.0.height as _,
                pod_collect_to_vec(&data),
            ))),
            32 => Box::new(ImageBufferCanvas::new(VecImageBuffer::<ArgbPixel>::from_raw(
                self.0.width as _,
                self.0.height as _,
                pod_collect_to_vec(&data),
            ))),
            _ => unimplemented!("Unsupported pixel format: {}", self.0.bpp),
        };

        Ok(FramebufferCanvas {
            framebuffer: self,
            context,
            canvas,
            flushed: false,
        })
    }

    pub fn write(&self, context: &mut dyn WIPICContext, data: &[u8]) -> Result<()> {
        context.write_bytes(context.data_ptr(self.0.buf)?, data)
    }

    /// Materialize only the pixels touched by a blit. The source is already a
    /// snapshot, so overlapping guest source/destination buffers remain safe.
    #[allow(clippy::too_many_arguments)]
    pub fn try_draw_image(
        &self,
        context: &mut dyn WIPICContext,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        image: &dyn Image,
        source_x: i32,
        source_y: i32,
        clip: Clip,
    ) -> Result<bool> {
        if !matches!(self.0.bpp, 16 | 32) {
            return Ok(false);
        }

        // Intersect in offset coordinates with i64 arithmetic, including source
        // bounds before allocating or converting any guest coordinates.
        let left = 0i64.max(-(x as i64)).max(-(source_x as i64)).max(clip.x as i64 - x as i64);
        let top = 0i64.max(-(y as i64)).max(-(source_y as i64)).max(clip.y as i64 - y as i64);
        let right = (width as i64)
            .min(self.0.width as i64 - x as i64)
            .min(image.width() as i64 - source_x as i64)
            .min(clip.x as i64 + clip.width as i64 - x as i64);
        let bottom = (height as i64)
            .min(self.0.height as i64 - y as i64)
            .min(image.height() as i64 - source_y as i64)
            .min(clip.y as i64 + clip.height as i64 - y as i64);
        if left >= right || top >= bottom {
            return Ok(true);
        }
        let area = Clip {
            x: (x as i64 + left) as i32,
            y: (y as i64 + top) as i32,
            width: (right - left) as u32,
            height: (bottom - top) as u32,
        };
        let source_x = (source_x as i64 + left) as i32;
        let source_y = (source_y as i64 + top) as i32;
        match self.0.bpp {
            16 => self.draw_image_region::<Rgb565Pixel>(context, area, image, source_x, source_y)?,
            _ => self.draw_image_region::<ArgbPixel>(context, area, image, source_x, source_y)?,
        }
        Ok(true)
    }

    fn draw_image_region<T: PixelType + 'static>(
        &self,
        context: &mut dyn WIPICContext,
        area: Clip,
        image: &dyn Image,
        source_x: i32,
        source_y: i32,
    ) -> Result<()> {
        let bytes_per_pixel = self.0.bpp / 8;
        let (_, packed_bpl) = buffer_size(self.0.width, self.0.height, bytes_per_pixel)?;
        let backing_size = self.0.bpl.checked_mul(self.0.height).ok_or(WieError::AllocationFailure)?;
        if self.0.bpl < packed_bpl || backing_size > MAX_FRAMEBUFFER_BYTES {
            return Err(WieError::AllocationFailure);
        }
        let base = context.data_ptr(self.0.buf)?;
        base.checked_add(backing_size).ok_or(WieError::InvalidMemoryAccess(base))?;
        let row_bytes = (area.width * bytes_per_pixel) as usize;
        let mut pixels = vec![T::from_color(Color { a: 0, r: 0, g: 0, b: 0 }); (area.width * area.height) as usize];
        for (row, bytes) in bytemuck::cast_slice_mut(&mut pixels).chunks_exact_mut(row_bytes).enumerate() {
            let address = base + (area.y as u32 + row as u32) * self.0.bpl + area.x as u32 * bytes_per_pixel;
            context.read_bytes(address, bytes)?;
        }

        let mut canvas = ImageBufferCanvas::new(VecImageBuffer::<T>::from_raw(area.width, area.height, pixels));
        canvas.draw(0, 0, area.width, area.height, image, source_x, source_y, Clip { x: 0, y: 0, ..area });
        let raw = canvas.image().raw();
        for (row, bytes) in raw.chunks_exact(row_bytes).enumerate() {
            let address = base + (area.y as u32 + row as u32) * self.0.bpl + area.x as u32 * bytes_per_pixel;
            context.write_bytes(address, bytes)?;
        }
        Ok(())
    }

    /// Fill ordinary opaque copy pixels without reading or materializing the
    /// framebuffer. Other composition modes must use the caller's canvas path.
    /// The only temporary storage is a fixed-size chunk, independent of the
    /// backing size; all framebuffer state remains in guest memory.
    pub fn try_fill_opaque_rect(&self, context: &mut dyn WIPICContext, rectangle: Clip, color: Color, clip: Clip) -> Result<bool> {
        if color.a != 255 || !matches!(self.0.bpp, 16 | 32) {
            return Ok(false);
        }

        let area = rectangle.intersect(&clip).intersect(&Clip {
            x: 0,
            y: 0,
            width: self.0.width,
            height: self.0.height,
        });
        if area.width == 0 || area.height == 0 {
            return Ok(true);
        }

        let bytes_per_pixel = self.0.bpp / 8;
        let (_, packed_bpl) = buffer_size(self.0.width, self.0.height, bytes_per_pixel)?;
        let backing_size = self.0.bpl.checked_mul(self.0.height).ok_or(WieError::AllocationFailure)?;
        if self.0.bpl < packed_bpl || backing_size > MAX_FRAMEBUFFER_BYTES {
            return Err(WieError::AllocationFailure);
        }
        let base = context.data_ptr(self.0.buf)?;
        base.checked_add(backing_size).ok_or(WieError::InvalidMemoryAccess(base))?;

        let pixel = match self.0.bpp {
            16 => u32::from(Rgb565Pixel::from_color(color)).to_le_bytes(),
            _ => ArgbPixel::from_color(color).to_le_bytes(),
        };
        let row_bytes = area.width * bytes_per_pixel;
        let mut chunk = [0; 1024];
        let chunk_length = (row_bytes as usize).min(chunk.len());
        for destination in chunk[..chunk_length].chunks_exact_mut(bytes_per_pixel as usize) {
            destination.copy_from_slice(&pixel[..bytes_per_pixel as usize]);
        }

        for row in 0..area.height {
            let address = base + (area.y as u32 + row) * self.0.bpl + area.x as u32 * bytes_per_pixel;
            let mut offset = 0;
            while offset < row_bytes {
                let length = (row_bytes - offset).min(chunk_length as u32);
                context.write_bytes(address + offset, &chunk[..length as usize])?;
                offset += length;
            }
        }
        Ok(true)
    }

    pub fn pixel_to_color(&self, pixel: WIPICWord) -> Color {
        match self.0.bpp {
            16 => Rgb565Pixel::to_color(pixel as u16),
            _ => Rgb8Pixel::to_color(pixel),
        }
    }
}

pub struct FramebufferCanvas<'a> {
    framebuffer: &'a FrameBuffer,
    context: &'a mut dyn WIPICContext,
    canvas: Box<dyn Canvas>,
    flushed: bool,
}

impl FramebufferCanvas<'_> {
    pub fn flush(mut self) -> Result<()> {
        self.flushed = true;

        self.framebuffer.write(self.context, &self.canvas.image().raw())
    }
}

// best-effort fallback for canvases dropped without an explicit flush
impl Drop for FramebufferCanvas<'_> {
    fn drop(&mut self) {
        if self.flushed {
            return;
        }

        tracing::warn!("framebuffer canvas dropped without explicit flush; write-back errors will be lost");

        if let Err(err) = self.framebuffer.write(self.context, &self.canvas.image().raw()) {
            tracing::error!("Failed to flush framebuffer canvas: {err}");
        }
    }
}

impl Deref for FramebufferCanvas<'_> {
    type Target = Box<dyn Canvas>;

    fn deref(&self) -> &Self::Target {
        &self.canvas
    }
}

impl DerefMut for FramebufferCanvas<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.canvas
    }
}

#[cfg(test)]
mod test {
    use alloc::{vec, vec::Vec};

    use wipi_types::wipic::{WIPICFramebuffer, WIPICIndirectPtr};

    use wie_backend::canvas::{ArgbPixel, Clip, Color, PixelType, VecImageBuffer};
    use wie_util::{ByteRead, ByteWrite, Result, WieError};

    use crate::context::{WIPICContext, test::TestContext};

    use super::FrameBuffer;

    fn alpha_test_image() -> VecImageBuffer<ArgbPixel> {
        VecImageBuffer::from_raw(
            9,
            7,
            (0..63)
                .map(|i| {
                    ArgbPixel::from_color(Color {
                        a: [0, 1, 64, 127, 128, 254, 255][i % 7],
                        r: (i * 17) as u8,
                        g: (i * 41) as u8,
                        b: (i * 29) as u8,
                    })
                })
                .collect(),
        )
    }

    #[test]
    fn regional_blits_match_full_canvas_with_alpha_clipping_and_extreme_offsets() -> Result<()> {
        let image = alpha_test_image();
        let bounds = Clip {
            x: 0,
            y: 0,
            width: 32,
            height: 24,
        };
        let cases = [
            (2, 3, 9, 7, 0, 0, bounds),
            (-4, -3, 20, 20, 0, 0, bounds),
            (29, 21, 20, 20, -2, -1, bounds),
            (4, 3, 20, 20, 7, 5, bounds),
            (
                2,
                3,
                9,
                7,
                0,
                0,
                Clip {
                    x: 4,
                    y: 4,
                    width: 3,
                    height: 2,
                },
            ),
            (
                2,
                3,
                9,
                7,
                0,
                0,
                Clip {
                    x: -20,
                    y: -10,
                    width: 25,
                    height: 15,
                },
            ),
            (i32::MIN, 0, u32::MAX, 7, i32::MIN, 0, bounds),
            (i32::MAX, i32::MAX, u32::MAX, u32::MAX, 0, 0, bounds),
            (0, 0, u32::MAX, u32::MAX, i32::MIN, i32::MAX, bounds),
            (0, 0, 0, 7, 0, 0, bounds),
            (0, 0, 9, 7, 0, 0, Clip { width: 0, ..bounds }),
        ];
        for bpp in [16, 32] {
            let mut direct_context = TestContext::new();
            let mut reference_context = TestContext::new();
            let direct = FrameBuffer::new(&mut direct_context, 32, 24, bpp)?;
            let reference = FrameBuffer::new(&mut reference_context, 32, 24, bpp)?;
            let initial = (0..32 * 24 * (bpp / 8)).map(|i| (i * 37) as u8).collect::<Vec<_>>();
            direct.write(&mut direct_context, &initial)?;
            reference.write(&mut reference_context, &initial)?;
            for (index, (x, y, w, h, sx, sy, clip)) in cases.into_iter().enumerate() {
                assert!(direct.try_draw_image(&mut direct_context, x, y, w, h, &image, sx, sy, clip)?);
                let mut canvas = reference.canvas(&mut reference_context)?;
                canvas.draw(x, y, w, h, &image, sx, sy, clip);
                canvas.flush()?;
                assert_eq!(
                    direct.data(&direct_context)?,
                    reference.data(&reference_context)?,
                    "bpp={bpp}, case={index}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn regional_blit_preserves_padding_and_only_transfers_touched_pixels() -> Result<()> {
        let image = alpha_test_image();
        for bpp in [16, 32] {
            let mut context = TestContext::new();
            let mut reference_context = TestContext::new();
            let bytes_per_pixel = bpp / 8;
            let packed_bpl = 9 * bytes_per_pixel;
            let stride = packed_bpl + 6;
            let initial = (0..stride * 7 + 8).map(|i| (i * 37) as u8).collect::<Vec<_>>();
            let allocation = context.alloc(initial.len() as u32)?;
            context.write_bytes(allocation.0, &initial)?;
            let direct = FrameBuffer(wipi_types::wipic::WIPICFramebuffer {
                width: 9,
                height: 7,
                bpl: stride,
                bpp,
                buf: WIPICIndirectPtr(allocation.0 + 4),
            });
            let reference = FrameBuffer::new(&mut reference_context, 9, 7, bpp)?;
            let packed = initial[4..4 + (stride * 7) as usize]
                .chunks_exact(stride as usize)
                .flat_map(|row| row[..packed_bpl as usize].iter().copied())
                .collect::<Vec<_>>();
            reference.write(&mut reference_context, &packed)?;
            let clip = Clip {
                x: 2,
                y: 1,
                width: 4,
                height: 3,
            };
            context.reset_io_counts();
            assert!(direct.try_draw_image(&mut context, 0, 0, 9, 7, &image, 0, 0, clip)?);
            assert_eq!(
                context.io_counts(),
                ((4 * 3 * bytes_per_pixel) as usize, (4 * 3 * bytes_per_pixel) as usize)
            );
            let mut canvas = reference.canvas(&mut reference_context)?;
            canvas.draw(0, 0, 9, 7, &image, 0, 0, clip);
            canvas.flush()?;
            let packed = reference.data(&reference_context)?;
            let mut expected = initial.clone();
            for (row, pixels) in packed.chunks_exact(packed_bpl as usize).enumerate() {
                let offset = 4 + row * stride as usize;
                expected[offset..offset + packed_bpl as usize].copy_from_slice(pixels);
            }
            let mut actual = vec![0; initial.len()];
            context.read_bytes(allocation.0, &mut actual)?;
            assert_eq!(actual, expected, "bpp={bpp}");
        }
        Ok(())
    }

    #[test]
    fn test_new_overflow_returns_error() {
        let mut context = TestContext::new();

        assert!(matches!(
            FrameBuffer::new(&mut context, 0x10000, 0x10000, 32),
            Err(WieError::AllocationFailure)
        ));
    }

    #[test]
    fn test_new_over_heap_limit_returns_error() {
        let mut context = TestContext::new();

        assert!(matches!(
            FrameBuffer::new(&mut context, 0x4000, 0x4000, 32),
            Err(WieError::AllocationFailure)
        ));
    }

    #[test]
    fn test_new_zero_height_bpl_overflow_returns_error() {
        let mut context = TestContext::new();

        assert!(matches!(
            FrameBuffer::new(&mut context, 0xffff_ffff, 0, 32),
            Err(WieError::AllocationFailure)
        ));
    }

    #[test]
    fn test_new_normal_size_ok() {
        let mut context = TestContext::new();

        let framebuffer = FrameBuffer::new(&mut context, 100, 100, 16).unwrap();
        assert_eq!(framebuffer.0.width, 100);
        assert_eq!(framebuffer.0.height, 100);
        assert_eq!(framebuffer.0.bpl, 200);
        assert_eq!(framebuffer.0.bpp, 16);
        assert_eq!(framebuffer.data(&context).unwrap().len(), 20000);
    }

    #[test]
    fn opaque_fill_matches_canvas_at_clipped_and_extreme_coordinates() -> Result<()> {
        let bounds = Clip {
            x: 0,
            y: 0,
            width: 16,
            height: 10,
        };
        let cases = [
            (
                Clip {
                    x: 2,
                    y: 3,
                    width: 7,
                    height: 4,
                },
                bounds,
            ),
            (
                Clip {
                    x: -4,
                    y: -2,
                    width: 9,
                    height: 7,
                },
                bounds,
            ),
            (
                bounds,
                Clip {
                    x: 5,
                    y: 4,
                    width: 3,
                    height: 2,
                },
            ),
            (
                bounds,
                Clip {
                    x: -8,
                    y: -7,
                    width: 11,
                    height: 10,
                },
            ),
            (
                Clip {
                    x: i32::MAX,
                    y: 0,
                    width: u32::MAX,
                    height: 10,
                },
                bounds,
            ),
            (
                Clip {
                    x: i32::MIN,
                    y: i32::MIN,
                    width: u32::MAX,
                    height: u32::MAX,
                },
                bounds,
            ),
            (
                Clip {
                    x: 0,
                    y: 0,
                    width: 0,
                    height: 10,
                },
                bounds,
            ),
            (
                bounds,
                Clip {
                    x: 5,
                    y: 4,
                    width: 0,
                    height: 2,
                },
            ),
        ];
        for bpp in [16, 32] {
            let mut direct_context = TestContext::new();
            let mut canvas_context = TestContext::new();
            let direct = FrameBuffer::new(&mut direct_context, bounds.width, bounds.height, bpp)?;
            let reference = FrameBuffer::new(&mut canvas_context, bounds.width, bounds.height, bpp)?;
            let initial = (0..bounds.width * bounds.height * (bpp / 8)).map(|i| i as u8).collect::<Vec<_>>();
            direct.write(&mut direct_context, &initial)?;
            reference.write(&mut canvas_context, &initial)?;

            for (index, (rectangle, clip)) in cases.into_iter().enumerate() {
                let color = Color {
                    a: 255,
                    r: 211,
                    g: index as u8 * 31,
                    b: 73,
                };
                assert!(direct.try_fill_opaque_rect(&mut direct_context, rectangle, color, clip)?);
                let mut canvas = reference.canvas(&mut canvas_context)?;
                canvas.fill_rect(rectangle.x, rectangle.y, rectangle.width, rectangle.height, color, clip);
                canvas.flush()?;
                assert_eq!(direct.data(&direct_context)?, reference.data(&canvas_context)?, "bpp={bpp}, case={index}");
            }
        }
        Ok(())
    }

    #[test]
    fn opaque_fill_preserves_row_padding_and_surrounding_guest_memory() -> Result<()> {
        let mut context = TestContext::new();
        let allocation = context.alloc(64)?;
        context.write_bytes(allocation.0, &[0xa5; 64])?;
        let framebuffer = FrameBuffer(WIPICFramebuffer {
            width: 5,
            height: 4,
            bpl: 14,
            bpp: 16,
            buf: WIPICIndirectPtr(allocation.0 + 4),
        });
        assert!(framebuffer.try_fill_opaque_rect(
            &mut context,
            Clip {
                x: -2,
                y: 1,
                width: 7,
                height: 9
            },
            Color { a: 255, r: 255, g: 0, b: 0 },
            Clip {
                x: 2,
                y: 1,
                width: 99,
                height: 2
            },
        )?);

        let mut actual = [0; 64];
        context.read_bytes(allocation.0, &mut actual)?;
        let mut expected = [0xa5; 64];
        for y in 1..=2 {
            for x in 2..=4 {
                let offset = 4 + y * 14 + x * 2;
                expected[offset..offset + 2].copy_from_slice(&0xf800_u16.to_le_bytes());
            }
        }
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn opaque_fill_handles_rows_larger_than_the_fixed_chunk() -> Result<()> {
        let mut context = TestContext::new();
        let framebuffer = FrameBuffer::new(&mut context, 700, 2, 16)?;
        framebuffer.write(&mut context, &vec![0xa5; 2800])?;
        assert!(framebuffer.try_fill_opaque_rect(
            &mut context,
            Clip {
                x: 1,
                y: 0,
                width: 697,
                height: 2
            },
            Color { a: 255, r: 0, g: 255, b: 0 },
            Clip {
                x: 0,
                y: 0,
                width: 700,
                height: 2
            },
        )?);
        for row in framebuffer.data(&context)?.as_chunks::<1400>().0 {
            assert_eq!(&row[..2], &[0xa5; 2]);
            assert_eq!(&row[1396..], &[0xa5; 4]);
            assert!(row[2..1396].as_chunks::<2>().0.iter().all(|pixel| *pixel == 0x07e0_u16.to_le_bytes()));
        }
        Ok(())
    }

    #[test]
    fn opaque_fill_declines_transparent_colors_and_unsupported_formats_without_writing() -> Result<()> {
        let bounds = Clip {
            x: 0,
            y: 0,
            width: 2,
            height: 2,
        };
        for (bpp, alpha) in [(16, 0), (16, 128), (32, 0), (32, 128), (8, 255)] {
            let mut context = TestContext::new();
            let framebuffer = FrameBuffer::new(&mut context, 2, 2, bpp)?;
            let initial = vec![0xa5; (4 * bpp / 8) as usize];
            framebuffer.write(&mut context, &initial)?;
            assert!(!framebuffer.try_fill_opaque_rect(
                &mut context,
                bounds,
                Color {
                    a: alpha,
                    r: 255,
                    g: 0,
                    b: 0
                },
                bounds,
            )?);
            assert_eq!(framebuffer.data(&context)?, initial);
        }
        Ok(())
    }

    #[test]
    fn opaque_fill_rejects_invalid_guest_stride_and_address_overflow() -> Result<()> {
        let mut context = TestContext::new();
        let bounds = Clip {
            x: 0,
            y: 0,
            width: 2,
            height: 2,
        };
        let color = Color { a: 255, r: 255, g: 0, b: 0 };
        let mut framebuffer = FrameBuffer::new(&mut context, 2, 2, 16)?;
        framebuffer.0.bpl = 3;
        assert!(matches!(
            framebuffer.try_fill_opaque_rect(&mut context, bounds, color, bounds),
            Err(WieError::AllocationFailure)
        ));
        framebuffer.0.bpl = u32::MAX;
        assert!(matches!(
            framebuffer.try_fill_opaque_rect(&mut context, bounds, color, bounds),
            Err(WieError::AllocationFailure)
        ));
        framebuffer.0.bpl = 4;
        framebuffer.0.buf = WIPICIndirectPtr(u32::MAX - 3);
        assert!(matches!(
            framebuffer.try_fill_opaque_rect(&mut context, bounds, color, bounds),
            Err(WieError::InvalidMemoryAccess(_))
        ));
        Ok(())
    }
}
