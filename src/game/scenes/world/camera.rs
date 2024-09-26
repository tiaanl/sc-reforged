use cgmath::{Deg, InnerSpace, Matrix4, One, Point3, Quaternion, Rad, Rotation3, Vector3};

pub struct Camera {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
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
    pub fn from_position_rotation(position: Vector3<f32>, rotation: Quaternion<f32>) -> Self {
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
        let projection = cgmath::perspective(Deg(45.0), self.aspect, self.near, self.far);

        let rotation = cgmath::Matrix4::from(self.rotation);
        // Translation is inverted, because we're moving the world, not the camera.
        let translation = cgmath::Matrix4::from_translation(-self.position);
        let view = translation * rotation;

        Matrices {
            projection: projection.into(),
            view: view.into(),
        }
    }

    pub fn look_at(&mut self, target: Vector3<f32>) {
        let world_up = Vector3::unit_y();
        let world_forward = Vector3::unit_z(); // +Z goes into the screen.

        let forward = (target - self.position).normalize();

        let rotation_axis = world_forward.cross(forward).normalize();
        let dot_product = world_forward.dot(forward).clamp(-1.0, 1.0);

        let rotation_angle = Rad(dot_product.acos());

        self.rotation = if dot_product < -0.9999 {
            // If looking in exactly the opposite direction, rotate 180 degrees around the "up" vector
            Quaternion::from_axis_angle(world_up, Rad(std::f32::consts::PI))
        } else if dot_product > 0.9999 {
            // No rotation needed if eye is already facing target
            Quaternion::one()
        } else {
            // Regular rotation quaternion
            Quaternion::from_axis_angle(rotation_axis, rotation_angle)
        };
    }
}
