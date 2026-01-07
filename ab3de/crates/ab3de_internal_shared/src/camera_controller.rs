use std::f32::consts::FRAC_PI_2;

use ab3de_engine::CameraData;

use crate::inputting::{ElementState, KeyCode, MouseButton, MouseScrollDelta, PhysicalKey};

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

#[derive(Debug, Clone)]
pub struct CameraController {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    scroll: f32,
    speed: f32,
    sensitivity: f32,

    mouse_pressed: bool,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            scroll: 0.0,
            speed,
            sensitivity,
            mouse_pressed: false,
        }
    }

    pub fn default() -> Self {
        Self::new(4.0, 0.4)
    }

    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        let amount = if state == ElementState::Pressed {
            1.0
        } else {
            0.0
        };
        match key {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.amount_forward = amount;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.amount_backward = amount;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.amount_left = amount;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.amount_right = amount;
                true
            }
            KeyCode::Space => {
                self.amount_up = amount;
                true
            }
            KeyCode::ShiftLeft => {
                self.amount_down = amount;
                true
            }
            _ => false,
        }
    }

    pub fn handle_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        if self.mouse_pressed {
            self.rotate_horizontal = mouse_dx as f32;
            self.rotate_vertical = mouse_dy as f32;
        }
    }

    pub fn handle_mouse_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = -match delta {
            // I'm assuming a line is about 100 pixels
            MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
            MouseScrollDelta::PixelDelta((_, scroll)) => *scroll as f32,
        };
    }

    pub fn handle_mouse_input(&mut self, button: MouseButton, state: ElementState) {
        if button == MouseButton::Left {
            self.mouse_pressed = state == ElementState::Pressed;
        }
    }

    pub fn update_camera(&mut self, camera_data: &mut CameraData, dt_s: f32) {
        // Move forward/backward and left/right
        let (yaw_sin, yaw_cos) = camera_data.yaw_radians.sin_cos();
        let forward = glam::vec3(yaw_cos, 0.0, yaw_sin).normalize();
        let right = glam::vec3(-yaw_sin, 0.0, yaw_cos).normalize();
        camera_data.position +=
            forward * (self.amount_forward - self.amount_backward) * self.speed * dt_s;
        camera_data.position += right * (self.amount_right - self.amount_left) * self.speed * dt_s;

        // Move in/out (aka. "zoom")
        // Note: this isn't an actual zoom. The camera's position
        // changes when zooming. I've added this to make it easier
        // to get closer to an object you want to focus on.
        let (pitch_sin, pitch_cos) = camera_data.pitch_radians.sin_cos();
        let scrollward =
            glam::vec3(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
        camera_data.position += scrollward * self.scroll * self.speed * self.sensitivity * dt_s;
        self.scroll = 0.0;

        // Move up/down. Since we don't use roll, we can just
        // modify the y coordinate directly.
        camera_data.position.y += (self.amount_up - self.amount_down) * self.speed * dt_s;

        // Rotate
        camera_data.yaw_radians += self.rotate_horizontal * self.sensitivity * dt_s;
        camera_data.pitch_radians += -self.rotate_vertical * self.sensitivity * dt_s;
        // If process_mouse isn't called every frame, these values
        // will not get set to zero, and the camera will rotate
        // when moving in a non-cardinal direction.
        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;

        // Keep the camera's angle from going too high/low.
        if camera_data.pitch_radians < -SAFE_FRAC_PI_2 {
            camera_data.pitch_radians = -SAFE_FRAC_PI_2;
        } else if camera_data.pitch_radians > SAFE_FRAC_PI_2 {
            camera_data.pitch_radians = SAFE_FRAC_PI_2;
        }
    }
}

pub enum CameraControllerInput {
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

impl CameraController {
    pub fn handle_input(&mut self, input: CameraControllerInput) -> bool {
        match input {
            CameraControllerInput::MouseMotion { delta } => {
                self.handle_mouse(delta.0, delta.1);
                true
            }
            CameraControllerInput::KeyboardInput {
                physical_key: PhysicalKey::Code(key),
                state,
            } => self.process_keyboard(key, state),
            CameraControllerInput::MouseWheel { delta } => {
                self.handle_mouse_scroll(&delta);
                true
            }
            CameraControllerInput::MouseInput { button, state } => {
                self.handle_mouse_input(button, state);
                true
            }
            _ => false,
        }
    }
}
