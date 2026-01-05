use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

use crate::{
    camera_controller::{CameraController, CameraControllerInput},
    engine::{Engine, Viewport},
    io::window_handling::{ElementState, KeyCode, MouseButton, MouseScrollDelta, PhysicalKey},
    utils,
};

pub struct WinitWindowHandler {
    state: State,

    window: Option<Arc<Window>>,
}

enum State {
    Uninitialized,
    Ready(StateReady),
}

struct StateReady {
    device: wgpu::Device,
    queue: wgpu::Queue,

    engine: Engine,
    viewport: Viewport,
    camera_controller: CameraController,

    update_time_ms: u64,
}

impl WinitWindowHandler {
    pub fn new() -> Self {
        Self {
            state: State::Uninitialized,
            window: None,
        }
    }
}

impl ApplicationHandler for WinitWindowHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let State::Uninitialized = &mut self.state else {
            return;
        };

        let window_attributes = winit::window::WindowAttributes::default();

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window = Some(window.clone());
        let size = window.inner_size();
        let size = (size.width, size.height).into();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = {
            let adapter_future = instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            });
            pollster::block_on(adapter_future).unwrap()
        };

        let (device, queue) = {
            let device_future = adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("[ApplicationHandler::resumed]"),
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::defaults(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            });
            pollster::block_on(device_future).unwrap()
        };

        let engine = Engine::try_new(device.clone(), queue.clone()).unwrap();

        let viewport = engine.make_viewport(surface, &adapter, size);

        self.state = State::Ready(StateReady {
            device,
            queue,
            engine,
            viewport,
            camera_controller: CameraController::new(4.0, 0.4),
            update_time_ms: utils::now_ms(),
        });
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let s = match &mut self.state {
            State::Ready(state) => state,
            _ => return,
        };

        let _has_consumed = match event {
            DeviceEvent::MouseMotion { delta } => s
                .camera_controller
                .handle_input(CameraControllerInput::MouseMotion { delta }),
            _ => false,
        };
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let s = match &mut self.state {
            State::Ready(state) => state,
            _ => return,
        };

        let Some(window) = &self.window else {
            return;
        };

        if window_id != window.id() {
            return;
        }

        let has_consumed = match &event {
            WindowEvent::KeyboardInput { event, .. } => {
                s.camera_controller
                    .handle_input(CameraControllerInput::KeyboardInput {
                        physical_key: event.physical_key.into(),
                        state: event.state.into(),
                    })
            }
            WindowEvent::MouseWheel { delta, .. } => {
                s.camera_controller
                    .handle_input(CameraControllerInput::MouseWheel {
                        delta: (*delta).into(),
                    })
            }
            WindowEvent::MouseInput { button, state, .. } => {
                s.camera_controller
                    .handle_input(CameraControllerInput::MouseInput {
                        button: (*button).into(),
                        state: (*state).into(),
                    })
            }
            _ => false,
        };
        if has_consumed {
            return;
        }

        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key:
                            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape),
                        state: winit::event::ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                s.viewport
                    .resize(&s.device, &s.queue, size.width, size.height);
            }
            WindowEvent::RedrawRequested => {
                let last_ms = core::mem::replace(&mut s.update_time_ms, utils::now_ms());
                let dt_ms = s.update_time_ms - last_ms;
                let dt_s = dt_ms as f32 / 1000.0;

                s.viewport.update_camera(&s.queue, |camera_data| {
                    s.camera_controller.update_camera(camera_data, dt_s);
                });

                window.request_redraw();
                s.engine.update(s.update_time_ms, dt_s);

                match s.engine.render(&s.viewport) {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = window.inner_size();
                        s.viewport
                            .resize(&s.device, &s.queue, size.width, size.height);
                    }
                    Err(e) => {
                        log::error!("Unable to render {}", e)
                    }
                }
            }
            _ => {}
        }
    }
}

impl From<winit::keyboard::PhysicalKey> for PhysicalKey {
    fn from(value: winit::keyboard::PhysicalKey) -> Self {
        match value {
            winit::keyboard::PhysicalKey::Code(code) => PhysicalKey::Code(code.into()),
            _ => PhysicalKey::Other,
        }
    }
}

impl From<winit::keyboard::KeyCode> for KeyCode {
    fn from(value: winit::keyboard::KeyCode) -> Self {
        match value {
            winit::keyboard::KeyCode::ArrowUp => KeyCode::ArrowUp,
            winit::keyboard::KeyCode::ArrowDown => KeyCode::ArrowDown,
            winit::keyboard::KeyCode::ArrowLeft => KeyCode::ArrowLeft,
            winit::keyboard::KeyCode::ArrowRight => KeyCode::ArrowRight,
            winit::keyboard::KeyCode::KeyW => KeyCode::KeyW,
            winit::keyboard::KeyCode::KeyS => KeyCode::KeyS,
            winit::keyboard::KeyCode::KeyA => KeyCode::KeyA,
            winit::keyboard::KeyCode::KeyD => KeyCode::KeyD,
            winit::keyboard::KeyCode::Space => KeyCode::Space,
            winit::keyboard::KeyCode::ShiftLeft => KeyCode::ShiftLeft,
            _ => KeyCode::Other,
        }
    }
}

impl From<winit::event::MouseScrollDelta> for MouseScrollDelta {
    fn from(value: winit::event::MouseScrollDelta) -> Self {
        match value {
            winit::event::MouseScrollDelta::LineDelta(x, y) => MouseScrollDelta::LineDelta(x, y),
            winit::event::MouseScrollDelta::PixelDelta(pos) => {
                MouseScrollDelta::PixelDelta((pos.x, pos.y))
            }
        }
    }
}

impl From<winit::event::MouseButton> for MouseButton {
    fn from(value: winit::event::MouseButton) -> Self {
        match value {
            winit::event::MouseButton::Left => MouseButton::Left,
            _ => MouseButton::Other,
        }
    }
}

impl From<winit::event::ElementState> for ElementState {
    fn from(value: winit::event::ElementState) -> Self {
        match value {
            winit::event::ElementState::Pressed => ElementState::Pressed,
            winit::event::ElementState::Released => ElementState::Released,
        }
    }
}
