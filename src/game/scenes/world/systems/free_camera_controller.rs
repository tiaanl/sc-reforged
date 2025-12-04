use glam::{Quat, Vec3};
use winit::{event::MouseButton, keyboard::KeyCode};

use crate::{
    engine::input::InputState,
    game::scenes::world::{sim_world::Camera, systems::camera_system::CameraController},
};

pub struct FreeCameraControls {
    mouse_button: MouseButton,

    forward: KeyCode,
    back: KeyCode,
    right: KeyCode,
    left: KeyCode,
    up: KeyCode,
    down: KeyCode,
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
        }
    }
}

pub struct FreeCameraController {
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
            yaw: 0.0,
            pitch: 0.0,
            movement_speed,
            mouse_sensitivity,
            controls: FreeCameraControls::default(),
        }
    }
}

impl CameraController for FreeCameraController {
    fn handle_input(
        &mut self,
        target_camera: &mut Camera,
        input_state: &InputState,
        delta_time: f32,
    ) {
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

        {
            let delta = input_state.wheel_delta();
            if delta < 0.0 {
                self.movement_speed *= 0.9;
            } else if delta > 0.0 {
                self.movement_speed *= 1.1;
            }
        }

        if input_state.mouse_pressed(self.controls.mouse_button) {
            if let Some(delta) = input_state.mouse_delta() {
                let delta = delta.as_vec2();
                self.yaw -= delta.x * self.mouse_sensitivity;
                self.pitch += delta.y * self.mouse_sensitivity;
            }
        }

        let rotation = Quat::from_rotation_z(self.yaw.to_radians())
            * Quat::from_rotation_x(self.pitch.to_radians());

        // Translate in the direction to the new forward direction.
        let position_delta = rotation * direction * self.movement_speed;

        target_camera.rotation = rotation;
        target_camera.position += position_delta * delta_time;
    }
}
