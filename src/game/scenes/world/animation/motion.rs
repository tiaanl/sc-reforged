use ahash::{HashMap, HashMapExt};
use glam::{Quat, Vec3};

use crate::game::{skeleton::Skeleton, track::Track};

/// Holds key frames for a skeleton. Vec indices correspond to [BoneIndex] in the skeleton.
#[derive(Default)]
pub struct Motion {
    pub bone_ids: Vec<super::BoneIndex>,
    pub translations: HashMap<u32, Track<Vec3>>,
    pub rotations: HashMap<u32, Track<Quat>>,
}

impl Motion {
    /// Create an [Animation] with the bone structure of the given [Skeleton] and empty tracks.
    pub fn from_skeleton(skeleton: &Skeleton) -> Self {
        Self {
            bone_ids: (0..skeleton.bones.len() as super::BoneIndex).collect(),
            translations: HashMap::with_capacity(skeleton.bones.len()),
            rotations: HashMap::with_capacity(skeleton.bones.len()),
        }
    }

    pub fn max_frame_num(&self) -> u32 {
        println!("translations: {}", self.translations.len());

        let t_max = self
            .translations
            .values()
            .map(|track| track._last_frame().unwrap_or(0))
            .max()
            .unwrap_or(0);

        let r_max = self
            .translations
            .values()
            .map(|track| track._last_frame().unwrap_or(0))
            .max()
            .unwrap_or(0);

        t_max.max(r_max)
    }
}
