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

use crate::{
    app::App, drawing::textures, io::window_handling::native_winit::NativeWinitWindowHandler,
};

pub fn run() -> anyhow::Result<()> {
    cfg_select! {
        target_arch = "wasm32" => {
            console_log::init_with_level(log::Level::Debug).unwrap_throw();
        }
        _ => {
            env_logger::init();
        }
    }

    let event_loop = winit::event_loop::EventLoop::with_user_event().build()?;

    let mut handler = NativeWinitWindowHandler::new(
        Box::new(|surface_target, ctx, size| {
            Box::pin(App::try_new_as_boxed_handler(surface_target, ctx, size))
        }),
        #[cfg(target_arch = "wasm32")]
        &event_loop,
    );

    event_loop.run_app(&mut handler)?;

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    run().unwrap_throw();

    Ok(())
}
