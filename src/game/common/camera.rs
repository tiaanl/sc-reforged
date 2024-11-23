use crate::engine::{input, renderer::Renderer, shaders::Shaders};

use glam::{Mat4, Quat, Vec3};

pub fn register_camera_shader(shaders: &mut Shaders) {
    shaders.add_module(include_str!("camera.wgsl"), "camera.wgsl");
}

#[derive(Clone, Copy, Default, bytemuck::NoUninit)]
#[repr(C)]
pub struct Matrices {
    pub projection: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
}

#[derive(Debug, Default)]
pub struct Camera {
    pub position: Vec3,
    pub rotation: Quat,
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    pub const FORWARD: Vec3 = Vec3::Y;
    pub const RIGHT: Vec3 = Vec3::X;
    pub const UP: Vec3 = Vec3::Z;

    pub fn new(
        position: Vec3,
        rotation: Quat,
        fov: f32,
        aspect_ratio: f32,
        near: f32,
        far: f32,
    ) -> Self {
        Camera {
            position,
            rotation,
            fov,
            aspect_ratio,
            near,
            far,
        }
    }

    pub fn calculate_matrices(&self) -> Matrices {
        let projection = Mat4::perspective_lh(self.fov, self.aspect_ratio, self.near, self.far);

        let target = self.position + self.rotation * Self::FORWARD;
        let view = Mat4::look_at_lh(self.position, target, self.rotation * Self::UP);

        Matrices {
            projection: projection.to_cols_array_2d(),
            view: view.to_cols_array_2d(),
        }
    }
}

pub struct GpuCamera {
    buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl GpuCamera {
    pub fn new(renderer: &Renderer) -> Self {
        let buffer = renderer.create_uniform_buffer("camera_buffer", Matrices::default());

        let bind_group = renderer.create_uniform_bind_group("camera_bind_group", &buffer);

        Self { buffer, bind_group }
    }

    pub fn upload_matrices(&self, renderer: &Renderer, matrices: Matrices) {
        renderer
            .queue
            .write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[matrices]));
    }
}

pub trait CameraController {
    fn on_input(&mut self, input: &input::InputState, delta_time: f32);
    fn update_camera(&mut self, camera: &mut Camera);
}

#[derive(Default)]
pub struct FreeCameraController {
    pub position: Vec3,
    pub yaw: f32,   // degrees
    pub pitch: f32, // degrees

    pub movement_speed: f32,
    pub mouse_sensitivity: f32,
}

impl FreeCameraController {
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

impl CameraController for FreeCameraController {
    fn on_input(&mut self, input: &input::InputState, delta_time: f32) {
        let delta = delta_time * self.movement_speed;
        if input.key_pressed(input::KeyCode::KeyW) {
            self.move_forward(delta);
        }
        if input.key_pressed(input::KeyCode::KeyS) {
            self.move_forward(-delta);
        }
        if input.key_pressed(input::KeyCode::KeyA) {
            self.move_right(delta);
        }
        if input.key_pressed(input::KeyCode::KeyD) {
            self.move_right(-delta);
        }
        if input.key_pressed(input::KeyCode::KeyE) {
            self.move_up(delta);
        }
        if input.key_pressed(input::KeyCode::KeyQ) {
            self.move_up(-delta);
        }

        if input.mouse_pressed(input::MouseButton::Left) {
            let delta = input.mouse_delta() * self.mouse_sensitivity;
            self.yaw += delta.x;
            self.pitch -= delta.y;
        }
    }

    fn update_camera(&mut self, camera: &mut Camera) {
        camera.position = self.position;
        camera.rotation = self.rotation();
    }
}

#[derive(Default)]
pub struct ArcBacllCameraController {
    pub yaw: f32,   // degrees
    pub pitch: f32, // degrees
    pub distance: f32,

    pub mouse_sensitivity: f32,
}

impl ArcBacllCameraController {
    pub fn position_and_rotation(&self) -> (Vec3, Quat) {
        let rotation = Quat::from_rotation_z(self.yaw.to_radians())
            * Quat::from_rotation_x(self.pitch.to_radians());
        let position = rotation * Vec3::new(0.0, -self.distance, 0.0);

        (position, rotation)
    }
}

impl CameraController for ArcBacllCameraController {
    fn on_input(&mut self, input: &input::InputState, _delta_time: f32) {
        if input.mouse_pressed(input::MouseButton::Left) {
            let delta = input.mouse_delta() * self.mouse_sensitivity;
            self.yaw += delta.x;
            self.pitch -= delta.y;
        }
        let distance = self.distance / 10.0;
        self.distance -= input.wheel_delta() * distance;
    }

    fn update_camera(&mut self, camera: &mut Camera) {
        self.pitch = self.pitch.clamp(-89.0_f32, 89.0_f32);
        self.distance = self.distance.clamp(camera.near, camera.far);

        let (position, rotation) = self.position_and_rotation();

        camera.position = position;
        camera.rotation = rotation;
    }
}
