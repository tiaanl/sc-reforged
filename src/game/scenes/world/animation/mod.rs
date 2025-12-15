#![allow(unused)]

use ahash::HashMap;

use crate::{
    engine::transform::Transform,
    game::{common::skeleton::Skeleton, scenes::world::animation::pose::Pose},
};
use glam::{Mat3, Mat4, Quat, Vec3, Vec4};

pub mod motion;
pub mod pose;

pub type BoneIndex = u32;

pub const BONE_SENTINEL: BoneIndex = BoneIndex::MAX;

pub fn generate_pose(
    skeleton: &Skeleton,
    motion: &motion::Motion,
    time: f32,
    looping: bool,
) -> Pose {
    let mut bones: Vec<Mat4> = Vec::with_capacity(skeleton.bones.len());

    for (bone_index, bone) in skeleton.bones.iter().enumerate() {
        // Get the parent transform.
        let parent_transform = if bone.parent == u32::MAX {
            Mat4::IDENTITY
        } else {
            bones[bone.parent as usize]
        };

        let translation = match motion.translations.get(&bone.id) {
            Some(t) => t.sample_sub_frame(time, looping),
            None => bone.transform.translation,
        };
        let rotation = match motion.rotations.get(&bone.id) {
            Some(t) => t.sample_sub_frame(time, looping),
            None => bone.transform.rotation,
        };

        let local = Transform::new(translation, rotation).to_mat4();

        bones.push(parent_transform * local);
    }

    Pose { bones }
}
