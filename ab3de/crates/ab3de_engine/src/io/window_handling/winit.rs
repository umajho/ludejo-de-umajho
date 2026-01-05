use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

use crate::io::window_handling::{
    Application, ApplicationContext, ApplicationInit, ElementState, Input, KeyCode, MouseButton,
    MouseScrollDelta, PhysicalKey,
};

pub struct WinitWindowHandler {
    state: State,
    window: Option<Arc<Window>>,
}

enum State {
    Uninitialized(Option<ApplicationInit>),
    Ready(Application),
}

impl WinitWindowHandler {
    pub fn new(init: ApplicationInit) -> Self {
        Self {
            state: State::Uninitialized(Some(init)),
            window: None,
        }
    }
}

impl ApplicationHandler for WinitWindowHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let State::Uninitialized(init) = &mut self.state else {
            return;
        };
        let init = init.take();
        let Some(init) = init else {
            return;
        };

        let window_attributes = winit::window::WindowAttributes::default();

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window = Some(window.clone());
        let size = window.inner_size();
        let size = (size.width, size.height).into();

        let ctx = Box::new(WinitApplicationContext {
            window: window.clone(),
        });

        let inner_handler_future = (init)(window.into(), ctx, size);
        self.state = State::Ready(pollster::block_on(inner_handler_future).unwrap());
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let inner_handler = match &mut self.state {
            State::Ready(inner_handler) => inner_handler,
            _ => return,
        };

        let _has_consumed = match event {
            DeviceEvent::MouseMotion { delta } => {
                inner_handler.handle_input(Input::MouseMotion { delta })
            }
            _ => false,
        };
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let inner_handler = match &mut self.state {
            State::Ready(inner_handler) => inner_handler,
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
                inner_handler.handle_input(Input::KeyboardInput {
                    physical_key: event.physical_key.into(),
                    state: event.state.into(),
                })
            }
            WindowEvent::MouseWheel { delta, .. } => {
                inner_handler.handle_input(Input::MouseWheel {
                    delta: (*delta).into(),
                })
            }
            WindowEvent::MouseInput { button, state, .. } => {
                inner_handler.handle_input(Input::MouseInput {
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
                inner_handler.handle_resized((size.width, size.height));
            }
            WindowEvent::RedrawRequested => {
                inner_handler.handle_redraw_requested();
            }
            _ => {}
        }
    }
}

struct WinitApplicationContext {
    window: Arc<Window>,
}

impl ApplicationContext for WinitApplicationContext {
    fn request_redraw(&self) {
        self.window.request_redraw();
    }

    fn window_size(&self) -> (u32, u32) {
        let size = self.window.inner_size();
        (size.width, size.height)
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
