use glam::{Quat, Vec3};

use crate::game::track::Track;

use super::skeleton::Skeleton;

/// Holds key frames for a skeleton. Vec indices correspond to [BoneIndex] in the skeleton.
pub struct Animation {
    pub translations: Vec<Track<Vec3>>,
    pub rotations: Vec<Track<Quat>>,
}

impl Animation {
    /// Create an [Animation] with the bone structure of the given [Skeleton] and empty tracks.
    pub fn from_skeleton(skeleton: &Skeleton) -> Self {
        Self {
            translations: vec![Track::default(); skeleton.bones.len()],
            rotations: vec![Track::default(); skeleton.bones.len()],
        }
    }
}
