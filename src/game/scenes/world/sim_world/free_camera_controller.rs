use bevy_ecs::prelude::*;
use glam::{Quat, Vec3};
use winit::{event::MouseButton, keyboard::KeyCode};

use crate::{
    engine::input::InputState,
    game::{
        interpolate::Interpolate,
        scenes::world::{
            sim_world::{Camera, ecs::ActiveCamera},
            systems::Time,
        },
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

#[derive(Component)]
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

pub fn input(
    mut cameras: Query<(&mut Camera, &mut FreeCameraController), With<ActiveCamera>>,
    input_state: Res<InputState>,
    time: Res<Time>,
) {
    for (mut camera, mut controller) in cameras.iter_mut() {
        let direction = {
            let mut direction = Vec3::ZERO;

            if input_state.key_pressed(controller.controls.forward) {
                direction += Camera::FORWARD;
            }
            if input_state.key_pressed(controller.controls.back) {
                direction -= Camera::FORWARD
            }

            if input_state.key_pressed(controller.controls.right) {
                direction += Camera::RIGHT;
            }
            if input_state.key_pressed(controller.controls.left) {
                direction -= Camera::RIGHT;
            }

            if input_state.key_pressed(controller.controls.up) {
                direction += Camera::UP;
            }
            if input_state.key_pressed(controller.controls.down) {
                direction -= Camera::UP;
            }

            direction
        };

        {
            let delta = input_state.wheel_delta();
            if delta < 0.0 {
                controller.movement_speed *= 0.9;
            } else if delta > 0.0 {
                controller.movement_speed *= 1.1;
            }
        }

        if input_state.mouse_pressed(controller.controls.mouse_button)
            && let Some(delta) = input_state.mouse_delta()
        {
            let delta = delta.as_vec2();
            controller.target.yaw -= delta.x * controller.mouse_sensitivity;
            controller.target.pitch += delta.y * controller.mouse_sensitivity;
        }

        let rotation = Quat::from_rotation_z(controller.target.yaw.to_radians())
            * Quat::from_rotation_x(controller.target.pitch.to_radians());

        // Translate in the direction to the new forward direction.
        let movement_speed = controller.movement_speed;
        controller.target.position += rotation * direction * movement_speed * time.delta_time;

        // Interpolate.
        controller.current = Interpolate::interpolate(controller.current, controller.target, 0.2);

        // Set the actual values.
        camera.rotation = Quat::from_rotation_z(controller.current.yaw.to_radians())
            * Quat::from_rotation_x(controller.current.pitch.to_radians());
        camera.position = controller.current.position;
    }
}
