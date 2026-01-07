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
#[allow(unused)]
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
