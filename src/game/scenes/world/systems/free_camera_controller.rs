use glam::{Quat, Vec3};
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

    #[inline]
    fn rotation(&self) -> Quat {
        Quat::from_rotation_z(self.yaw.to_radians())
            * Quat::from_rotation_x(self.pitch.to_radians())
    }

    pub fn move_forward(&mut self, position: &mut Vec3, distance: f32) {
        *position += self.rotation() * Camera::FORWARD * distance;
    }

    pub fn move_right(&mut self, position: &mut Vec3, distance: f32) {
        *position += self.rotation() * Camera::RIGHT * distance;
    }

    pub fn move_up(&mut self, position: &mut Vec3, distance: f32) {
        *position += self.rotation() * Camera::UP * distance;
    }
}

impl CameraController for FreeCameraController {
    fn handle_input(&mut self, camera: &mut Camera, input_state: &InputState, delta_time: f32) {
        let delta = if input_state.key_pressed(KeyCode::ShiftLeft)
            || input_state.key_pressed(KeyCode::ShiftRight)
        {
            self.movement_speed * 2.0
        } else {
            self.movement_speed
        } * delta_time;

        if input_state.key_pressed(self.controls.forward) {
            self.move_forward(&mut camera.position, delta);
        }
        if input_state.key_pressed(self.controls.back) {
            self.move_forward(&mut camera.position, -delta);
        }
        if input_state.key_pressed(self.controls.right) {
            self.move_right(&mut camera.position, delta);
        }
        if input_state.key_pressed(self.controls.left) {
            self.move_right(&mut camera.position, -delta);
        }
        if input_state.key_pressed(self.controls.up) {
            self.move_up(&mut camera.position, delta);
        }
        if input_state.key_pressed(self.controls.down) {
            self.move_up(&mut camera.position, -delta);
        }

        if input_state.mouse_pressed(self.controls.mouse_button) {
            if let Some(delta) = input_state.mouse_delta() {
                let delta = delta * self.mouse_sensitivity;
                if delta.x != 0.0 || delta.y != 0.0 {
                    self.yaw += delta.x;
                    self.pitch -= delta.y;
                }
            }
        }
    }
}
