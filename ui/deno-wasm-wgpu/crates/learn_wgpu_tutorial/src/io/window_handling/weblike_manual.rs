use std::fmt::Debug;

use wasm_bindgen::{JsValue, UnwrapThrowExt};
use web_sys::{
    OffscreenCanvas,
    js_sys::{Function, Reflect},
};

use crate::io::window_handling::{
    ApplicationContext, ApplicationInit, SimpleApplicationEventHandler,
};

pub struct WeblikeManualWindowHandler {
    app: Box<dyn SimpleApplicationEventHandler>,
}

impl WeblikeManualWindowHandler {
    pub async fn new(canvas: OffscreenCanvas, init: ApplicationInit) -> Self {
        let surface_target = wgpu::SurfaceTarget::OffscreenCanvas(canvas.clone());
        let ctx = Box::new(WeblikeManualApplicationContext::new(canvas.clone()));
        let size = glam::uvec2(canvas.width(), canvas.height());

        Self {
            app: (init)(surface_target, ctx, size).await.unwrap_throw(),
        }
    }

    pub fn handle_resized(&mut self, width: u32, height: u32) {
        self.app.handle_resized((width, height));
    }

    pub fn handle_redraw_requested(&mut self) {
        self.app.handle_redraw_requested();
    }
}

impl Debug for WeblikeManualWindowHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WeblikeManualWindowHandler").finish()
    }
}

struct WeblikeManualApplicationContext {
    canvas: OffscreenCanvas,
    request_redraw: Function,
}

impl WeblikeManualApplicationContext {
    fn new(canvas: OffscreenCanvas) -> Self {
        let request_redraw =
            Reflect::get(&canvas, &JsValue::from_str("xRequestRedraw")).unwrap_throw();
        let request_redraw: Function = request_redraw.into();

        Self {
            canvas,
            request_redraw,
        }
    }
}

impl ApplicationContext for WeblikeManualApplicationContext {
    fn request_redraw(&self) {
        self.request_redraw.call0(&self.canvas).unwrap_throw();
    }

    fn window_size(&self) -> (u32, u32) {
        (self.canvas.width() as u32, self.canvas.height() as u32)
    }
}
