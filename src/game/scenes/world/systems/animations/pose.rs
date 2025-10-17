use glam::{Quat, Vec3};

pub struct PoseBone {
    pub translation: Vec3,
    pub rotation: Quat,
}

pub struct Pose {
    pub bones: Vec<PoseBone>,
}
