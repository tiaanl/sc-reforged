use glam::{Quat, Vec3};

pub trait Interpolate: Copy {
    fn interpolate(left: Self, right: Self, n: f32) -> Self;
}

impl Interpolate for f32 {
    #[inline]
    fn interpolate(left: Self, right: Self, n: f32) -> Self {
        left + (right - left) * n
    }
}

impl Interpolate for Vec3 {
    #[inline]
    fn interpolate(left: Self, right: Self, n: f32) -> Self {
        left.lerp(right, n)
    }
}

impl Interpolate for Quat {
    #[inline]
    fn interpolate(left: Self, right: Self, n: f32) -> Self {
        left.slerp(right, n)
    }
}
