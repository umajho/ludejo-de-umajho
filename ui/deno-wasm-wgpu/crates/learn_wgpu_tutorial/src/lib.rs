#![feature(cfg_select)]

use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use winit::event_loop::EventLoop;
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

pub struct State {
    window: Arc<Window>,
}

impl State {
    pub async fn try_new(window: Arc<Window>) -> anyhow::Result<Self> {
        Ok(Self { window })
    }

    pub fn resize(&mut self, _width: u32, _height: u32) {
        // todo!()
    }

    pub fn render(&mut self) {
        self.window.request_redraw();
    }
}

pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>,
}

impl App {
    #[cfg(target_arch = "wasm32")]
    pub fn new(event_loop: &EventLoop<State>) -> Self {
        Self {
            proxy: Some(event_loop.create_proxy()),
            state: None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> Self {
        Self { state: None }
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
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

        cfg_select! {
          target_arch = "wasm32" => {
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(
                        proxy
                            .send_event(
                                State::try_new(window)
                                    .await
                                    .expect("Unabled to create canvas!!!")
                            )
                            .is_ok()
                    )
                })
            }
          }
          _ => {
            self.state = Some(pollster::block_on(State::try_new(window)).unwrap());
          }
        }
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        #[cfg(target_arch = "wasm32")]
        {
            event.window.request_redraw();
            event.resize(
                event.window.inner_size().width,
                event.window.inner_size().height,
            );
        }
        self.state = Some(event);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(state) => state,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => state.render(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => match (code, key_state.is_pressed()) {
                (KeyCode::Escape, true) => event_loop.exit(),
                _ => {}
            },
            _ => {}
        }
    }
}
