use glam::{Quat, Vec3};
use winit::{event::MouseButton, keyboard::KeyCode};

use crate::{
    engine::input::InputState,
    game::{
        interpolate::Interpolate,
        scenes::world::{sim_world::Camera, systems::camera_system::CameraController},
    },
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

#[derive(Clone, Copy, Default)]
struct State {
    /// Current position of the camera.
    position: Vec3,
    /// Current yaw angle of the camera in *degrees*.
    pub yaw: f32,
    /// Current pitch angle of the camera in *degrees*.
    pub pitch: f32,
}

impl Interpolate for State {
    #[inline]
    fn interpolate(left: Self, right: Self, n: f32) -> Self {
        State {
            position: Interpolate::interpolate(left.position, right.position, n),
            yaw: Interpolate::interpolate(left.yaw, right.yaw, n),
            pitch: Interpolate::interpolate(left.pitch, right.pitch, n),
        }
    }
}

pub struct FreeCameraController {
    /// Current state of the camera. This will be interpolated towards the
    /// `target_state`.
    current: State,

    /// Target state of the camera.
    target: State,

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
            current: State::default(),
            target: State::default(),
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
        let direction = {
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
                self.target.yaw -= delta.x * self.mouse_sensitivity;
                self.target.pitch += delta.y * self.mouse_sensitivity;
            }
        }

        let rotation = Quat::from_rotation_z(self.target.yaw.to_radians())
            * Quat::from_rotation_x(self.target.pitch.to_radians());

        // Translate in the direction to the new forward direction.
        self.target.position += rotation * direction * self.movement_speed * delta_time;

        // Interpolate.
        self.current = Interpolate::interpolate(self.current, self.target, 0.2);

        // Set the actual values.
        target_camera.rotation = Quat::from_rotation_z(self.current.yaw.to_radians())
            * Quat::from_rotation_x(self.current.pitch.to_radians());
        target_camera.position = self.current.position;
    }
}
