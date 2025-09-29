use glam::{Quat, Vec3};
use winit::{event::MouseButton, keyboard::KeyCode};

use crate::{
    engine::input::InputState,
    game::{
        camera::Camera,
        scenes::world::{
            render_world::RenderWorld,
            sim_world::SimWorld,
            systems::{Extract, PostUpdate, Prepare},
        },
    },
};

use super::{PreUpdate, Time};

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

pub struct FreeCameraSystem {
    /// Index of the camera to control.
    camera_index: usize,
    /// World position of the camera.
    pub position: Vec3,
    /// Current yaw angle of the camera in *degrees*.
    pub yaw: f32,
    /// Current pitch angle of the camera in *degrees*.
    pub pitch: f32,
    /// The speed at which movements will be calculated.
    pub movement_speed: f32,
    /// The rotation sensitivity of mouse movement.
    pub mouse_sensitivity: f32,
    /// The controls used for movement & rotation.
    controls: FreeCameraControls,
}

impl FreeCameraSystem {
    pub fn new(camera_index: usize, movement_speed: f32, mouse_sensitivity: f32) -> Self {
        Self {
            camera_index,
            position: Vec3::ZERO,
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

    pub fn move_forward(&mut self, distance: f32) {
        self.position += self.rotation() * Camera::FORWARD * distance;
    }

    pub fn move_right(&mut self, distance: f32) {
        self.position += self.rotation() * Camera::RIGHT * distance;
    }

    pub fn move_up(&mut self, distance: f32) {
        self.position += self.rotation() * Camera::UP * distance;
    }
}

impl PreUpdate for FreeCameraSystem {
    fn pre_update(&mut self, _sim_world: &mut SimWorld, time: &Time, input_state: &InputState) {
        let delta = if input_state.key_pressed(KeyCode::ShiftLeft)
            || input_state.key_pressed(KeyCode::ShiftRight)
        {
            self.movement_speed * 2.0
        } else {
            self.movement_speed
        } * time.delta_time;

        if input_state.key_pressed(self.controls.forward) {
            self.move_forward(delta);
        }
        if input_state.key_pressed(self.controls.back) {
            self.move_forward(-delta);
        }
        if input_state.key_pressed(self.controls.right) {
            self.move_right(delta);
        }
        if input_state.key_pressed(self.controls.left) {
            self.move_right(-delta);
        }
        if input_state.key_pressed(self.controls.up) {
            self.move_up(delta);
        }
        if input_state.key_pressed(self.controls.down) {
            self.move_up(-delta);
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

impl PostUpdate for FreeCameraSystem {
    fn post_update(&mut self, sim_world: &mut SimWorld) {
        let camera = &mut sim_world.cameras[self.camera_index];

        camera.position = self.position;
        camera.rotation = self.rotation();
    }
}

impl Extract for FreeCameraSystem {
    fn extract(&mut self, sim_world: &SimWorld, render_world: &mut RenderWorld) {
        let source = &sim_world.cameras[self.camera_index];

        let view_projection = source.calculate_view_projection();
        let frustum = view_projection.frustum();
        let forward = view_projection
            .mat
            .project_point3(Camera::FORWARD)
            .normalize();

        let target = &mut render_world.cameras[self.camera_index];
        target.proj_view = view_projection.mat.to_cols_array_2d();
        target.frustum = frustum
            .planes
            .map(|plane| plane.normal.extend(plane.distance).to_array());
        target.position = source.position.extend(1.0).to_array();
        target.forward = forward.extend(0.0).to_array();
    }
}

impl Prepare for FreeCameraSystem {
    fn prepare(
        &mut self,
        render_world: &mut RenderWorld,
        renderer: &crate::engine::prelude::Renderer,
    ) {
        let offset = (std::mem::size_of::<Camera>() * self.camera_index) as wgpu::BufferAddress;
        let data = bytemuck::bytes_of(&render_world.cameras[self.camera_index]);
        renderer
            .queue
            .write_buffer(&render_world.camera_buffer, offset, data);
    }
}
