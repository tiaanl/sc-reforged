use bevy_ecs::prelude::*;
use glam::{Mat4, Vec3};
use std::collections::HashMap;

use crate::{engine::transform::Transform, game::skeleton::Skeleton};

use super::motion::Motion;

#[derive(Clone, Component, Debug, Default)]
pub struct Pose {
    pub bones: Vec<Mat4>,
    pub local_transforms: Vec<Transform>,
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
    previous_pose: Option<&Pose>,
) -> Pose {
    generate_pose_impl(
        skeleton,
        motion,
        Some(time),
        looping,
        None,
        root_translation_override,
        previous_pose,
    )
}

/// Generate a model-space pose for `motion` at an exact keyframe index.
///
/// This is used for terminal motion application where the original runtime
/// explicitly applies the configured end-frame keyframe.
pub fn generate_pose_at_key_frame(
    skeleton: &Skeleton,
    motion: &Motion,
    key_frame_index: u32,
    root_translation_override: Option<Vec3>,
    previous_pose: Option<&Pose>,
) -> Pose {
    generate_pose_impl(
        skeleton,
        motion,
        None,
        false,
        Some(key_frame_index),
        root_translation_override,
        previous_pose,
    )
}

fn generate_pose_impl(
    skeleton: &Skeleton,
    motion: &Motion,
    time: Option<f32>,
    looping: bool,
    key_frame_index: Option<u32>,
    root_translation_override: Option<Vec3>,
    previous_pose: Option<&Pose>,
) -> Pose {
    let mut local_transforms = previous_pose
        .filter(|pose| pose.local_transforms.len() == skeleton.bones.len())
        .map(|pose| pose.local_transforms.clone())
        .unwrap_or_else(|| {
            skeleton
                .bones
                .iter()
                .map(|bone| bone.transform.clone())
                .collect()
        });

    let mut bone_indices = HashMap::with_capacity(skeleton.bones.len());
    for (bone_index, bone) in skeleton.bones.iter().enumerate() {
        bone_indices.insert(bone.id, bone_index);
    }

    let sampled_updates = if let Some(key_frame_index) = key_frame_index {
        motion.sample_bone_updates_at_key_frame(key_frame_index)
    } else {
        motion.sample_bone_updates(time.unwrap_or_default(), looping)
    };

    let mut root_translation_updated = false;
    for update in sampled_updates {
        let Some(&bone_index) = bone_indices.get(&update.bone_id) else {
            continue;
        };

        if let Some(rotation) = update.rotation {
            local_transforms[bone_index].rotation = rotation;
        }

        // Match original behavior more closely: translation channels are only
        // applied to the COG/root bone; other bones keep their local offsets.
        if update.bone_id == 1
            && let Some(translation) = update.translation
        {
            local_transforms[bone_index].translation = translation;
            root_translation_updated = true;
        }
    }

    // The original controller can pin the COG/root bone to a state
    // default when posture sequences are active.
    if !root_translation_updated
        && let Some(override_translation) = root_translation_override
        && let Some(&root_index) = bone_indices.get(&1)
    {
        local_transforms[root_index].translation = override_translation;
    }

    let mut bones: Vec<Mat4> = Vec::with_capacity(skeleton.bones.len());
    for (bone_index, bone) in skeleton.bones.iter().enumerate() {
        // Get the parent transform.
        let parent_transform = if bone.parent == u32::MAX {
            Mat4::IDENTITY
        } else {
            bones[bone.parent as usize]
        };

        let local = local_transforms[bone_index].to_mat4();
        bones.push(parent_transform * local);
    }

    Pose {
        bones,
        local_transforms,
    }
}
