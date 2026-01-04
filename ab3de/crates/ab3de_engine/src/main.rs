fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    ab3de_engine::run_native_winit().unwrap();
}
