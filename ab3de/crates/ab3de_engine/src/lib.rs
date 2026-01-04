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

thread_local! {
    #[cfg(all(target_arch = "wasm32", feature = "wasm-weblike-manual"))]
    static HANDLER: std::sync::OnceLock<std::cell::RefCell<WeblikeManualWindowHandler>> =
        std::sync::OnceLock::new();
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-weblike-manual"))]
#[wasm_bindgen]
pub async fn run_web_weblike_manual(
    canvas: web_sys::OffscreenCanvas,
) -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).unwrap_throw();

    let handler = WeblikeManualWindowHandler::new(
        canvas,
        Box::new(|surface_target, ctx, size| {
            Box::pin(App::try_new_as_boxed_handler(surface_target, ctx, size))
        }),
    )
    .await;

    HANDLER.with(|cell| {
        cell.set(std::cell::RefCell::new(handler)).unwrap();
    });

    Ok(())
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-weblike-manual"))]
#[wasm_bindgen]
pub fn handle_resized(width: u32, height: u32) {
    HANDLER.with(|cell| {
        if let Some(handler) = cell.get() {
            handler.borrow_mut().handle_resized(width, height);
        }
    });
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-weblike-manual"))]
#[wasm_bindgen]
pub fn handle_redraw_requested() {
    HANDLER.with(|cell| {
        if let Some(handler) = cell.get() {
            handler.borrow_mut().handle_redraw_requested();
        }
    });
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-weblike-manual"))]
#[wasm_bindgen]
pub fn handle_input_mouse_motion(delta_x: f64, delta_y: f64) {
    HANDLER.with(|cell| {
        if let Some(handler) = cell.get() {
            handler
                .borrow_mut()
                .handle_input_mouse_motion(delta_x, delta_y);
        }
    });
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-weblike-manual"))]
#[wasm_bindgen]
pub fn handle_input_keyboard(physical_key_code: &str, is_down: bool) {
    HANDLER.with(|cell| {
        if let Some(handler) = cell.get() {
            handler
                .borrow_mut()
                .handle_input_keyboard(physical_key_code, is_down);
        }
    });
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-weblike-manual"))]
#[wasm_bindgen]
pub fn handle_input_mouse_wheel(delta_x: f64, delta_y: f64, delta_mode: u8) {
    HANDLER.with(|cell| {
        if let Some(handler) = cell.get() {
            handler
                .borrow_mut()
                .handle_input_mouse_wheel(delta_x, delta_y, delta_mode);
        }
    });
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-weblike-manual"))]
#[wasm_bindgen]
pub fn handle_input_mouse_input(button: u8, is_down: bool) {
    HANDLER.with(|cell| {
        if let Some(handler) = cell.get() {
            handler
                .borrow_mut()
                .handle_input_mouse_input(button, is_down);
        }
    });
}
