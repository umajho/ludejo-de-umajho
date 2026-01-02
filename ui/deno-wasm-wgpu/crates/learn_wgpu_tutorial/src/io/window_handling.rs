pub mod native_winit;

pub trait SimpleApplicationEventHandler {
    #[must_use]
    fn handle_input(&mut self, input: Input) -> bool;

    /// corresponds to [`winit::event::WindowEvent::Resized`].
    fn handle_resized(&mut self, size: (u32, u32));

    /// corresponds to [`winit::event::WindowEvent::RedrawRequested`].
    fn handle_redraw_requested(&mut self, get_window_size: Option<Box<dyn FnOnce() -> (u32, u32)>>);
}

pub enum Input {
    /// corresponds to [`winit::event::DeviceEvent::MouseMotion`].
    MouseMotion { delta: (f64, f64) },
    /// corresponds to [`winit::event::WindowEvent::KeyboardInput`].
    KeyboardInput {
        physical_key: PhysicalKey,
        state: ElementState,
    },
    /// corresponds to [`winit::event::WindowEvent::MouseWheel`].
    MouseWheel { delta: MouseScrollDelta },
    /// corresponds to [`winit::event::WindowEvent::MouseInput`].
    MouseInput {
        button: MouseButton,
        state: ElementState,
    },
}

/// corresponds to [`winit::keyboard::PhysicalKey`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalKey {
    Code(KeyCode),
    Other,
}

/// corresponds to [`winit::keyboard::KeyCode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    KeyW,
    KeyS,
    KeyA,
    KeyD,
    Space,
    ShiftLeft,
    Other,
}

/// corresponds to [`winit::event::ElementState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementState {
    Pressed,
    Released,
}

/// corresponds to [`winit::event::MouseScrollDelta`].
pub enum MouseScrollDelta {
    LineDelta(f32, f32),
    PixelDelta((f64, f64)),
}

/// corresponds to [`winit::event::MouseButton`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Other,
}
