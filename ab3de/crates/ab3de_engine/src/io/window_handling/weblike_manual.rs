use std::fmt::Debug;

use wasm_bindgen::{JsValue, UnwrapThrowExt};
use web_sys::{
    OffscreenCanvas,
    js_sys::{Function, Reflect},
};

use crate::{
    camera_controller::{CameraController, CameraControllerInput},
    engine::{Engine, Viewport},
    io::window_handling::{ElementState, KeyCode, MouseScrollDelta, PhysicalKey},
    utils,
};

pub struct WeblikeManualWindowHandler {
    ctx: WeblikeManualApplicationContext,

    device: wgpu::Device,
    queue: wgpu::Queue,

    engine: Engine,
    viewport: Viewport,
    camera_controller: CameraController,

    update_time_ms: u64,
}

impl WeblikeManualWindowHandler {
    pub async fn new(canvas: OffscreenCanvas) -> Self {
        let surface_target = wgpu::SurfaceTarget::OffscreenCanvas(canvas.clone());
        let ctx = WeblikeManualApplicationContext::new(canvas.clone());
        let size = glam::uvec2(canvas.width(), canvas.height());

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(surface_target).unwrap_throw();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap_throw();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("[ApplicationHandler::resumed]"),
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::defaults(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap_throw();

        let engine = Engine::try_new(device.clone(), queue.clone()).unwrap_throw();

        let viewport = engine.make_viewport(surface, &adapter, size);

        Self {
            ctx,
            device,
            queue,
            engine,
            viewport,
            camera_controller: CameraController::default(),
            update_time_ms: utils::now_ms(),
        }
    }

    pub fn handle_resized(&mut self, width: u32, height: u32) {
        self.viewport
            .resize(&self.device, &self.queue, width, height);
    }

    pub fn handle_redraw_requested(&mut self) {
        let last_ms = core::mem::replace(&mut self.update_time_ms, utils::now_ms());
        let dt_ms = self.update_time_ms - last_ms;
        let dt_s = dt_ms as f32 / 1000.0;

        self.viewport.update_camera(&self.queue, |camera_data| {
            self.camera_controller.update_camera(camera_data, dt_s);
        });

        self.ctx.request_redraw();
        self.engine.update(self.update_time_ms, dt_s);
        match self.engine.render(&self.viewport) {
            Ok(_) => {}
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                let size = self.ctx.window_size();
                self.viewport
                    .resize(&self.device, &self.queue, size.0, size.1);
            }
            Err(e) => {
                log::error!("Unable to render {}", e)
            }
        }
    }

    pub fn handle_input_mouse_motion(&mut self, delta_x: f64, delta_y: f64) {
        self.camera_controller
            .handle_input(CameraControllerInput::MouseMotion {
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

        self.camera_controller
            .handle_input(CameraControllerInput::KeyboardInput {
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

        self.camera_controller
            .handle_input(CameraControllerInput::MouseWheel { delta });
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

        self.camera_controller
            .handle_input(CameraControllerInput::MouseInput { button, state });
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

impl WeblikeManualApplicationContext {
    fn request_redraw(&self) {
        self.request_redraw.call0(&self.canvas).unwrap_throw();
    }

    fn window_size(&self) -> (u32, u32) {
        (self.canvas.width() as u32, self.canvas.height() as u32)
    }
}
