use glam::{Mat4, Quat, Vec3};

/// A translation and rotation that can be converted into a 4x4 matrix.
#[derive(Clone, Debug, Default)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
}

#[allow(unused)]
impl Transform {
    pub fn new(translation: Vec3, rotation: Quat) -> Self {
        Self {
            translation,
            rotation,
        }
    }

    /// Create a new transform from a translation.
    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            rotation: Quat::IDENTITY,
        }
    }

    pub fn from_rotation(rotation: Quat) -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation,
        }
    }

    /// Create a new transform from euler angles as a rotation.
    pub fn from_euler_rotation(rotation: Vec3) -> Self {
        Self::from_rotation(Quat::from_euler(
            glam::EulerRot::XYZ,
            rotation.x,
            rotation.y,
            rotation.z,
        ))
    }

    pub fn with_translation(mut self, translation: Vec3) -> Self {
        self.translation = translation;
        self
    }

    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_euler_rotation(mut self, rotation: Vec3) -> Self {
        self.rotation = Quat::from_euler(glam::EulerRot::XYZ, rotation.x, rotation.y, rotation.z);
        self
    }

    pub fn to_mat4(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.rotation, self.translation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let transform = Transform::default().to_mat4();
        assert_eq!(transform, Mat4::IDENTITY);

        let transform = Transform::default()
            .with_rotation(Quat::from_xyzw(0.0, 0.0, 0.0, 1.0))
            .to_mat4();
        assert_eq!(transform, Mat4::IDENTITY);

        let transform = Transform::default()
            .with_translation(Vec3::new(10.0, 8.0, 6.0))
            .to_mat4();

        let transform = transform * Transform::default().to_mat4();

        assert_eq!(transform, Mat4::from_translation(Vec3::new(10.0, 8.0, 6.0)));
    }
}
