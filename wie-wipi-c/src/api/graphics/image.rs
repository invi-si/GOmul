use alloc::{boxed::Box, vec};

use wie_backend::canvas::{ArgbPixel, Image, PixelType, Rgb565Pixel, VecImageBuffer, decode_image};
use wie_util::{Result, WieError, read_generic, write_generic};

use wipi_types::wipic::{WIPICFramebuffer, WIPICImage, WIPICIndirectPtr, WIPICWord};

use crate::{api::graphics::framebuffer::FrameBuffer, context::WIPICContext};

pub fn create_wipi_image(context: &mut dyn WIPICContext, buf: WIPICIndirectPtr, offset: WIPICWord, len: WIPICWord) -> Result<WIPICImage> {
    let decoded = decode_source(context, buf, offset, len)?;
    let (img_framebuffer, mask_framebuffer) = if context.indirect_image_framebuffers() {
        // Native KTF blitters use the image's pixel stride for LCD writes too.
        // Match our RGB565 LCD and retain transparency as the ABI's byte mask.
        let colors = decoded.colors();
        let pixels = colors.iter().map(|&color| Rgb565Pixel::from_color(color)).collect();
        let native = VecImageBuffer::<Rgb565Pixel>::from_raw(decoded.width(), decoded.height(), pixels);
        let framebuffer = FrameBuffer::from_image(context, &native)?;
        let mask = if colors.iter().any(|color| color.a != 255) {
            let result = FrameBuffer::new(context, decoded.width(), decoded.height(), 8);
            let mask = match result {
                Ok(mask) => mask,
                Err(error) => {
                    context.free(framebuffer.0.buf)?;
                    return Err(error);
                }
            };
            let alpha: alloc::vec::Vec<u8> = colors.iter().map(|color| color.a).collect();
            context.write_bytes(context.data_ptr(mask.0.buf)?, &alpha)?;
            mask
        } else {
            FrameBuffer::empty()
        };
        (framebuffer, mask)
    } else {
        (FrameBuffer::from_image(context, &*decoded)?, FrameBuffer::empty())
    };

    Ok(WIPICImage {
        img: img_framebuffer.0,
        mask: mask_framebuffer.0,
        loop_count: 0,
        delay: 0,
        animated: 0,
        buf,
        offset,
        current: 0,
        len,
    })
}

fn decode_source(context: &dyn WIPICContext, buf: WIPICIndirectPtr, offset: u32, len: u32) -> Result<Box<dyn Image>> {
    let address = context.data_ptr(buf)?.checked_add(offset).ok_or(WieError::AllocationFailure)?;
    let mut data = vec![0; len as usize];
    context.read_bytes(address, &mut data)?;
    decode_image(&data)
}
pub fn decode_image_framebuffer(context: &mut dyn WIPICContext, buf: WIPICIndirectPtr, offset: WIPICWord, len: WIPICWord) -> Result<FrameBuffer> {
    let decoded = decode_source(context, buf, offset, len)?;
    FrameBuffer::from_image(context, &*decoded)
}

pub fn rendering_image(context: &mut dyn WIPICContext, image: WIPICImage) -> Result<Box<dyn Image>> {
    let pixels = FrameBuffer(image.img).image(context)?;
    if image.mask.buf.0 == 0 {
        return Ok(pixels);
    }
    if image.mask.bpp != 8 || image.mask.width != image.img.width || image.mask.height != image.img.height {
        return Err(WieError::FatalError("Invalid image alpha-mask geometry".into()));
    }
    let alpha = FrameBuffer(image.mask).data(context)?;
    let colors = pixels
        .colors()
        .into_iter()
        .zip(alpha)
        .map(|(mut color, a)| {
            color.a = a;
            ArgbPixel::from_color(color)
        })
        .collect();
    Ok(Box::new(VecImageBuffer::<ArgbPixel>::from_raw(image.img.width, image.img.height, colors)))
}

// KTF callers dereference image data[0] and data[1] as framebuffer memory IDs.
// The remainder is private bookkeeping, also stored in guest memory so states
// remain self-contained. Live descriptors, not the bookkeeping copy, are authoritative.
const INDIRECT_PREFIX_SIZE: u32 = 8;
pub fn allocate_image(context: &mut dyn WIPICContext, image: WIPICImage) -> Result<WIPICIndirectPtr> {
    let mut owned = alloc::vec![image.img.buf];
    if image.mask.buf.0 != 0 {
        owned.push(image.mask.buf);
    }
    let result = (|| {
        if !context.indirect_image_framebuffers() {
            let handle = context.alloc(size_of::<WIPICImage>() as u32)?;
            owned.push(handle);
            write_generic(context, context.data_ptr(handle)?, image)?;
            return Ok(handle);
        }
        let framebuffer = context.alloc(size_of::<WIPICFramebuffer>() as u32)?;
        owned.push(framebuffer);
        write_generic(context, context.data_ptr(framebuffer)?, image.img)?;
        let mask = if image.mask.buf.0 == 0 {
            WIPICIndirectPtr(0)
        } else {
            let handle = context.alloc(size_of::<WIPICFramebuffer>() as u32)?;
            owned.push(handle);
            write_generic(context, context.data_ptr(handle)?, image.mask)?;
            handle
        };
        let handle = context.alloc(INDIRECT_PREFIX_SIZE + size_of::<WIPICImage>() as u32)?;
        owned.push(handle);
        let address = context.data_ptr(handle)?;
        write_generic(context, address, [framebuffer.0, mask.0])?;
        write_generic(context, address + INDIRECT_PREFIX_SIZE, image)?;
        Ok(handle)
    })();
    if result.is_err() {
        for handle in owned.into_iter().rev() {
            context.free(handle)?;
        }
    }
    result
}
pub fn read_image(context: &dyn WIPICContext, handle: WIPICIndirectPtr) -> Result<WIPICImage> {
    let address = context.data_ptr(handle)?;
    if !context.indirect_image_framebuffers() {
        return read_generic(context, address);
    }
    let handles: [u32; 2] = read_generic(context, address)?;
    let mut image: WIPICImage = read_generic(context, address + INDIRECT_PREFIX_SIZE)?;
    image.img = read_generic(context, context.data_ptr(WIPICIndirectPtr(handles[0]))?)?;
    image.mask = if handles[1] == 0 {
        FrameBuffer::empty().0
    } else {
        read_generic(context, context.data_ptr(WIPICIndirectPtr(handles[1]))?)?
    };
    Ok(image)
}
pub fn release_image(context: &mut dyn WIPICContext, handle: WIPICIndirectPtr) -> Result<()> {
    if handle.0 == 0 {
        return Ok(());
    }
    let image = read_image(context, handle)?;
    context.free(image.img.buf)?;
    if image.mask.buf.0 != 0 {
        context.free(image.mask.buf)?;
    }
    if context.indirect_image_framebuffers() {
        let handles: [u32; 2] = read_generic(context, context.data_ptr(handle)?)?;
        context.free(WIPICIndirectPtr(handles[0]))?;
        if handles[1] != 0 {
            context.free(WIPICIndirectPtr(handles[1]))?;
        }
    }
    // Encoded source memory is borrowed by CreateImage, not owned by the image.
    context.free(handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::graphics::{get_image_framebuffer, get_image_property},
        context::test::TestContext,
    };
    use wie_util::ByteWrite;
    #[futures_test::test]
    async fn native_images_match_lcd_depth_and_keep_alpha_out_of_pixel_storage() -> Result<()> {
        let source = VecImageBuffer::<ArgbPixel>::from_raw(2, 1, vec![0x80ff0000, 0x0000ff00]);
        let png = wie_backend::canvas::encode_png(&source)?;
        for indirect in [false, true] {
            let mut context = TestContext::new();
            context.indirect_images = indirect;
            let encoded = context.alloc(png.len() as u32)?;
            let address = context.data_ptr(encoded)?;
            context.write_bytes(address, &png)?;
            let decoded = create_wipi_image(&mut context, encoded, 0, png.len() as u32)?;
            assert_eq!(decoded.img.bpp, if indirect { super::super::FRAMEBUFFER_DEPTH } else { 32 });
            assert_eq!(decoded.img.bpl, if indirect { 4 } else { 8 });
            let rendered = rendering_image(&mut context, decoded)?;
            assert_eq!(rendered.get_pixel(0, 0).a, 128);
            assert_eq!(rendered.get_pixel(1, 0).a, 0);
            assert_eq!(rendered.get_pixel(0, 0).r, 255);
            if indirect {
                assert_eq!(FrameBuffer(decoded.img).data(&context)?.len(), 4);
                assert_eq!(decoded.mask.bpp, 8);
                assert_eq!(FrameBuffer(decoded.mask).data(&context)?, vec![128, 0]);
            } else {
                assert_eq!(decoded.mask.buf.0, 0);
            }
            let handle = allocate_image(&mut context, decoded)?;
            release_image(&mut context, handle)?;
            assert!(!context.freed.contains(&encoded.0));
            if indirect {
                assert!(context.freed.contains(&decoded.mask.buf.0));
            }
        }
        Ok(())
    }

    #[futures_test::test]
    async fn carrier_image_abi_preserves_live_framebuffer_aliases_and_ownership() -> Result<()> {
        for indirect in [false, true] {
            let mut context = TestContext::new();
            context.indirect_images = indirect;
            let source = context.alloc(16)?;
            let framebuffer = FrameBuffer::new(&mut context, 4, 3, 16)?.0;
            let image = WIPICImage {
                img: framebuffer,
                mask: FrameBuffer::empty().0,
                loop_count: 0,
                delay: 0,
                animated: 0,
                buf: source,
                offset: 0,
                current: 0,
                len: 16,
            };
            let handle = allocate_image(&mut context, image)?;
            let fb = get_image_framebuffer(&mut context, handle).await?;
            if indirect {
                let prefix: [u32; 2] = read_generic(&context, context.data_ptr(handle)?)?;
                assert_eq!(prefix, [fb.0, 0]);
                assert_ne!(fb.0, handle.0);
            } else {
                assert_eq!(fb.0, handle.0);
            }
            assert_eq!(get_image_property(&mut context, handle, 4).await?, 4);
            let mut live: WIPICFramebuffer = read_generic(&context, context.data_ptr(fb)?)?;
            assert_eq!(live.buf.0, framebuffer.buf.0);
            live.width = 2;
            let address = context.data_ptr(fb)?;
            write_generic(&mut context, address, live)?;
            assert_eq!(get_image_property(&mut context, handle, 4).await?, 2);
            assert_eq!(read_image(&context, handle)?.buf.0, source.0);
            release_image(&mut context, handle)?;
            assert!(context.freed.contains(&framebuffer.buf.0));
            assert!(context.freed.contains(&handle.0));
            assert!(!context.freed.contains(&source.0));
            if indirect {
                assert!(context.freed.contains(&fb.0));
            }
            assert_eq!(context.freed.len(), if indirect { 3 } else { 2 });
        }
        Ok(())
    }
}
