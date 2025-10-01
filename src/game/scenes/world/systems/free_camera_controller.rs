use glam::{Quat, Vec2, Vec3};
use winit::{event::MouseButton, keyboard::KeyCode};

use crate::{
    engine::input::InputState,
    game::{camera::Camera, scenes::world::systems::camera_system::CameraController},
};

pub struct FreeCameraControls {
    mouse_button: MouseButton,

    forward: KeyCode,
    back: KeyCode,
    right: KeyCode,
    left: KeyCode,
    up: KeyCode,
    down: KeyCode,

    accelerate: KeyCode,
}

impl Default for FreeCameraControls {
    fn default() -> Self {
        Self {
            mouse_button: MouseButton::Right,

            forward: KeyCode::KeyW,
            back: KeyCode::KeyS,
            right: KeyCode::KeyD,
            left: KeyCode::KeyA,
            up: KeyCode::KeyE,
            down: KeyCode::KeyQ,

            accelerate: KeyCode::ShiftLeft,
        }
    }
}

/// Input state that can be changed by the user.
#[derive(Default)]
struct Input {
    /// Directional input made by the user.
    direction: Vec3,
    /// Whether the movement accelerator is held down.
    accelerator: bool,
    /// Amount of mouse delta input.
    mouse_delta: Vec2,
}

pub struct FreeCameraController {
    /// User input this frame.
    input: Input,
    /// Current yaw angle of the camera in *degrees*.
    pub yaw: f32,
    /// Current pitch angle of the camera in *degrees*.
    pub pitch: f32,
    /// The speed at which movements will be calculated.
    pub movement_speed: f32,
    /// The rotation sensitivity of mouse movement.
    pub mouse_sensitivity: f32,
    /// Controls used to manipulate the camera.
    pub controls: FreeCameraControls,
}

impl FreeCameraController {
    pub fn new(movement_speed: f32, mouse_sensitivity: f32) -> Self {
        Self {
            input: Input::default(),
            yaw: 0.0,
            pitch: 0.0,
            movement_speed,
            mouse_sensitivity,
            controls: FreeCameraControls::default(),
        }
    }
}

impl CameraController for FreeCameraController {
    fn handle_input(&mut self, input_state: &InputState) {
        let mut input = Input::default();

        input.direction = {
            let mut direction = Vec3::ZERO;

            if input_state.key_pressed(self.controls.forward) {
                direction += Camera::FORWARD;
            }
            if input_state.key_pressed(self.controls.back) {
                direction -= Camera::FORWARD
            }

            if input_state.key_pressed(self.controls.right) {
                direction += Camera::RIGHT;
            }
            if input_state.key_pressed(self.controls.left) {
                direction -= Camera::RIGHT;
            }

            if input_state.key_pressed(self.controls.up) {
                direction += Camera::UP;
            }
            if input_state.key_pressed(self.controls.down) {
                direction -= Camera::UP;
            }

            direction
        };

        input.accelerator = input_state.key_pressed(self.controls.accelerate);

        if input_state.mouse_pressed(self.controls.mouse_button) {
            if let Some(delta) = input_state.mouse_delta() {
                input.mouse_delta = delta;
            }
        }

        self.input = input;
    }

    fn update(&mut self, camera: &mut Camera, delta_time: f32) {
        // Set the new rotation.
        self.yaw += self.input.mouse_delta.x * self.mouse_sensitivity;
        self.pitch -= self.input.mouse_delta.y * self.mouse_sensitivity;

        let rotation = Quat::from_rotation_z(self.yaw.to_radians())
            * Quat::from_rotation_x(self.pitch.to_radians());

        // Translate the direction to the new forward direction.
        let position = rotation * self.input.direction;

        camera.rotation = rotation;
        camera.position += position * delta_time;
    }
}
