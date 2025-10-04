use glam::{Quat, Vec3};
use winit::{event::MouseButton, keyboard::KeyCode};

use crate::{
    engine::prelude::InputState,
    game::{
        animations::Interpolate, camera::Camera,
        scenes::world::systems::camera_system::CameraController,
    },
};

pub struct TopDownCameraControls {
    pub forward: KeyCode,
    pub backward: KeyCode,
    pub right: KeyCode,
    pub left: KeyCode,
    pub up: KeyCode,
    pub down: KeyCode,
    pub look_up: KeyCode,
    pub look_down: KeyCode,
    pub look_left: KeyCode,
    pub look_right: KeyCode,
    pub rotate_mouse_button: MouseButton,
}

impl Default for TopDownCameraControls {
    fn default() -> Self {
        Self {
            forward: KeyCode::KeyW,
            backward: KeyCode::KeyS,
            right: KeyCode::KeyD,
            left: KeyCode::KeyA,
            up: KeyCode::PageUp,
            down: KeyCode::PageDown,
            look_up: KeyCode::Home,
            look_down: KeyCode::End,
            look_left: KeyCode::KeyQ,
            look_right: KeyCode::KeyE,
            rotate_mouse_button: MouseButton::Right,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct CameraData {
    /// The position of the camera.
    position: Vec3,
    /// Current yaw angle of the camera in *degrees*.
    pub yaw: f32,
    /// Current pitch angle of the camera in *degrees*.
    pub pitch: f32,
}

impl Interpolate for CameraData {
    fn interpolate(left: Self, right: Self, n: f32) -> Self {
        Self {
            position: Interpolate::interpolate(left.position, right.position, n),
            yaw: Interpolate::interpolate(left.yaw, right.yaw, n),
            pitch: Interpolate::interpolate(left.pitch, right.pitch, n),
        }
    }
}

#[derive(Default)]
struct Input {
    /// Direction the player wants to move the camera, relative to the current forward direction.
    move_direction: Vec3,
    /// Direction the player wants to set the pitch of the camera.
    pitch_direction: f32,
    /// Direction the player wants to set the yaw of the camera.
    yaw_direction: f32,
}

pub struct TopDownCameraController {
    /// The speed at which movements will be calculated.
    movement_speed: f32,
    /// The speed at which rotations will be calculated.
    rotation_speed: f32,
    /// The desired data for the camera which will be interpolated each frame.
    desired: CameraData,
    /// The current data for the camera this frame.
    current: CameraData,
    /// Controls used for the camera.
    controls: TopDownCameraControls,
}

impl TopDownCameraController {
    pub fn new(
        position: Vec3,
        yaw: f32,
        pitch: f32,
        movement_speed: f32,
        rotation_speed: f32,
    ) -> Self {
        let initial = CameraData {
            position,
            yaw,
            pitch,
        };

        Self {
            movement_speed,
            rotation_speed,
            desired: initial,
            current: initial,
            controls: Default::default(),
        }
    }

    fn gather_input(&self, input_state: &InputState) -> Input {
        let mut input = Input::default();

        input.move_direction = {
            let mut move_direction = Vec3::ZERO;

            if input_state.key_pressed(self.controls.forward) {
                move_direction += Camera::FORWARD;
            }
            if input_state.key_pressed(self.controls.backward) {
                move_direction -= Camera::FORWARD;
            }

            if input_state.key_pressed(self.controls.right) {
                move_direction += Camera::RIGHT;
            }
            if input_state.key_pressed(self.controls.left) {
                move_direction -= Camera::RIGHT;
            }

            if input_state.key_pressed(self.controls.up) {
                move_direction += Camera::UP;
            }
            if input_state.key_pressed(self.controls.down) {
                move_direction -= Camera::UP;
            }

            move_direction
        };

        input.pitch_direction = {
            let mut pitch_direction = 0.0;

            if input_state.key_pressed(self.controls.look_up) {
                pitch_direction += 1.0;
            }
            if input_state.key_pressed(self.controls.look_down) {
                pitch_direction -= 1.0;
            }

            pitch_direction
        };

        input.yaw_direction = {
            let mut yaw_direction = 0.0;

            if input_state.key_pressed(self.controls.look_left) {
                yaw_direction -= 1.0;
            }
            if input_state.key_pressed(self.controls.look_right) {
                yaw_direction += 1.0;
            }

            yaw_direction
        };

        /*
        if input_state.mouse_pressed(self.controls.rotate_mouse_button) {
            if let Some(delta) = input_state.mouse_delta() {
                let delta = delta * self.rotation_speed;
                if delta.x != 0.0 {
                    self.desired.yaw += delta.x;
                }
                if delta.y != 0.0 {
                    self.desired.position.z -= delta.y * move_delta;
                }
            }
        }

        if input_state.wheel_delta() != 0.0 {
            self.desired.position.z += input_state.wheel_delta() * move_delta * 3.0;
        }
        */

        input
    }

    fn update_camera(&mut self, move_rotation: Quat, camera: &mut Camera) {
        camera.position = self.current.position;
        camera.rotation = move_rotation * Quat::from_rotation_x(self.current.pitch.to_radians());
    }
}

impl CameraController for TopDownCameraController {
    fn handle_input(
        &mut self,
        target_camera: &mut Camera,
        input_state: &InputState,
        delta_time: f32,
    ) {
        let input = self.gather_input(input_state);

        let move_rotation = Quat::from_rotation_z(self.current.yaw.to_radians());

        self.desired.position +=
            (move_rotation * input.move_direction) * self.movement_speed * delta_time;

        self.desired.pitch += input.pitch_direction * self.rotation_speed * delta_time;
        self.desired.yaw += input.yaw_direction * self.rotation_speed * delta_time;

        self.current = Interpolate::interpolate(self.current, self.desired, 0.1);

        self.update_camera(move_rotation, target_camera);
    }
}
