use crate::engine::renderer::Renderer;

use glam::{Mat4, Quat, Vec3, Vec4};

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
    const CONVERT: Mat4 = Mat4::from_cols(
        Vec4::new(Vec3::NEG_X.x, Vec3::NEG_X.y, Vec3::NEG_X.z, 0.0),
        Vec4::new(Vec3::Z.x, Vec3::Z.y, Vec3::Z.z, 0.0),
        Vec4::new(Vec3::Y.x, Vec3::Y.y, Vec3::Y.z, 0.0),
        Vec4::new(0.0, 0.0, 0.0, 1.0),
    );

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

    pub fn move_forward(&mut self, distance: f32) {
        let v = self.rotation * Self::FORWARD;
        self.position += v * distance;
    }

    pub fn move_right(&mut self, distance: f32) {
        let v = self.rotation * Self::RIGHT;
        self.position += v * distance;
    }

    pub fn move_up(&mut self, distance: f32) {
        let v = self.rotation * Self::UP;
        self.position += v * distance;
    }

    pub fn calculate_matrices(&self) -> Matrices {
        let projection = Mat4::perspective_lh(self.fov, self.aspect_ratio, self.near, self.far);

        let view = Mat4::from_translation(self.position) * Mat4::from_quat(self.rotation);
        let view = Self::CONVERT * view.inverse();

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
