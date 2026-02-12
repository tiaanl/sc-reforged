use bevy_ecs::prelude::*;
use glam::{Mat4, Vec3};

use crate::{engine::transform::Transform, game::skeleton::Skeleton};

use super::motion::Motion;

#[derive(Clone, Component, Debug, Default)]
pub struct Pose {
    pub bones: Vec<Mat4>,
}

/// Generate a model-space pose for `motion` at `time`.
///
/// When `root_translation_override` is provided, bone id `1` (COG/root) is
/// pinned to that translation to mirror state-driven COG stabilization from the
/// original animation controller.
pub fn generate_pose(
    skeleton: &Skeleton,
    motion: &Motion,
    time: f32,
    looping: bool,
    root_translation_override: Option<Vec3>,
) -> Pose {
    let mut bones: Vec<Mat4> = Vec::with_capacity(skeleton.bones.len());

    for bone in &skeleton.bones {
        // Get the parent transform.
        let parent_transform = if bone.parent == u32::MAX {
            Mat4::IDENTITY
        } else {
            bones[bone.parent as usize]
        };

        let translation_sample = motion.sample_bone_translation(bone.id, time, looping);
        let mut translation = bone.transform.translation;
        if let Some(sample) = translation_sample
            && sample.has_channel
        {
            translation = sample.value;
        }
        // The original controller can pin the COG/root bone to a state
        // default when posture sequences are active.
        if bone.id == 1
            && translation_sample.is_none_or(|sample| !sample.has_channel)
            && let Some(override_translation) = root_translation_override
        {
            translation = override_translation;
        }
        let rotation = motion
            .sample_bone_rotation(bone.id, time, looping)
            .map(|sample| sample.value)
            .unwrap_or(bone.transform.rotation);

        let local = Transform::new(translation, rotation).to_mat4();

        bones.push(parent_transform * local);
    }

    Pose { bones }
}
