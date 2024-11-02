use crate::engine::renderer::Renderer;

use glam::{Mat4, Quat, Vec3};

#[derive(Clone, Copy, Default, bytemuck::NoUninit)]
#[repr(C)]
pub struct Matrices {
    pub projection: [f32; 16],
    pub view: [f32; 16],
}

#[derive(Default)]
pub struct Camera {
    pub position: Vec3,
    pub rotation: Quat,
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
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

    pub fn forward_vector(&self) -> Vec3 {
        self.rotation * Vec3::Z
    }

    pub fn right_vector(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    pub fn up_vector(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }

    pub fn calculate_matrices(&self) -> Matrices {
        let projection = Mat4::perspective_lh(self.fov, self.aspect_ratio, self.near, self.far);

        let rotation_matrix = Mat4::from_quat(self.rotation).transpose();
        let translation_matrix = Mat4::from_translation(-self.position);
        let view = rotation_matrix * translation_matrix;

        Matrices {
            projection: projection.to_cols_array(),
            view: view.to_cols_array(),
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
