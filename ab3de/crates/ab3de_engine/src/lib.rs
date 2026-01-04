#![feature(cfg_select)]
#![feature(decl_macro)]

mod app;
mod camera_controller;
mod drawing;
mod embedded_demo_resources;
mod io;
mod model_loaders;
mod utils;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(all(target_arch = "wasm32", feature = "wasm-weblike-manual"))]
use crate::io::window_handling::weblike_manual::WeblikeManualWindowHandler;
use crate::{
    app::App,
    drawing::textures,
    io::window_handling::{ApplicationInit, winit::WinitWindowHandler},
};

pub fn run_winit() -> anyhow::Result<()> {
    let event_loop = winit::event_loop::EventLoop::with_user_event().build()?;

    let init: ApplicationInit = Box::new(|surface_target, ctx, size| {
        Box::pin(App::try_new_as_boxed_handler(surface_target, ctx, size))
    });
    let mut handler = WinitWindowHandler::new(
        init,
        #[cfg(target_arch = "wasm32")]
        &event_loop,
    );

    event_loop.run_app(&mut handler)?;

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_native_winit() -> anyhow::Result<()> {
    env_logger::init();
    run_winit()
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-winit"))]
#[wasm_bindgen(start)]
pub fn run_web_winit() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).unwrap_throw();

    run_winit().unwrap_throw();

    Ok(())
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-weblike-manual"))]
static HAS_INITIALIZED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(all(target_arch = "wasm32", feature = "wasm-weblike-manual"))]
#[wasm_bindgen]
pub struct Runner {
    state: RunerState,
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-weblike-manual"))]
enum RunerState {
    NotStarted(web_sys::OffscreenCanvas),
    Started(WeblikeManualWindowHandler),
    Invalid,
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-weblike-manual"))]
macro runner_app($self:ident) {
    match &mut $self.state {
        RunerState::Started(app) => app,
        _ => return,
    }
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-weblike-manual"))]
#[wasm_bindgen]
impl Runner {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: web_sys::OffscreenCanvas) -> Runner {
        if !HAS_INITIALIZED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            console_error_panic_hook::set_once();
            console_log::init_with_level(log::Level::Debug).unwrap_throw();
        }

        Runner {
            state: RunerState::NotStarted(canvas),
        }
    }

    pub async fn start(&mut self) -> Result<(), wasm_bindgen::JsValue> {
        let ret;
        (self.state, ret) = match core::mem::replace(&mut self.state, RunerState::Invalid) {
            RunerState::NotStarted(canvas) => {
                let handler = WeblikeManualWindowHandler::new(
                    canvas,
                    Box::new(|surface_target, ctx, size| {
                        Box::pin(App::try_new_as_boxed_handler(surface_target, ctx, size))
                    }),
                )
                .await;

                (RunerState::Started(handler), Ok(()))
            }
            RunerState::Started(app) => (
                RunerState::Started(app),
                Err(wasm_bindgen::JsValue::from_str(
                    "Runner has already been started!",
                )),
            ),
            RunerState::Invalid => unreachable!(),
        };

        ret
    }

    pub fn handle_resized(&mut self, width: u32, height: u32) {
        let app = runner_app!(self);
        app.handle_resized(width, height);
    }

    pub fn handle_redraw_requested(&mut self) {
        let app = runner_app!(self);
        app.handle_redraw_requested();
    }

    pub fn handle_input_mouse_motion(&mut self, delta_x: f64, delta_y: f64) {
        let app = runner_app!(self);
        app.handle_input_mouse_motion(delta_x, delta_y);
    }

    pub fn handle_input_keyboard(&mut self, physical_key_code: &str, is_down: bool) {
        let app = runner_app!(self);
        app.handle_input_keyboard(physical_key_code, is_down);
    }

    pub fn handle_input_mouse_wheel(&mut self, delta_x: f64, delta_y: f64, delta_mode: u8) {
        let app = runner_app!(self);
        app.handle_input_mouse_wheel(delta_x, delta_y, delta_mode);
    }

    pub fn handle_input_mouse_input(&mut self, button: u8, is_down: bool) {
        let app = runner_app!(self);
        app.handle_input_mouse_input(button, is_down);
    }
}
