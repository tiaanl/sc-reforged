use glam::{Quat, Vec3};
use winit::{event::MouseButton, keyboard::KeyCode};

use crate::game::{
    animations::Interpolate,
    camera::Camera,
    scenes::world::{sim_world::SimWorld, systems::System},
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

impl CameraData {
    #[inline]
    fn rotation(&self) -> Quat {
        Quat::from_rotation_z(self.yaw.to_radians())
            * Quat::from_rotation_x(self.pitch.to_radians())
    }
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

pub struct TopDownCameraSystem {
    /// Index of the camera to control.
    camera_index: usize,
    /// The speed at which movements will be calculated.
    movement_speed: f32,
    /// The speed at which rotations will be calculated.
    rotation_speed: f32,
    /// Controls used for the camera.
    controls: TopDownCameraControls,
    /// The desired data for the camera which will be interpolated each frame.
    desired: CameraData,
    /// The current data for the camera this frame.
    current: CameraData,
}

impl TopDownCameraSystem {
    pub fn new(camera_index: usize, movement_speed: f32, rotation_speed: f32) -> Self {
        Self {
            camera_index,
            movement_speed,
            rotation_speed,
            controls: TopDownCameraControls::default(),
            desired: CameraData::default(),
            current: CameraData::default(),
        }
    }

    pub fn move_forward(&mut self, distance: f32) {
        self.desired.position +=
            Quat::from_rotation_z(self.current.yaw.to_radians()) * Camera::FORWARD * distance;
    }

    pub fn move_right(&mut self, distance: f32) {
        self.desired.position +=
            Quat::from_rotation_z(self.current.yaw.to_radians()) * Camera::RIGHT * distance;
    }

    pub fn move_up(&mut self, distance: f32) {
        self.desired.position += Camera::UP * distance;
    }
}

impl System for TopDownCameraSystem {
    fn pre_update(
        &mut self,
        _sim_world: &mut SimWorld,
        time: &super::Time,
        input_state: &crate::engine::prelude::InputState,
    ) {
        let move_delta = if input_state.key_pressed(KeyCode::ShiftLeft)
            || input_state.key_pressed(KeyCode::ShiftRight)
        {
            self.movement_speed * 5.0
        } else {
            self.movement_speed
        } * time.delta_time;

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
            self.desired.pitch += time.delta_time * 360.0_f32.to_radians() * 10.0;
        }
        if input_state.key_pressed(self.controls.look_down) {
            self.desired.pitch -= time.delta_time * 360.0_f32.to_radians() * 10.0;
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
    }

    fn update(&mut self, _sim_world: &mut SimWorld, time: &super::Time) {
        // Interpolate the desired closer to the target.
        self.current = Interpolate::interpolate(self.current, self.desired, 0.1 * time.delta_time);
    }

    fn post_update(&mut self, sim_world: &mut SimWorld) {
        let camera = &mut sim_world.cameras[self.camera_index];

        camera.position = self.current.position;
        camera.rotation = self.current.rotation();
    }

    fn extract(
        &mut self,
        sim_world: &SimWorld,
        render_world: &mut crate::game::scenes::world::render_world::RenderWorld,
    ) {
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

    fn prepare(
        &mut self,
        render_world: &mut crate::game::scenes::world::render_world::RenderWorld,
        renderer: &crate::engine::prelude::Renderer,
    ) {
        let offset = (std::mem::size_of::<Camera>() * self.camera_index) as wgpu::BufferAddress;
        let data = bytemuck::bytes_of(&render_world.cameras[self.camera_index]);
        renderer
            .queue
            .write_buffer(&render_world.camera_buffer, offset, data);
    }
}
