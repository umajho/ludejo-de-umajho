use std::fmt::Debug;

use wasm_bindgen::{JsValue, UnwrapThrowExt};
use web_sys::{
    OffscreenCanvas,
    js_sys::{Function, Reflect},
};

use crate::{
    app::App,
    io::window_handling::{
        ApplicationContext, ElementState, Input, KeyCode, MouseScrollDelta, PhysicalKey,
        SimpleApplicationEventHandler,
    },
};

pub struct WeblikeManualWindowHandler {
    app: App,
}

impl WeblikeManualWindowHandler {
    pub async fn new(canvas: OffscreenCanvas) -> Self {
        let surface_target = wgpu::SurfaceTarget::OffscreenCanvas(canvas.clone());
        let ctx = Box::new(WeblikeManualApplicationContext::new(canvas.clone()));
        let size = glam::uvec2(canvas.width(), canvas.height());

        Self {
            app: App::try_new(surface_target, ctx, size).await.unwrap_throw(),
        }
    }

    pub fn handle_resized(&mut self, width: u32, height: u32) {
        self.app.handle_resized((width, height));
    }

    pub fn handle_redraw_requested(&mut self) {
        self.app.handle_redraw_requested();
    }

    pub fn handle_input_mouse_motion(&mut self, delta_x: f64, delta_y: f64) {
        _ = self.app.handle_input(Input::MouseMotion {
            delta: (delta_x, delta_y),
        });
    }

    pub fn handle_input_keyboard(&mut self, physical_key_code: &str, is_down: bool) {
        let physical_key = match physical_key_code {
            "ArrowUp" => PhysicalKey::Code(KeyCode::ArrowUp),
            "ArrowDown" => PhysicalKey::Code(KeyCode::ArrowDown),
            "ArrowLeft" => PhysicalKey::Code(KeyCode::ArrowLeft),
            "ArrowRight" => PhysicalKey::Code(KeyCode::ArrowRight),
            "KeyW" => PhysicalKey::Code(KeyCode::ArrowUp),
            "KeyS" => PhysicalKey::Code(KeyCode::ArrowDown),
            "KeyA" => PhysicalKey::Code(KeyCode::ArrowLeft),
            "KeyD" => PhysicalKey::Code(KeyCode::ArrowRight),
            "Space" => PhysicalKey::Code(KeyCode::Space),
            "ShiftLeft" => PhysicalKey::Code(KeyCode::ShiftLeft),
            _ => PhysicalKey::Code(KeyCode::Other),
        };

        let state = if is_down {
            ElementState::Pressed
        } else {
            ElementState::Released
        };

        _ = self.app.handle_input(Input::KeyboardInput {
            physical_key,
            state,
        });
    }

    const WHEEL_EVENT_DOM_DELTA_PIXEL: u8 = 0;
    const WHEEL_EVENT_DOM_DELTA_LINE: u8 = 1;

    pub fn handle_input_mouse_wheel(&mut self, delta_x: f64, delta_y: f64, delta_mode: u8) {
        let delta = match delta_mode {
            Self::WHEEL_EVENT_DOM_DELTA_PIXEL => MouseScrollDelta::PixelDelta((delta_x, delta_y)),
            Self::WHEEL_EVENT_DOM_DELTA_LINE => {
                MouseScrollDelta::LineDelta(delta_x as f32, delta_y as f32)
            }
            _ => return,
        };

        _ = self.app.handle_input(Input::MouseWheel { delta });
    }

    pub fn handle_input_mouse_input(&mut self, button: u8, is_down: bool) {
        let button = match button {
            0 => crate::io::window_handling::MouseButton::Left,
            _ => crate::io::window_handling::MouseButton::Other,
        };

        let state = if is_down {
            ElementState::Pressed
        } else {
            ElementState::Released
        };

        _ = self.app.handle_input(Input::MouseInput { button, state });
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
