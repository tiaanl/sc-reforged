use glam::{Mat4, Quat, Vec3};

pub struct Camera {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    aspect: f32,
    near: f32,
    far: f32,
}

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
pub struct Matrices {
    pub projection: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
}

impl Camera {
    pub fn from_position_rotation(position: Vec3, rotation: Quat) -> Self {
        Self {
            position,
            rotation,
            aspect: 1.0,
            near: 0.1,
            far: 1000.0,
        }
    }

    /// Adjust the aspect ratio of the camera view plane.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height.max(1) as f32;
    }

    /// Create and returns the projection and view matrices based on the position and rotation of the camera.
    pub fn create_matrices(&self) -> Matrices {
        let projection =
            glam::Mat4::perspective_lh(45.0_f32.to_radians(), self.aspect, self.near, self.far);

        let rotation = Mat4::from_quat(self.rotation);
        // Translation is inverted, because we're moving the world, not the camera.
        let translation = Mat4::from_translation(-self.position);
        let view = translation * rotation;

        Matrices {
            projection: projection.to_cols_array_2d(),
            view: view.to_cols_array_2d(),
        }
    }

    pub fn look_at(&mut self, target: Vec3) {
        let world_up = Vec3::Y;
        let world_forward = Vec3::Z; // +Z goes into the screen.

        let forward = (target - self.position).normalize();

        let rotation_axis = world_forward.cross(forward).normalize();
        let dot_product = world_forward.dot(forward).clamp(-1.0, 1.0);

        let rotation_angle = dot_product.acos().to_radians();

        self.rotation = if dot_product < -0.9999 {
            // If looking in exactly the opposite direction, rotate 180 degrees around the "up" vector
            Quat::from_axis_angle(world_up, std::f32::consts::PI)
        } else if dot_product > 0.9999 {
            // No rotation needed if eye is already facing target
            Quat::IDENTITY
        } else {
            // Regular rotation quaternion.
            Quat::from_axis_angle(rotation_axis, rotation_angle)
        };
    }
}
