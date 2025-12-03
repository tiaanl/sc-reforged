#![allow(dead_code)]

use crate::game::math::ViewProjection;

use glam::{Mat4, Quat, Vec3};

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
    pub const RIGHT: Vec3 = Vec3::NEG_X;
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

    #[inline]
    pub fn calculate_view_projection(&self) -> ViewProjection {
        ViewProjection::from_projection_view(self.calculate_projection(), self.calculate_view())
    }

    pub fn look_at(&mut self, camera_to: Vec3) {
        let forward = (camera_to - self.position).normalize();
        let world_up = Self::UP;
        let right = world_up.cross(forward).normalize();
        let up = forward.cross(right);
        let rotation_matrix = glam::Mat3::from_cols(right, up, forward);
        self.rotation = Quat::from_mat3(&rotation_matrix);
    }

    pub fn view_slice_planes(&self, count: u32, lambda: f32) -> Vec<[Vec3; 4]> {
        fn cascade_view_splits(near: f32, far: f32, count: u32, lambda: f32) -> Vec<f32> {
            assert!(count >= 1 && far > near);
            let mut d = Vec::with_capacity(count as usize + 1);
            d.push(near);
            for i in 1..count {
                let s = i as f32 / count as f32;
                let d_lin = near + (far - near) * s;
                let d_log = near * (far / near).powf(s);
                d.push(d_lin * (1.0 - lambda) + d_log * lambda);
            }
            d.push(far);
            d
        }

        // Distances along camera forward (+Z in your LH view space)
        let inverse_view = self.calculate_view().inverse();
        let right = inverse_view.col(0).truncate();
        let up = inverse_view.col(1).truncate();
        let forward = inverse_view.col(2).truncate();
        let position = inverse_view.col(3).truncate();

        cascade_view_splits(self.near, self.far, count, lambda)
            .iter()
            .map(|distance| {
                let half_height = (self.fov * 0.5).tan() * distance;
                let half_width = half_height * self.aspect_ratio;
                let center = position + forward * distance;

                [
                    center + up * half_height - right * half_width,
                    center + up * half_height + right * half_width,
                    center - up * half_height + right * half_width,
                    center - up * half_height - right * half_width,
                ]
            })
            .collect::<Vec<_>>()
    }

    #[inline]
    pub fn calculate_projection(&self) -> Mat4 {
        Mat4::perspective_lh(self.fov, self.aspect_ratio, self.near, self.far)
    }

    #[inline]
    pub fn calculate_view(&self) -> Mat4 {
        let target = self.position + self.rotation * Self::FORWARD;
        Mat4::look_at_lh(self.position, target, self.rotation * Self::UP)
    }
}
