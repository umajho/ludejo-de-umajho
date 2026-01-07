#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Date, js_name = now)]
    pub fn now_js() -> f64;
}

#[cfg(target_arch = "wasm32")]
pub fn now_ms() -> u64 {
    now_js() as u64
}
