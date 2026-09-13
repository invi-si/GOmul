mod encode;
mod framebuffer;
mod grp_context;
mod image;
pub mod primitives;

pub use encode::encode_image;
pub use framebuffer::FrameBuffer;
pub use image::decode_image_framebuffer;

use core::mem::size_of;

use wie_backend::{
    Event,
    canvas::{Clip, Color, PixelType, Rgb565Pixel, string_width},
};
use wie_util::{Result, read_generic, write_generic};

use wipi_types::wipic::{WIPICDisplayInfo, WIPICFramebuffer, WIPICGraphicsContext, WIPICIndirectPtr, WIPICWord};

use crate::context::WIPICContext;

use self::{
    grp_context::WIPICGraphicsContextIdx,
    image::{allocate_image, create_wipi_image, read_image, release_image, rendering_image},
};

const FRAMEBUFFER_DEPTH: u32 = 16; // XXX hardcode to 16bpp as some game requires 16bpp framebuffer
const SCREEN_FRAMEBUFFER_PTR: u32 = 0x7fff1000;
const DEFAULT_FONT_ASCENT: i32 = 10;

// Graphics-context offsets are signed 16-bit destination translations.
// Image source coordinates remain relative to the source image.
fn translated_point(graphics: &WIPICGraphicsContext, x: i32, y: i32) -> (i32, i32) {
    (
        x.wrapping_add(i32::from(graphics.offset[0] as i16)),
        y.wrapping_add(i32::from(graphics.offset[1] as i16)),
    )
}

pub fn screen_framebuffer(context: &dyn WIPICContext) -> Result<Option<WIPICIndirectPtr>> {
    let address: u32 = read_generic(context, SCREEN_FRAMEBUFFER_PTR)?;
    Ok((address != 0).then_some(WIPICIndirectPtr(address)))
}

pub async fn get_screen_framebuffer(context: &mut dyn WIPICContext, a0: WIPICWord) -> Result<WIPICIndirectPtr> {
    tracing::debug!("MC_grpGetScreenFrameBuffer({a0:#x})");

    if let Some(framebuffer) = screen_framebuffer(context)? {
        return Ok(framebuffer);
    }

    let (width, height) = {
        let platform = context.system().platform();
        let screen = platform.screen();
        (screen.width(), screen.height())
    };

    let framebuffer = FrameBuffer::new(context, width, height, FRAMEBUFFER_DEPTH)?;

    let memory = context.alloc(size_of::<WIPICFramebuffer>() as WIPICWord)?;
    write_generic(context, context.data_ptr(memory)?, framebuffer.0)?;
    write_generic(context, SCREEN_FRAMEBUFFER_PTR, memory.0)?;

    Ok(memory)
}

pub async fn init_context(context: &mut dyn WIPICContext, p_grp_ctx: WIPICWord) -> Result<()> {
    tracing::debug!("MC_grpInitContext({p_grp_ctx:#x})");

    let grp_ctx = WIPICGraphicsContext {
        clip: [0, 0, i16::MAX as u16, i16::MAX as u16],
        ..WIPICGraphicsContext::default()
    };
    write_generic(context, p_grp_ctx, grp_ctx)?;
    Ok(())
}

pub async fn set_context(context: &mut dyn WIPICContext, p_grp_ctx: WIPICWord, op: WIPICGraphicsContextIdx, pv: WIPICWord) -> Result<()> {
    tracing::debug!("MC_grpSetContext({p_grp_ctx:#x}, {op:?}, {pv:#x})");

    let mut grp_ctx: WIPICGraphicsContext = read_generic(context, p_grp_ctx)?;
    match op {
        WIPICGraphicsContextIdx::ClipIdx => {
            // KTF callers pass NULL when their requested rectangle covers the
            // entire framebuffer. It removes clipping rather than pointing at
            // a rectangle at guest address zero. LGT has a separate adapter.
            let clip: [i32; 4] = if pv == 0 {
                [0, 0, i16::MAX as i32, i16::MAX as i32]
            } else {
                read_generic(context, pv)?
            };
            grp_ctx.clip = clip.map(|value| value as u16);
        }
        WIPICGraphicsContextIdx::FgPixelIdx => {
            grp_ctx.fgpxl = pv as _;
        }
        WIPICGraphicsContextIdx::BgPixelIdx => {
            grp_ctx.bgpxl = pv as _;
        }
        WIPICGraphicsContextIdx::TransPixelIdx => {
            grp_ctx.transpxl = pv as _;
        }
        WIPICGraphicsContextIdx::AlphaIdx => {
            grp_ctx.alpha = pv as _;
            // grp_ctx.pixel_op_func_ptr = todo!();
            // grp_ctx.param1 = todo!();
        }
        WIPICGraphicsContextIdx::PixelopIdx => {
            grp_ctx.pixel_op_func_ptr = pv;
        }
        WIPICGraphicsContextIdx::PixelParam1Idx => {
            grp_ctx.param1 = pv;
        }
        WIPICGraphicsContextIdx::FontIdx => {
            grp_ctx.font = pv;
        }
        WIPICGraphicsContextIdx::StyleIdx => {
            grp_ctx.style = pv;
        }
        WIPICGraphicsContextIdx::OffsetIdx => {
            let offset: [i32; 2] = read_generic(context, pv)?;
            grp_ctx.offset = offset.map(|value| value as u16);
        }
        _ => {
            tracing::warn!("MC_grpSetContext({p_grp_ctx:#x}, {op:?}, {pv:#x}): ignoring invalid op");
        }
    }
    write_generic(context, p_grp_ctx, grp_ctx)?;

    Ok(())
}

pub async fn get_context(context: &mut dyn WIPICContext, p_grp_ctx: WIPICWord, op: WIPICGraphicsContextIdx, pv: WIPICWord) -> Result<()> {
    tracing::debug!("MC_grpGetContext({p_grp_ctx:#x}, {op:?}, {pv:#x})");

    let grp_ctx: WIPICGraphicsContext = read_generic(context, p_grp_ctx)?;
    match op {
        WIPICGraphicsContextIdx::ClipIdx => write_generic(context, pv, grp_ctx.clip.map(|value| i32::from(value as i16)))?,
        WIPICGraphicsContextIdx::FgPixelIdx => write_generic(context, pv, grp_ctx.fgpxl)?,
        WIPICGraphicsContextIdx::BgPixelIdx => write_generic(context, pv, grp_ctx.bgpxl)?,
        WIPICGraphicsContextIdx::TransPixelIdx => write_generic(context, pv, grp_ctx.transpxl)?,
        WIPICGraphicsContextIdx::AlphaIdx => write_generic(context, pv, grp_ctx.alpha)?,
        WIPICGraphicsContextIdx::PixelopIdx => write_generic(context, pv, grp_ctx.pixel_op_func_ptr)?,
        WIPICGraphicsContextIdx::PixelParam1Idx => write_generic(context, pv, grp_ctx.param1)?,
        WIPICGraphicsContextIdx::FontIdx => write_generic(context, pv, grp_ctx.font)?,
        WIPICGraphicsContextIdx::StyleIdx => write_generic(context, pv, grp_ctx.style)?,
        WIPICGraphicsContextIdx::OffsetIdx => write_generic(context, pv, grp_ctx.offset.map(|value| i32::from(value as i16)))?,
        _ => tracing::warn!("MC_grpGetContext({p_grp_ctx:#x}, {op:?}, {pv:#x}): ignoring invalid op"),
    }

    Ok(())
}

pub async fn put_pixel(context: &mut dyn WIPICContext, dst_fb: WIPICIndirectPtr, x: i32, y: i32, p_gctx: WIPICWord) -> Result<()> {
    tracing::debug!("MC_grpPutPixel({:#x}, {x}, {y}, {p_gctx:?})", dst_fb.0);

    let framebuffer = FrameBuffer(read_generic(context, context.data_ptr(dst_fb)?)?);
    let gctx: WIPICGraphicsContext = read_generic(context, p_gctx)?;
    let (x, y) = translated_point(&gctx, x, y);

    let color = framebuffer.pixel_to_color(gctx.fgpxl);
    primitives::put_pixel(
        context,
        &framebuffer,
        x as _,
        y as _,
        color,
        Clip {
            x: 0,
            y: 0,
            width: framebuffer.0.width,
            height: framebuffer.0.height,
        },
    )
}

pub async fn fill_rect(context: &mut dyn WIPICContext, dst_fb: WIPICIndirectPtr, x: i32, y: i32, w: i32, h: i32, p_gctx: WIPICWord) -> Result<()> {
    tracing::debug!("MC_grpFillRect({:#x}, {x}, {y}, {w}, {h}, {p_gctx:#x})", dst_fb.0);

    if w <= 0 || h <= 0 {
        return Ok(());
    }

    let framebuffer = FrameBuffer(read_generic(context, context.data_ptr(dst_fb)?)?);
    let gctx: WIPICGraphicsContext = read_generic(context, p_gctx)?;
    let (x, y) = translated_point(&gctx, x, y);
    let clip = Clip {
        x: x as _,
        y: y as _,
        width: w as _,
        height: h as _,
    };

    let color = framebuffer.pixel_to_color(gctx.fgpxl);
    primitives::fill_rect(context, &framebuffer, x, y, w as u32, h as u32, color, clip)
}

#[allow(clippy::too_many_arguments)]
pub async fn draw_arc(
    context: &mut dyn WIPICContext,
    dst: WIPICIndirectPtr,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    start_angle: i32,
    arc_angle: i32,
    p_gctx: WIPICWord,
) -> Result<()> {
    tracing::debug!("MC_grpDrawArc({:#x}, {x}, {y}, {w}, {h}, {start_angle}, {arc_angle}, {p_gctx:#x})", dst.0);

    if w <= 0 || h <= 0 {
        return Ok(());
    }

    let framebuffer = FrameBuffer(read_generic(context, context.data_ptr(dst)?)?);
    let gctx: WIPICGraphicsContext = read_generic(context, p_gctx)?;
    let (x, y) = translated_point(&gctx, x, y);
    let clip = Clip {
        x: x as _,
        y: y as _,
        width: w as _,
        height: h as _,
    };

    let color = framebuffer.pixel_to_color(gctx.fgpxl);
    primitives::draw_arc(context, &framebuffer, x, y, w as u32, h as u32, start_angle, arc_angle, color, clip)
}

#[allow(clippy::too_many_arguments)]
pub async fn fill_arc(
    context: &mut dyn WIPICContext,
    dst: WIPICIndirectPtr,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    start_angle: i32,
    arc_angle: i32,
    p_gctx: WIPICWord,
) -> Result<()> {
    tracing::debug!("MC_grpFillArc({:#x}, {x}, {y}, {w}, {h}, {start_angle}, {arc_angle}, {p_gctx:#x})", dst.0);

    if w <= 0 || h <= 0 {
        return Ok(());
    }

    let framebuffer = FrameBuffer(read_generic(context, context.data_ptr(dst)?)?);
    let gctx: WIPICGraphicsContext = read_generic(context, p_gctx)?;
    let (x, y) = translated_point(&gctx, x, y);
    let clip = Clip {
        x: x as _,
        y: y as _,
        width: w as _,
        height: h as _,
    };

    let color = framebuffer.pixel_to_color(gctx.fgpxl);
    primitives::fill_arc(context, &framebuffer, x, y, w as u32, h as u32, start_angle, arc_angle, color, clip)
}

pub async fn create_image(
    context: &mut dyn WIPICContext,
    ptr_image: WIPICWord,
    image_data: WIPICIndirectPtr,
    offset: u32,
    len: u32,
) -> Result<WIPICWord> {
    tracing::debug!("MC_grpCreateImage({ptr_image:#x}, {:#x}, {offset}, {len})", image_data.0);

    let image = create_wipi_image(context, image_data, offset, len)?;

    let memory = allocate_image(context, image)?;
    write_generic(context, ptr_image, memory)?;

    Ok(1) // MC_GRP_IMAGE_DONE
}

pub async fn destroy_image(context: &mut dyn WIPICContext, image: WIPICIndirectPtr) -> Result<()> {
    tracing::debug!("MC_grpDestroyImage({:#x})", image.0);

    release_image(context, image)?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn draw_image(
    context: &mut dyn WIPICContext,
    framebuffer: WIPICIndirectPtr,
    dx: i32,
    dy: i32,
    w: i32,
    h: i32,
    image: WIPICIndirectPtr,
    sx: i32,
    sy: i32,
    graphics_context: WIPICWord,
) -> Result<()> {
    tracing::debug!(
        "MC_grpDrawImage({:#x}, {dx}, {dy}, {w}, {h}, {:#x}, {sx}, {sy}, {graphics_context:#x})",
        framebuffer.0,
        image.0
    );

    if w <= 0 || h <= 0 {
        return Ok(());
    }

    let framebuffer = FrameBuffer(read_generic(context, context.data_ptr(framebuffer)?)?);
    let graphics_context: WIPICGraphicsContext = read_generic(context, graphics_context)?;
    let (dx, dy) = translated_point(&graphics_context, dx, dy);
    // Decode the context's 16-bit clip corners as signed; right/bottom are excluded.
    let [left, top, right, bottom] = graphics_context.clip.map(|value| i32::from(value as i16));
    let clip = Clip {
        x: left,
        y: top,
        width: (right - left).max(0) as u32,
        height: (bottom - top).max(0) as u32,
    }
    .intersect(&Clip {
        x: 0,
        y: 0,
        width: framebuffer.0.width,
        height: framebuffer.0.height,
    });
    if clip.width == 0 || clip.height == 0 {
        return Ok(());
    }

    let image = read_image(context, image)?;
    let src_image = rendering_image(context, image)?;
    primitives::draw_image(context, &framebuffer, dx, dy, w as u32, h as u32, &*src_image, sx, sy, clip)
}

pub async fn flush_lcd(
    context: &mut dyn WIPICContext,
    i: WIPICWord,
    framebuffer: WIPICIndirectPtr,
    x: WIPICWord,
    y: WIPICWord,
    w: WIPICWord,
    h: WIPICWord,
) -> Result<()> {
    tracing::debug!("MC_grpFlushLcd({i:#x}, {:#x}, {x:#x}, {y:#x}, {w:#x}, {h:#x})", framebuffer.0);

    let framebuffer = FrameBuffer(read_generic(context, context.data_ptr(framebuffer)?)?);

    let src_canvas = framebuffer.image(context)?;

    let platform = context.system().platform();
    let screen = platform.screen();

    screen.paint(&*src_canvas);

    Ok(())
}

pub async fn get_pixel_from_rgb(_context: &mut dyn WIPICContext, r: i32, g: i32, b: i32) -> Result<WIPICWord> {
    tracing::debug!("MC_grpGetPixelFromRGB({r:#x}, {g:#x}, {b:#x})");
    if (r > 0xff) || (g > 0xff) | (b > 0xff) {
        tracing::debug!("MC_grpGetPixelFromRGB({r:#x}, {g:#x}, {b:#x}): value clipped to 8 bits");
    }

    let color = Rgb565Pixel::from_color(Color {
        a: 0xff,
        r: r as u8,
        g: g as u8,
        b: b as u8,
    });

    Ok(color as WIPICWord)
}

pub async fn get_rgb_from_pixel(context: &mut dyn WIPICContext, pixel: i32, r: WIPICWord, g: WIPICWord, b: WIPICWord) -> Result<i32> {
    tracing::debug!("MC_grpGetRGBFromPixel({pixel}, {r:#x}, {g:#x}, {b:#x})");

    let color = Rgb565Pixel::to_color(pixel as u16);

    write_generic(context, r, color.r as i32)?;
    write_generic(context, g, color.g as i32)?;
    write_generic(context, b, color.b as i32)?;

    Ok(pixel)
}

pub async fn get_display_info(context: &mut dyn WIPICContext, reserved: WIPICWord, out_ptr: WIPICWord) -> Result<WIPICWord> {
    tracing::debug!("MC_grpGetDisplayInfo({reserved:#x}, {out_ptr:#x})");

    assert_eq!(reserved, 0);

    let platform = context.system().platform();
    let screen = platform.screen();

    let info = WIPICDisplayInfo {
        bpp: FRAMEBUFFER_DEPTH,
        depth: 16,
        width: screen.width(),
        height: screen.height(),
        bpl: 2 * screen.width(),
        color_type: 1, // 1==MC_GRP_DIRECT_COLOR_TYPE
        red_mask: 0xf800,
        green_mask: 0x7e0,
        blue_mask: 0x1f,
    };

    write_generic(context, out_ptr, info)?;
    Ok(1)
}

#[allow(clippy::too_many_arguments)]
pub async fn copy_area(
    context: &mut dyn WIPICContext,
    dst: WIPICIndirectPtr,
    dx: i32,
    dy: i32,
    w: i32,
    h: i32,
    x: i32,
    y: i32,
    pgc: WIPICWord,
) -> Result<()> {
    tracing::debug!("MC_grpCopyArea({:#x}, {dx}, {dy}, {w}, {h}, {x}, {y}, {pgc:#x})", dst.0);

    if w < 0 || h < 0 {
        tracing::warn!("Skipping negative dimension");

        return Ok(());
    }

    let framebuffer = FrameBuffer(read_generic(context, context.data_ptr(dst)?)?);

    let clip = Clip {
        x: dx as _,
        y: dy as _,
        width: w as _,
        height: h as _,
    };

    primitives::copy_area(context, &framebuffer, dx, dy, w as u32, h as u32, x, y, clip)
}

pub async fn create_offscreen_framebuffer(context: &mut dyn WIPICContext, w: i32, h: i32) -> Result<WIPICIndirectPtr> {
    tracing::debug!("MC_grpCreateOffScreenFrameBuffer({w}, {h})");

    let framebuffer = FrameBuffer::new(context, w as _, h as _, FRAMEBUFFER_DEPTH)?;

    let memory = context.alloc(size_of::<WIPICFramebuffer>() as WIPICWord)?;
    write_generic(context, context.data_ptr(memory)?, framebuffer.0)?;

    Ok(memory)
}

pub async fn destroy_offscreen_framebuffer(context: &mut dyn WIPICContext, framebuffer: WIPICIndirectPtr) -> Result<()> {
    tracing::debug!("MC_grpDestroyOffScreenFrameBuffer({:#x})", framebuffer.0);

    // Cleanup may run before an offscreen allocation has succeeded. The LCD
    // framebuffer is owned by the display and must also survive this call.
    if framebuffer.0 == 0 || framebuffer.0 == read_generic::<u32, _>(context, SCREEN_FRAMEBUFFER_PTR)? {
        return Ok(());
    }
    let data: WIPICFramebuffer = read_generic(context, context.data_ptr(framebuffer)?)?;
    context.free(data.buf)?;
    context.free(framebuffer)?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn copy_frame_buffer(
    context: &mut dyn WIPICContext,
    dst: WIPICIndirectPtr,
    dx: i32,
    dy: i32,
    w: i32,
    h: i32,
    src: WIPICIndirectPtr,
    sx: i32,
    sy: i32,
    pgc: WIPICWord,
) -> Result<()> {
    tracing::debug!(
        "MC_grpCopyFrameBuffer({:#x}, {dx}, {dy}, {w}, {h}, {:#x}, {sx}, {sy}, {pgc:#x})",
        dst.0,
        src.0
    );

    let src_framebuffer = FrameBuffer(read_generic(context, context.data_ptr(src)?)?);
    let dst_framebuffer = FrameBuffer(read_generic(context, context.data_ptr(dst)?)?);

    let clip = Clip {
        x: dx as _,
        y: dy as _,
        width: w as _,
        height: h as _,
    };

    primitives::copy_framebuffer(context, &dst_framebuffer, dx, dy, w as u32, h as u32, &src_framebuffer, sx, sy, clip)
}

pub async fn get_font(_: &mut dyn WIPICContext, face: i32, size: i32, style: i32) -> Result<i32> {
    tracing::warn!("stub MC_grpGetFont({face}, {size}, {style})");

    Ok(0)
}

pub async fn get_font_height(_: &mut dyn WIPICContext, font: i32) -> Result<i32> {
    tracing::warn!("stub MC_grpGetFontHeight({font})");

    Ok(12)
}

pub async fn get_font_ascent(_: &mut dyn WIPICContext, font: i32) -> Result<i32> {
    tracing::warn!("stub MC_grpGetFontAscent({font})");

    Ok(DEFAULT_FONT_ASCENT)
}

pub async fn get_font_descent(_: &mut dyn WIPICContext, font: i32) -> Result<i32> {
    tracing::warn!("stub MC_grpGetFontDescent({font})");

    Ok(2)
}

pub async fn get_string_width(context: &mut dyn WIPICContext, font: i32, ptr_string: WIPICWord, length: i32) -> Result<i32> {
    tracing::debug!("MC_grpGetStringWidth({font}, {ptr_string:#x}, {length})");

    let Some(string) = primitives::read_text_for_measurement(context, ptr_string, length)? else {
        return Ok(0);
    };
    Ok(string_width(context.system().platform().font(), &string, 10.0) as i32)
}

pub async fn draw_string(
    context: &mut dyn WIPICContext,
    dst: WIPICIndirectPtr,
    x: i32,
    y: i32,
    ptr_string: WIPICWord,
    length: i32,
    pgc: WIPICWord,
) -> Result<()> {
    tracing::debug!("MC_grpDrawString({:#x}, {x}, {y}, {ptr_string:#x}, {length}, {pgc:#x})", dst.0);

    let Some(string) = primitives::read_text(context, ptr_string, length)? else {
        return Ok(());
    };

    let framebuffer = FrameBuffer(read_generic(context, context.data_ptr(dst)?)?);
    let gctx: WIPICGraphicsContext = read_generic(context, pgc)?;
    let (x, y) = translated_point(&gctx, x, y);

    let clip = Clip {
        x: 0,
        y: 0,
        width: framebuffer.0.width,
        height: framebuffer.0.height,
    };

    let color = framebuffer.pixel_to_color(gctx.fgpxl);
    // MC_grpDrawString receives a baseline, while the shared canvas text
    // adapter adds the default font ascent to its top-origin coordinate.
    primitives::draw_text(context, &framebuffer, &string, x, y.saturating_sub(DEFAULT_FONT_ASCENT), color, clip)
}

pub async fn repaint(context: &mut dyn WIPICContext, lcd: i32, x: i32, y: i32, width: i32, height: i32) -> Result<()> {
    tracing::debug!("MC_grpRepaint({lcd}, {x}, {y}, {width}, {height})");

    let platform = context.system().platform();
    let screen = platform.screen();
    screen.request_redraw().unwrap();

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn get_rgb_pixels(
    context: &mut dyn WIPICContext,
    src: WIPICIndirectPtr,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    pd: WIPICWord,
    ipl: i32,
) -> Result<()> {
    tracing::debug!("MC_grpGetRGBPixels({:#x}, {x}, {y}, {w}, {h}, {pd:#x}, {ipl})", src.0);
    if w <= 0 || h <= 0 {
        return Ok(());
    }
    let framebuffer = FrameBuffer(read_generic(context, context.data_ptr(src)?)?);
    let image = framebuffer.image(context)?;
    primitives::get_rgb_pixels(context, &*image, x, y, w, h, pd, ipl)
}

#[allow(clippy::too_many_arguments)]
pub async fn set_rgb_pixels(
    context: &mut dyn WIPICContext,
    dst: WIPICIndirectPtr,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    psrc: WIPICWord,
    ibpl: i32,
    _pgc: WIPICWord,
) -> Result<()> {
    tracing::debug!("MC_grpSetRGBPixels({:#x}, {x}, {y}, {w}, {h}, {psrc:#x}, {ibpl})", dst.0);
    if w <= 0 || h <= 0 {
        return Ok(());
    }

    let framebuffer = FrameBuffer(read_generic(context, context.data_ptr(dst)?)?);
    let clip = Clip {
        x: 0,
        y: 0,
        width: framebuffer.0.width,
        height: framebuffer.0.height,
    };
    primitives::set_rgb_pixels(context, &framebuffer, x, y, w, h, psrc, ibpl, clip)
}

pub async fn get_image_framebuffer(context: &mut dyn WIPICContext, image: WIPICIndirectPtr) -> Result<WIPICIndirectPtr> {
    tracing::debug!("MC_grpGetImageFrameBuffer({:#x})", image.0);

    if context.indirect_image_framebuffers() {
        return read_generic(context, context.data_ptr(image)?);
    }
    // WIPICImage starts with `img: WIPICFramebuffer` at offset 0,
    // so the image handle doubles as a framebuffer handle.
    Ok(image)
}

pub async fn get_image_property(context: &mut dyn WIPICContext, image: WIPICIndirectPtr, property: i32) -> Result<i32> {
    tracing::debug!("MC_grpGetImageProperty({:#x}, {property})", image.0);

    let image = read_image(context, image)?;

    Ok(match property {
        4 => image.img.width as _,
        5 => image.img.height as _,
        _ => {
            tracing::warn!("unknown property {property}");
            0
        }
    })
}

pub async fn draw_rect(context: &mut dyn WIPICContext, dst: WIPICIndirectPtr, x: i32, y: i32, w: i32, h: i32, pgc: WIPICWord) -> Result<()> {
    tracing::debug!("MC_grpDrawRect({:#x}, {x}, {y}, {w}, {h}, {pgc:#x})", dst.0);

    if w <= 0 || h <= 0 {
        return Ok(());
    }

    let framebuffer = FrameBuffer(read_generic(context, context.data_ptr(dst)?)?);
    let gctx: WIPICGraphicsContext = read_generic(context, pgc)?;
    let (x, y) = translated_point(&gctx, x, y);
    let clip = Clip {
        x: x as _,
        y: y as _,
        width: w as _,
        height: h as _,
    };

    let color = framebuffer.pixel_to_color(gctx.fgpxl);
    primitives::draw_rect(context, &framebuffer, x, y, w as u32, h as u32, color, clip)
}

pub async fn draw_line(context: &mut dyn WIPICContext, dst: WIPICIndirectPtr, x1: i32, y1: i32, x2: i32, y2: i32, pgc: WIPICWord) -> Result<()> {
    tracing::debug!("MC_grpDrawLine({:#x}, {x1}, {y1}, {x2}, {y2}, {pgc:#x})", dst.0);

    let framebuffer = FrameBuffer(read_generic(context, context.data_ptr(dst)?)?);
    let gctx: WIPICGraphicsContext = read_generic(context, pgc)?;
    let (x1, y1) = translated_point(&gctx, x1, y1);
    let (x2, y2) = translated_point(&gctx, x2, y2);
    let clip = Clip {
        x: 0,
        y: 0,
        width: framebuffer.0.width as _,
        height: framebuffer.0.height as _,
    };

    let color = framebuffer.pixel_to_color(gctx.fgpxl);
    primitives::draw_line(context, &framebuffer, x1, y1, x2, y2, color, clip)
}

pub async fn post_event(context: &mut dyn WIPICContext, id: i32, r#type: i32, param1: i32, param2: i32) -> Result<i32> {
    tracing::debug!("MC_grpPostEvent({id}, {type}, {param1}, {param2})");

    context.system().event_queue().push(Event::Notify { r#type, param1, param2 });

    Ok(0)
}

// it's not documented api, but lgt apps gets pointer via api call
pub async fn get_framebuffer_pointer(context: &mut dyn WIPICContext, framebuffer: WIPICIndirectPtr) -> Result<WIPICWord> {
    tracing::debug!("MC_GRP_GET_FRAME_BUFFER_POINTER({:#x})", framebuffer.0);

    let framebuffer: WIPICFramebuffer = read_generic(context, context.data_ptr(framebuffer)?)?;

    Ok(framebuffer.buf.0)
}

pub async fn get_framebuffer_width(context: &mut dyn WIPICContext, framebuffer: WIPICIndirectPtr) -> Result<i32> {
    tracing::debug!("MC_GRP_GET_FRAME_BUFFER_WIDTH({:#x})", framebuffer.0);

    let framebuffer: WIPICFramebuffer = read_generic(context, context.data_ptr(framebuffer)?)?;

    Ok(framebuffer.width as _)
}

pub async fn get_framebuffer_height(context: &mut dyn WIPICContext, framebuffer: WIPICIndirectPtr) -> Result<i32> {
    tracing::debug!("MC_GRP_GET_FRAME_BUFFER_HEIGHT({:#x})", framebuffer.0);

    let framebuffer: WIPICFramebuffer = read_generic(context, context.data_ptr(framebuffer)?)?;

    Ok(framebuffer.height as _)
}

pub async fn get_framebuffer_bpl(context: &mut dyn WIPICContext, framebuffer: WIPICIndirectPtr) -> Result<i32> {
    tracing::debug!("MC_GRP_GET_FRAME_BUFFER_BPL({:#x})", framebuffer.0);

    let framebuffer: WIPICFramebuffer = read_generic(context, context.data_ptr(framebuffer)?)?;

    Ok(framebuffer.bpl as _)
}

pub async fn get_framebuffer_bpp(context: &mut dyn WIPICContext, framebuffer: WIPICIndirectPtr) -> Result<i32> {
    tracing::debug!("MC_GRP_GET_FRAME_BUFFER_BPP({:#x})", framebuffer.0);

    let framebuffer: WIPICFramebuffer = read_generic(context, context.data_ptr(framebuffer)?)?;

    Ok(framebuffer.bpp as _)
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;
    use wie_util::ByteWrite;

    use crate::{MethodImpl, context::test::TestContext};

    use super::*;

    #[futures_test::test]
    async fn drawing_offsets_match_explicit_coordinates_for_shapes_and_text() -> Result<()> {
        for (ox, oy) in [(7i32, 5i32), (-4, -3), (0, 0)] {
            for operation in 0..7 {
                let mut context = TestContext::with_system(wie_backend::System::new(
                    Box::new(test_utils::TestPlatform::new()),
                    "",
                    "",
                    wie_backend::DefaultTaskRunner,
                ));
                let expected = create_offscreen_framebuffer(&mut context, 40, 40).await?;
                let actual = create_offscreen_framebuffer(&mut context, 40, 40).await?;
                let graphics = context.alloc_raw(size_of::<WIPICGraphicsContext>() as u32)?;
                let offset = context.alloc_raw(8)?;
                let text = context.alloc_raw(3)?;
                write_generic(&mut context, text, *b"AB\0")?;
                for (target, x, y, translation) in [(expected, 8 + ox, 16 + oy, [0, 0]), (actual, 8, 16, [ox, oy])] {
                    init_context(&mut context, graphics).await?;
                    set_context(&mut context, graphics, WIPICGraphicsContextIdx::FgPixelIdx, 0xffff).await?;
                    write_generic(&mut context, offset, translation)?;
                    set_context(&mut context, graphics, WIPICGraphicsContextIdx::OffsetIdx, offset).await?;
                    match operation {
                        0 => put_pixel(&mut context, target, x, y, graphics).await?,
                        1 => fill_rect(&mut context, target, x, y, 8, 6, graphics).await?,
                        2 => draw_rect(&mut context, target, x, y, 8, 6, graphics).await?,
                        3 => draw_line(&mut context, target, x, y, x + 8, y + 6, graphics).await?,
                        4 => draw_arc(&mut context, target, x, y, 8, 6, 0, 360, graphics).await?,
                        5 => fill_arc(&mut context, target, x, y, 8, 6, 0, 360, graphics).await?,
                        _ => draw_string(&mut context, target, x, y, text, -1, graphics).await?,
                    }
                }
                let expected = FrameBuffer(read_generic(&context, context.data_ptr(expected)?)?);
                let actual = FrameBuffer(read_generic(&context, context.data_ptr(actual)?)?);
                let mut painted = 0;
                for index in 0..1600 {
                    let a: u16 = read_generic(&context, context.data_ptr(actual.0.buf)? + index * 2)?;
                    let e: u16 = read_generic(&context, context.data_ptr(expected.0.buf)? + index * 2)?;
                    assert_eq!(a, e, "operation {operation}, offset ({ox},{oy}), pixel {index}");
                    painted += usize::from(a != 0);
                }
                assert!(painted > 0);
            }
        }
        Ok(())
    }

    #[futures_test::test]
    async fn image_offsets_translate_destination_without_moving_source_or_clip() -> Result<()> {
        for indirect in [false, true] {
            for (offset, destination, clip, origin) in [([2i32, 1i32], [0, 0], [3i32, 1, 5, 3], [2, 1]), ([-2, -1], [3, 2], [0, 0, 6, 4], [1, 1])] {
                let (mut context, target, image, graphics, framebuffer) = draw_image_fixture(indirect).await?;
                let values = context.alloc_raw(16)?;
                write_generic(&mut context, values, offset)?;
                set_context(&mut context, graphics, WIPICGraphicsContextIdx::OffsetIdx, values).await?;
                write_generic(&mut context, values, clip)?;
                set_context(&mut context, graphics, WIPICGraphicsContextIdx::ClipIdx, values).await?;
                draw_image(&mut context, target, destination[0], destination[1], 3, 2, image, 1, 1, graphics).await?;
                for y in 0i32..4 {
                    for x in 0i32..6 {
                        let expected = if x >= origin[0]
                            && x < origin[0] + 3
                            && y >= origin[1]
                            && y < origin[1] + 2
                            && x >= clip[0]
                            && x < clip[2]
                            && y >= clip[1]
                            && y < clip[3]
                        {
                            0x1001 + ((1 + y - origin[1]) * 6 + 1 + x - origin[0]) as u16
                        } else {
                            0
                        };
                        assert_eq!(
                            read_generic::<u16, _>(&context, context.data_ptr(framebuffer.0.buf)? + (y * 6 + x) as u32 * 2)?,
                            expected
                        );
                    }
                }
            }
        }
        Ok(())
    }

    #[futures_test::test]
    async fn native_text_uses_baseline_instead_of_top_origin() -> Result<()> {
        for baseline in [0, 12, 24] {
            let mut context = TestContext::with_system(wie_backend::System::new(
                Box::new(test_utils::TestPlatform::new()),
                "",
                "",
                wie_backend::DefaultTaskRunner,
            ));
            let target = create_offscreen_framebuffer(&mut context, 40, 40).await?;
            let framebuffer = FrameBuffer(read_generic(&context, context.data_ptr(target)?)?);
            let graphics = context.alloc_raw(size_of::<WIPICGraphicsContext>() as u32)?;
            init_context(&mut context, graphics).await?;
            set_context(&mut context, graphics, WIPICGraphicsContextIdx::FgPixelIdx, 0xffff).await?;
            let text = context.alloc_raw(3)?;
            write_generic(&mut context, text, *b"AB\0")?;
            draw_string(&mut context, target, 4, baseline, text, -1, graphics).await?;
            let mut painted = 0;
            for y in 0..40 {
                for x in 0..40 {
                    let pixel: u16 = read_generic(&context, context.data_ptr(framebuffer.0.buf)? + (y * 40 + x) * 2)?;
                    if pixel != 0 {
                        painted += 1;
                        assert!((y as i32) < baseline, "uppercase glyph below baseline {baseline}: ({x},{y})");
                        assert!((y as i32) >= baseline - get_font_ascent(&mut context, 0).await?);
                        assert!(x >= 4);
                    }
                }
            }
            assert_eq!(painted > 0, baseline > 0);
        }
        Ok(())
    }

    async fn draw_image_fixture(indirect: bool) -> Result<(TestContext, WIPICIndirectPtr, WIPICIndirectPtr, u32, FrameBuffer)> {
        let mut context = TestContext::new();
        context.indirect_images = indirect;
        let source = FrameBuffer::new(&mut context, 6, 5, 16)?;
        for index in 0..30 {
            let address = context.data_ptr(source.0.buf)? + index * 2;
            write_generic(&mut context, address, 0x1001u16 + index as u16)?;
        }
        let image = allocate_image(
            &mut context,
            wipi_types::wipic::WIPICImage {
                img: source.0,
                mask: FrameBuffer::empty().0,
                loop_count: 0,
                delay: 0,
                animated: 0,
                buf: WIPICIndirectPtr(0),
                offset: 0,
                current: 0,
                len: 0,
            },
        )?;
        let target = create_offscreen_framebuffer(&mut context, 6, 4).await?;
        let framebuffer = FrameBuffer(read_generic(&context, context.data_ptr(target)?)?);
        let graphics = context.alloc_raw(size_of::<WIPICGraphicsContext>() as u32)?;
        init_context(&mut context, graphics).await?;
        Ok((context, target, image, graphics, framebuffer))
    }

    #[futures_test::test]
    async fn draw_image_clips_exclusive_corners_without_shifting_source_pixels() -> Result<()> {
        for indirect in [false, true] {
            for (clip, destination, expected_bounds) in [
                ([2, 1, 4, 3], [0, 0, 5, 4], [2, 1, 4, 3]),
                ([-2, -1, 2, 2], [-1, -1, 5, 4], [0, 0, 2, 2]),
                ([2, 1, 4, 3], [3, 2, 1, 1], [3, 2, 4, 3]),
            ] {
                let (mut context, target, image, graphics, framebuffer) = draw_image_fixture(indirect).await?;
                let rectangle = context.alloc_raw(16)?;
                write_generic(&mut context, rectangle, clip)?;
                set_context(&mut context, graphics, WIPICGraphicsContextIdx::ClipIdx, rectangle).await?;
                let [dx, dy, width, height] = destination;
                draw_image(&mut context, target, dx, dy, width, height, image, 1, 1, graphics).await?;
                let [left, top, right, bottom] = expected_bounds;
                for y in 0..4 {
                    for x in 0..6 {
                        let expected = if x >= left && x < right && y >= top && y < bottom {
                            0x1001 + ((1 + y - dy) * 6 + 1 + x - dx) as u16
                        } else {
                            0
                        };
                        let actual: u16 = read_generic(&context, context.data_ptr(framebuffer.0.buf)? + ((y * 6 + x) * 2) as u32)?;
                        assert_eq!(
                            actual, expected,
                            "indirect={indirect}, clip={clip:?}, destination={destination:?}, pixel=({x},{y})"
                        );
                    }
                }
            }
        }
        Ok(())
    }

    #[futures_test::test]
    async fn draw_image_default_and_null_clips_are_unbounded_but_explicit_empty_clips_draw_nothing() -> Result<()> {
        for clip in [None, Some([0, 0, 0, 0]), Some([3, 1, 2, 3]), Some([1, 3, 4, 2])] {
            let (mut context, target, image, graphics, framebuffer) = draw_image_fixture(true).await?;
            let default = WIPICGraphicsContext {
                clip: [0, 0, i16::MAX as u16, i16::MAX as u16],
                ..WIPICGraphicsContext::default()
            };
            assert_eq!(
                bytemuck::bytes_of(&read_generic::<WIPICGraphicsContext, _>(&context, graphics)?),
                bytemuck::bytes_of(&default)
            );
            if let Some(clip) = clip {
                let rectangle = context.alloc_raw(16)?;
                write_generic(&mut context, rectangle, clip)?;
                set_context(&mut context, graphics, WIPICGraphicsContextIdx::ClipIdx, rectangle).await?;
            }
            draw_image(&mut context, target, 0, 0, 5, 4, image, 1, 1, graphics).await?;
            for y in 0..4 {
                for x in 0..6 {
                    let expected = if clip.is_none() && x < 5 {
                        0x1001 + ((y + 1) * 6 + x + 1) as u16
                    } else {
                        0
                    };
                    let actual: u16 = read_generic(&context, context.data_ptr(framebuffer.0.buf)? + ((y * 6 + x) * 2) as u32)?;
                    assert_eq!(actual, expected, "clip={clip:?}, pixel=({x},{y})");
                }
            }
            // Reset a restricted context through the existing NULL contract.
            set_context(&mut context, graphics, WIPICGraphicsContextIdx::ClipIdx, 0).await?;
            draw_image(&mut context, target, 0, 0, 5, 4, image, 1, 1, graphics).await?;
            for y in 0..4 {
                for x in 0..6 {
                    let expected = if x < 5 { 0x1001 + ((y + 1) * 6 + x + 1) as u16 } else { 0 };
                    let actual: u16 = read_generic(&context, context.data_ptr(framebuffer.0.buf)? + ((y * 6 + x) * 2) as u32)?;
                    assert_eq!(actual, expected, "reset from clip={clip:?}, pixel=({x},{y})");
                }
            }
        }
        Ok(())
    }

    #[futures_test::test]
    async fn draw_image_nonpositive_dimensions_do_not_access_guest_pixels() -> Result<()> {
        let mut context = TestContext::new();
        for (width, height) in [(0, 4), (4, 0), (-1, 4), (4, -1), (i32::MIN, i32::MIN)] {
            draw_image(&mut context, WIPICIndirectPtr(0), 0, 0, width, height, WIPICIndirectPtr(0), 0, 0, 0).await?;
        }
        assert_eq!(context.io_counts(), (0, 0));
        Ok(())
    }

    #[futures_test::test]
    async fn offscreen_cleanup_preserves_lcd_and_releases_owned_pixels() -> Result<()> {
        let mut context = TestContext::new();
        destroy_offscreen_framebuffer(&mut context, WIPICIndirectPtr(0)).await?;
        assert_eq!(context.io_counts(), (0, 0));
        let lcd = create_offscreen_framebuffer(&mut context, 4, 3).await?;
        write_generic(&mut context, SCREEN_FRAMEBUFFER_PTR, lcd.0)?;
        destroy_offscreen_framebuffer(&mut context, lcd).await?;
        assert!(context.freed.is_empty());
        let offscreen = create_offscreen_framebuffer(&mut context, 2, 2).await?;
        let data: WIPICFramebuffer = read_generic(&context, context.data_ptr(offscreen)?)?;
        destroy_offscreen_framebuffer(&mut context, offscreen).await?;
        assert_eq!(context.freed, alloc::vec![data.buf.0, offscreen.0]);
        assert_eq!(read_generic::<u32, _>(&context, SCREEN_FRAMEBUFFER_PTR)?, lcd.0);
        Ok(())
    }

    #[futures_test::test]
    async fn null_clip_resets_bounds_without_resetting_other_context_state() -> Result<()> {
        let mut context = TestContext::new();
        let address = context.alloc_raw(size_of::<WIPICGraphicsContext>() as u32)?;
        let rectangle = context.alloc_raw(16)?;
        let mut initial = WIPICGraphicsContext::default();
        initial.fgpxl = 0x123456;
        initial.bgpxl = 0xabcdef;
        initial.alpha = 128;
        initial.font = 7;
        initial.offset = [3, 4];
        write_generic(&mut context, address, initial)?;
        write_generic(&mut context, rectangle, [2i32, 3, 20, 30])?;
        let set = set_context.into_body();
        set.call(&mut context, Box::new([address, 0, rectangle])).await?;
        assert_eq!(read_generic::<WIPICGraphicsContext, _>(&context, address)?.clip, [2, 3, 20, 30]);
        set.call(&mut context, Box::new([address, 0, 0])).await?;
        let actual: WIPICGraphicsContext = read_generic(&context, address)?;
        initial.clip = [0, 0, 32767, 32767];
        assert_eq!(bytemuck::bytes_of(&actual), bytemuck::bytes_of(&initial));
        // An empty explicit rectangle remains empty, distinct from NULL.
        write_generic(&mut context, rectangle, [0i32; 4])?;
        set.call(&mut context, Box::new([address, 0, rectangle])).await?;
        assert_eq!(read_generic::<WIPICGraphicsContext, _>(&context, address)?.clip, [0; 4]);
        Ok(())
    }

    #[futures_test::test]
    async fn korean_width_probes_reserve_complete_character_cells() -> Result<()> {
        let mut context = TestContext::with_system(wie_backend::System::new(
            Box::new(test_utils::TestPlatform::new()),
            "",
            "",
            wie_backend::DefaultTaskRunner,
        ));
        let bytes = encoding_rs::EUC_KR.encode("AB 공주님을 찾아").0.into_owned();
        let address = context.alloc_raw(bytes.len() as u32)?;
        context.write_bytes(address, &bytes)?;
        // A caller stops at the first over-width byte and uses the preceding
        // prefix. Every overflow within a Korean pair must happen on its lead.
        for start in [3usize, 5, 7, 9, 12, 14] {
            let lead = get_string_width(&mut context, 0, address, (start + 1) as i32).await?;
            let complete = get_string_width(&mut context, 0, address, (start + 2) as i32).await?;
            assert_eq!(lead, complete, "split at byte {start}");
        }
        assert_eq!(primitives::read_text(&context, address + 3, 1)?.unwrap(), "\u{fffd}");
        assert_eq!(primitives::read_text_for_measurement(&context, address + 3, 2)?.unwrap(), "공");
        assert_eq!(primitives::read_text_for_measurement(&context, address, -2)?, None);
        // An invalid standalone byte must not be reclassified as a wide glyph.
        context.write_bytes(address, &[0xff])?;
        assert_eq!(primitives::read_text_for_measurement(&context, address, 1)?.unwrap(), "\u{fffd}");
        Ok(())
    }

    #[futures_test::test]
    async fn context_values_are_written_to_the_output_pointer() -> Result<()> {
        let mut context = TestContext::new();
        let ptr_context = context.alloc_raw(size_of::<WIPICGraphicsContext>() as u32)?;
        let input = context.alloc_raw(16)?;
        let output = context.alloc_raw(20)?;
        init_context(&mut context, ptr_context).await?;

        let set = set_context.into_body();
        let get = get_context.into_body();
        for (op, value) in [
            (1, 0x12345678),
            (2, 0x87654321),
            (3, 0xff00ff),
            (4, 128),
            (5, 0x1001),
            (6, 42),
            (7, 8),
            (8, 1),
        ] {
            set.call(&mut context, Box::new([ptr_context, op, value])).await?;
            write_generic(&mut context, output, [0xccccccccu32; 5])?;
            get.call(&mut context, Box::new([ptr_context, op, output])).await?;
            assert_eq!(
                read_generic::<[u32; 5], _>(&context, output)?,
                [value, 0xcccccccc, 0xcccccccc, 0xcccccccc, 0xcccccccc]
            );
        }

        write_generic(&mut context, input, [-5i32, -8, 176, 220])?;
        set.call(&mut context, Box::new([ptr_context, 0, input])).await?;
        write_generic(&mut context, output, [999i32; 5])?;
        get.call(&mut context, Box::new([ptr_context, 0, output])).await?;
        assert_eq!(read_generic::<[i32; 5], _>(&context, output)?, [-5, -8, 176, 220, 999]);

        write_generic(&mut context, input, [-12i32, 34])?;
        set.call(&mut context, Box::new([ptr_context, 10, input])).await?;
        write_generic(&mut context, output, [999i32; 5])?;
        get.call(&mut context, Box::new([ptr_context, 10, output])).await?;
        assert_eq!(read_generic::<[i32; 5], _>(&context, output)?, [-12, 34, 999, 999, 999]);

        get.call(&mut context, Box::new([ptr_context, 0xff, output])).await?;
        assert_eq!(read_generic::<[i32; 5], _>(&context, output)?, [-12, 34, 999, 999, 999]);
        Ok(())
    }
}
