use alloc::{sync::Arc, vec::Vec};
use core::{
    cell::RefCell,
    sync::atomic::{AtomicBool, Ordering},
};

use wasm_bindgen::{Clamped, JsCast};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

use wie_backend::{Screen, canvas::Image};
use wie_util::Result;

pub struct WindowImpl {
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,
    rgba: RefCell<Vec<u8>>,
    should_redraw: Arc<AtomicBool>,
}

unsafe impl Send for WindowImpl {} // XXX We're on wasm, so it's fine
unsafe impl Sync for WindowImpl {}

impl WindowImpl {
    pub fn new(canvas: HtmlCanvasElement, should_redraw: Arc<AtomicBool>) -> Self {
        let context = canvas.get_context("2d").unwrap().unwrap().dyn_into::<CanvasRenderingContext2d>().unwrap();
        Self {
            canvas,
            context,
            rgba: RefCell::new(Vec::new()),
            should_redraw,
        }
    }
}

impl Screen for WindowImpl {
    fn resize(&self, width: u32, height: u32) -> Result<()> {
        self.canvas.set_width(width);
        self.canvas.set_height(height);
        self.request_redraw()
    }

    fn request_redraw(&self) -> Result<()> {
        self.should_redraw.store(true, Ordering::SeqCst);

        Ok(())
    }

    fn paint(&self, image: &dyn Image) {
        let mut rgba = self.rgba.borrow_mut();
        image.write_rgba(&mut rgba);
        let data = ImageData::new_with_u8_clamped_array_and_sh(Clamped(&rgba), self.width(), self.height()).unwrap();

        self.context.put_image_data(&data, 0.0, 0.0).unwrap();
    }

    fn width(&self) -> u32 {
        self.canvas.width()
    }

    fn height(&self) -> u32 {
        self.canvas.height()
    }
}
