use std::{pin::Pin, sync::Arc};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use winit::event_loop::EventLoop;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

use crate::io::window_handling::{
    ElementState, Input, KeyCode, MouseButton, MouseScrollDelta, PhysicalKey,
};

pub struct NativeWinitWindowHandler {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<UserEvent>>,

    state: State,
    window: Option<Arc<Window>>,
}

type Init = Box<
    dyn FnOnce(
        wgpu::SurfaceTarget<'static>, // surface_target
        Box<dyn Fn() + 'static>,      // request_redraw
        glam::UVec2,                  // size
    ) -> Pin<
        Box<dyn std::future::Future<Output = anyhow::Result<InnerHandler>> + 'static>,
    >,
>;
type InnerHandler = Box<dyn super::SimpleApplicationEventHandler + 'static>;

#[allow(unused)]
pub struct UserEvent {
    inner_handler: InnerHandler,
    window: Arc<Window>,
}

enum State {
    Uninitialized(Option<Init>),
    Ready(InnerHandler),
}

impl NativeWinitWindowHandler {
    pub fn new(
        init: Init,
        #[cfg(target_arch = "wasm32")] event_loop: &EventLoop<UserEvent>,
    ) -> Self {
        Self {
            #[cfg(target_arch = "wasm32")]
            proxy: Some(event_loop.create_proxy()),

            state: State::Uninitialized(Some(init)),
            window: None,
        }
    }
}

impl ApplicationHandler<UserEvent> for NativeWinitWindowHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let State::Uninitialized(init) = &mut self.state else {
            return;
        };
        let init = init.take();
        let Some(init) = init else {
            return;
        };

        #[allow(unused_mut)]
        let mut window_attributes = winit::window::WindowAttributes::default();

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window = Some(window.clone());
        let request_redraw = {
            let window = window.clone();
            Box::new(move || window.request_redraw())
        };
        let size = window.inner_size();
        let size = (size.width, size.height).into();

        cfg_select! {
          target_arch = "wasm32" => {
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    let inner_handler = (init)(window.clone().into(), request_redraw, size).await;
                    let inner_handler = inner_handler.unwrap();
                    assert!(proxy.send_event(UserEvent { inner_handler, window }).is_ok())
                })
            }
          }
          _ => {
            let inner_handler_future = (init)(window.into(), request_redraw, size);
            self.state = State::Ready(pollster::block_on(inner_handler_future).unwrap());
          }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut user_event: UserEvent) {
        user_event.inner_handler.handle_redraw_requested(None);
        let size = user_event.window.inner_size();
        user_event
            .inner_handler
            .handle_resized((size.width, size.height));
        self.state = State::Ready(user_event.inner_handler);
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
                let window = window.clone();
                inner_handler.handle_redraw_requested(Some(Box::new(move || {
                    let size = window.inner_size();
                    (size.width, size.height)
                })));
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
