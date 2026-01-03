fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    learn_wgpu_tutorial::run_native_winit().unwrap();
}
