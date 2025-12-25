#![feature(cfg_select)]

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use winit::event_loop::EventLoop;

use learn_wgpu_tutorial::App;

pub fn run() -> anyhow::Result<()> {
    cfg_select! {
        target_arch = "wasm32" => {
            console_log::init_with_level(log::Level::Debug).unwrap_throw();
        }
        _ => {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = cfg_select! {
        target_arch = "wasm32" => { App::new(&event_loop) }
        _ => { App::new() }
    };
    event_loop.run_app(&mut app)?;

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    run().unwrap_throw();

    Ok(())
}

fn main() {
    run().unwrap();
}
