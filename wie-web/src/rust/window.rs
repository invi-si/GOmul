use alloc::{sync::Arc, vec::Vec};
use core::{
    cell::RefCell,
    sync::atomic::{AtomicBool, Ordering},
};
use wasm_bindgen::prelude::*;
use wie_backend::{Screen, canvas::Image};
use wie_util::{Result, WieError};

#[wasm_bindgen(module = "/src/ts/wasm_screen.ts")]
extern "C" {
    type ScreenTarget;
    #[wasm_bindgen(constructor)]
    fn new(canvas: JsValue) -> ScreenTarget;
    #[wasm_bindgen(method)]
    fn resize(this: &ScreenTarget, width: u32, height: u32);
    #[wasm_bindgen(method)]
    fn width(this: &ScreenTarget) -> u32;
    #[wasm_bindgen(method)]
    fn height(this: &ScreenTarget) -> u32;
    #[wasm_bindgen(method)]
    fn paint(this: &ScreenTarget, bytes: &[u8]);
}

pub struct WindowImpl {
    target: ScreenTarget,
    display_override: Option<(u32, u32, bool)>,
    rgba: RefCell<Vec<u8>>,
    should_redraw: Arc<AtomicBool>,
}
// Each WASM instance and its JS bridge are confined to one browser thread.
unsafe impl Send for WindowImpl {}
unsafe impl Sync for WindowImpl {}
impl WindowImpl {
    pub fn new(canvas: JsValue, should_redraw: Arc<AtomicBool>, display_override: Option<(u32, u32, bool)>) -> Self {
        Self {
            target: ScreenTarget::new(canvas),
            display_override,
            rgba: RefCell::new(Vec::new()),
            should_redraw,
        }
    }
}
impl Screen for WindowImpl {
    fn display_override(&self) -> Option<(u32, u32, bool)> {
        self.display_override
    }
    fn resize(&self, width: u32, height: u32) -> Result<()> {
        if width == 0 || height == 0 || width > 1024 || height > 1024 {
            return Err(WieError::FatalError("Unsupported display size".into()));
        }
        self.target.resize(width, height);
        self.request_redraw()
    }
    fn request_redraw(&self) -> Result<()> {
        self.should_redraw.store(true, Ordering::SeqCst);
        Ok(())
    }
    fn paint(&self, image: &dyn Image) {
        let mut rgba = self.rgba.borrow_mut();
        write_lcd_rgba(image, &mut rgba);
        if self.target.width() != image.width() || self.target.height() != image.height() {
            self.target.resize(image.width(), image.height());
        }
        self.target.paint(&rgba);
    }
    fn width(&self) -> u32 {
        self.target.width()
    }
    fn height(&self) -> u32 {
        self.target.height()
    }
}

fn write_lcd_rgba(image: &dyn Image, rgba: &mut Vec<u8>) {
    image.write_rgba(rgba);
    // A phone LCD is opaque, matching the Android presenter.
    for pixel in rgba.chunks_exact_mut(4) {
        pixel[3] = 255;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wie_backend::canvas::{ArgbPixel, Color, ImageBuffer, VecImageBuffer};
    #[test]
    fn presentation_preserves_rgb_and_makes_lcd_opaque() {
        let mut image = VecImageBuffer::<ArgbPixel>::new(1, 1);
        image.put_pixel(0, 0, Color { r: 31, g: 47, b: 59, a: 0 });
        let mut bytes = Vec::new();
        write_lcd_rgba(&image, &mut bytes);
        assert_eq!(bytes, alloc::vec![31, 47, 59, 255]);
    }
}
