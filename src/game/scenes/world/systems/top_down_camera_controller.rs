use glam::Vec3;
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
            rotate_mouse_button: MouseButton::Right,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct CameraData {
    /// The position of the camera.
    position: Vec3,
    /// Current yaw angle of the camera in *degrees*.
    pub yaw: f32,
    /// Current pitch angle of the camera in *degrees*.
    pub pitch: f32,
}

// impl CameraData {
//     #[inline]
//     fn rotation(&self) -> Quat {
//         Quat::from_rotation_z(self.yaw.to_radians())
//             * Quat::from_rotation_x(self.pitch.to_radians())
//     }
// }

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
}

pub struct TopDownCameraController {
    /// The input the player added this frame.
    input: Input,
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
    pub fn new(movement_speed: f32, rotation_speed: f32) -> Self {
        Self {
            input: Input::default(),
            movement_speed,
            rotation_speed,
            desired: Default::default(),
            current: Default::default(),
            controls: Default::default(),
        }
    }

    // pub fn move_forward(&mut self, distance: f32) {
    //     self.desired.position +=
    //         Quat::from_rotation_z(self.current.yaw.to_radians()) * Camera::FORWARD * distance;
    // }

    // pub fn move_right(&mut self, distance: f32) {
    //     self.desired.position +=
    //         Quat::from_rotation_z(self.current.yaw.to_radians()) * Camera::RIGHT * distance;
    // }

    // pub fn move_up(&mut self, distance: f32) {
    //     self.desired.position += Camera::UP * distance;
    // }
}

impl CameraController for TopDownCameraController {
    fn handle_input(&mut self, input_state: &InputState) {
        self.input.move_direction = {
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

        /*
        let move_delta = if input_state.key_pressed(KeyCode::ShiftLeft)
            || input_state.key_pressed(KeyCode::ShiftRight)
        {
            self.movement_speed * 5.0
        } else {
            self.movement_speed
        } * delta_time;

        if input_state.key_pressed(self.controls.forward) {
            self.move_forward(move_delta);
        }
        if input_state.key_pressed(self.controls.backward) {
            self.move_forward(-move_delta);
        }
        if input_state.key_pressed(self.controls.right) {
            self.move_right(move_delta);
        }
        if input_state.key_pressed(self.controls.left) {
            self.move_right(-move_delta);
        }
        if input_state.key_pressed(self.controls.up) {
            self.move_up(move_delta);
        }
        if input_state.key_pressed(self.controls.down) {
            self.move_up(-move_delta);
        }
        if input_state.key_pressed(self.controls.look_up) {
            self.desired.pitch += delta_time * 360.0_f32.to_radians() * 10.0;
        }
        if input_state.key_pressed(self.controls.look_down) {
            self.desired.pitch -= delta_time * 360.0_f32.to_radians() * 10.0;
        }

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
    }

    fn update(&mut self, camera: &mut Camera, delta_time: f32) {
        self.desired.position += self.input.move_direction * self.movement_speed * delta_time;

        self.current = Interpolate::interpolate(self.current, self.desired, 0.1 * delta_time);

        camera.position = self.current.position;
    }
}
