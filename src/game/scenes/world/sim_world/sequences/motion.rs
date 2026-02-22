use bitflags::bitflags;
use glam::{Quat, Vec3};
use shadow_company_tools::bmf;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::{
    engine::assets::AssetError,
    game::{Asset, AssetLoadContext},
};

use super::state::State;

bitflags! {
    /// Per-motion behavior flags used by sequencer/runtime systems.
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    pub struct MotionFlags: u32 {
        /// Enables Z-index/depth-style behavior for this motion.
        const Z_IND_MOTION = 1 << 0;
        /// Disables linear/root velocity extraction from this motion.
        const NO_LVE_MOTION = 1 << 1;
        /// Skips terminal keyframe handling at motion end.
        const SKIP_LAST_FRAME = 1 << 2;
        /// Enables sped-motion time scaling behavior.
        const SPED_MOTION = 1 << 3;
    }
}

/// Holds key frames for a skeleton. Vec indices correspond to [BoneIndex] in the skeleton.
#[derive(Debug)]
pub struct Motion {
    pub name: String,
    /// Number of keyframes in this motion.
    pub frame_count: u32,
    /// Last frame index of this motion timeline.
    pub last_frame: u32,
    /// Base timing step for frame advancement.
    pub base_ticks_per_frame: u32,
    pub from_state: State,
    pub to_state: State,
    /// Raw keyframes as loaded from the motion asset.
    pub key_frames: Vec<bmf::KeyFrame>,
    flags: AtomicU32,
}

/// A sampled per-bone update produced from keyframe interpolation/apply rules.
#[derive(Clone, Copy, Debug)]
pub struct BoneSampleUpdate {
    /// Target bone id in the skeleton.
    pub bone_id: u32,
    /// Sampled translation channel, when available.
    pub translation: Option<Vec3>,
    /// Sampled rotation channel, when available.
    pub rotation: Option<Quat>,
}

impl Motion {
    /// Resolve adjacent keyframes and interpolation factor similar to original runtime behavior.
    fn interpolation_pair(
        &self,
        time: f32,
        looping: bool,
    ) -> Option<(&bmf::KeyFrame, &bmf::KeyFrame, f32)> {
        let frame_count = self.key_frames.len();
        if frame_count == 0 {
            return None;
        }
        if frame_count == 1 {
            let frame = &self.key_frames[0];
            return Some((frame, frame, 0.0));
        }

        let max_frame = (frame_count - 1) as f32;
        let local_time = if looping {
            time.rem_euclid(frame_count as f32)
        } else {
            time.clamp(0.0, max_frame)
        };

        let left_index = (local_time.floor() as usize).min(frame_count - 1);
        let right_index = if left_index + 1 < frame_count {
            left_index + 1
        } else if looping {
            0
        } else {
            left_index
        };

        let left = &self.key_frames[left_index];
        let right = &self.key_frames[right_index];

        if left_index == right_index {
            return Some((left, right, 0.0));
        }

        let left_time = left.frame as f32;
        let right_time = right.frame as f32;
        let denom = right_time - left_time;
        if denom.abs() <= f32::EPSILON {
            return Some((left, right, 0.0));
        }

        let t = ((local_time - left_time) / denom).abs();
        // Original runtime skips interpolation when t is outside [0, 1].
        let t = if (0.0..=1.0).contains(&t) { t } else { 0.0 };
        Some((left, right, t))
    }

    /// Normalize a quaternion and fall back to identity when invalid.
    #[inline]
    fn normalize_rotation_or_identity(rotation: Quat) -> Quat {
        let length_sq = rotation.length_squared();
        if !length_sq.is_finite() || length_sq <= f32::EPSILON {
            Quat::IDENTITY
        } else {
            rotation / length_sq.sqrt()
        }
    }

    /// Convert source quaternion basis into engine local-space basis.
    #[inline]
    fn convert_source_rotation(rotation: Quat) -> Quat {
        Quat::from_xyzw(-rotation.x, -rotation.y, -rotation.z, rotation.w)
    }

    /// Convert source translation/root vectors into engine local-space basis.
    #[inline]
    fn convert_source_translation(translation: Vec3) -> Vec3 {
        Vec3::new(-translation.x, translation.y, translation.z)
    }

    /// Quaternion interpolation matching original shortest-path + near-linear fallback.
    fn interpolate_rotation(left: Quat, right: Quat, t: f32) -> Quat {
        let left = Self::normalize_rotation_or_identity(left);
        let right = Self::normalize_rotation_or_identity(right);

        let mut dot = left.dot(right);
        let mut adjusted_right = right;
        if dot < 0.0 {
            dot = -dot;
            adjusted_right = -adjusted_right;
        }

        let (left_weight, right_weight) = if 1.0 - dot <= 0.005 {
            (1.0 - t, t)
        } else {
            let theta = dot.acos();
            let sin_theta = theta.sin();
            if sin_theta.abs() <= f32::EPSILON {
                (1.0 - t, t)
            } else {
                (
                    ((1.0 - t) * theta).sin() / sin_theta,
                    (t * theta).sin() / sin_theta,
                )
            }
        };

        Self::normalize_rotation_or_identity(Quat::from_xyzw(
            left.x * left_weight + adjusted_right.x * right_weight,
            left.y * left_weight + adjusted_right.y * right_weight,
            left.z * left_weight + adjusted_right.z * right_weight,
            left.w * left_weight + adjusted_right.w * right_weight,
        ))
    }

    /// Sample all per-bone channel updates for a timeline time.
    ///
    /// This follows the original keyframe interpolation behavior where bones are
    /// paired by list index between left/right keyframes.
    #[inline]
    pub fn sample_bone_updates(&self, time: f32, looping: bool) -> Vec<BoneSampleUpdate> {
        let Some((left, right, t)) = self.interpolation_pair(time, looping) else {
            return Vec::new();
        };

        let pair_count = left.bones.len().min(right.bones.len());
        let mut updates = Vec::with_capacity(pair_count);

        for index in 0..pair_count {
            let left_bone = &left.bones[index];
            let right_bone = &right.bones[index];

            let rotation = match (left_bone.rotation, right_bone.rotation) {
                (Some(left_rotation), Some(right_rotation)) => Some(Self::interpolate_rotation(
                    Self::convert_source_rotation(left_rotation),
                    Self::convert_source_rotation(right_rotation),
                    t,
                )),
                _ => None,
            };

            let translation = match (left_bone.position, right_bone.position) {
                (Some(left_position), Some(right_position)) => Some(
                    Self::convert_source_translation(left_position)
                        .lerp(Self::convert_source_translation(right_position), t),
                ),
                _ => None,
            };

            updates.push(BoneSampleUpdate {
                bone_id: left_bone.bone_id,
                translation,
                rotation,
            });
        }

        updates
    }

    /// Sample all per-bone channel updates from an exact keyframe.
    #[inline]
    pub fn sample_bone_updates_at_key_frame(&self, key_frame_index: u32) -> Vec<BoneSampleUpdate> {
        let Some(key_frame) = self.key_frames.get(key_frame_index as usize) else {
            return Vec::new();
        };

        let mut updates = Vec::with_capacity(key_frame.bones.len());
        for bone in &key_frame.bones {
            updates.push(BoneSampleUpdate {
                bone_id: bone.bone_id,
                translation: bone.position.map(Self::convert_source_translation),
                rotation: bone
                    .rotation
                    .map(Self::convert_source_rotation)
                    .map(Self::normalize_rotation_or_identity),
            });
        }

        updates
    }

    /// Sample linear velocity at `time` in local motion frame-space.
    #[inline]
    pub fn sample_linear_velocity(&self, time: f32, looping: bool) -> Vec3 {
        let Some((left, right, t)) = self.interpolation_pair(time, looping) else {
            return Vec3::ZERO;
        };
        Self::convert_source_translation(left.lve)
            .lerp(Self::convert_source_translation(right.lve), t)
    }

    /// Return the motion declaration flags currently applied to this motion.
    #[inline]
    pub fn flags(&self) -> MotionFlags {
        MotionFlags::from_bits_retain(self.flags.load(Ordering::Relaxed))
    }

    /// Set declaration flags on this motion.
    #[inline]
    pub fn add_flags(&self, flags: MotionFlags) {
        self.flags.fetch_or(flags.bits(), Ordering::Relaxed);
    }

    /// Return true when all `flags` are set on this motion.
    #[inline]
    pub fn has_flags(&self, flags: MotionFlags) -> bool {
        self.flags().contains(flags)
    }
}

impl Asset for Motion {
    fn from_memory(
        _context: &mut AssetLoadContext,
        path: std::path::PathBuf,
        data: &[u8],
    ) -> Result<Self, AssetError> {
        const MOTION_HEADER_RUNTIME_TICKS_OFFSET: usize = 0x9c;

        fn read_u32_le_at(data: &[u8], offset: usize) -> Option<u32> {
            let bytes = data.get(offset..offset + 4)?;
            Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        }

        let bmf = bmf::Motion::read(&mut std::io::Cursor::new(data))
            .map_err(|err| AssetError::from_io_error(err, path.as_ref()))?;

        // The original runtime uses the serialized field at header offset 0x9c
        // for per-frame timing when activating motions.
        let runtime_ticks_per_frame =
            read_u32_le_at(data, MOTION_HEADER_RUNTIME_TICKS_OFFSET).unwrap_or(bmf.ticks_per_frame);

        let key_frames = bmf.key_frames;
        let motion = Motion {
            name: bmf.name,
            frame_count: bmf.key_frame_count,
            last_frame: bmf.last_frame,
            base_ticks_per_frame: runtime_ticks_per_frame.max(1),
            from_state: State::from_motion_state_id(bmf.from_state),
            to_state: State::from_motion_state_id(bmf.to_state),
            key_frames,
            flags: AtomicU32::new(MotionFlags::empty().bits()),
        };

        Ok(motion)
    }
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            name: String::new(),
            frame_count: 0,
            last_frame: 0,
            base_ticks_per_frame: 0,
            from_state: State::None,
            to_state: State::None,
            key_frames: Vec::new(),
            flags: AtomicU32::new(MotionFlags::empty().bits()),
        }
    }
}
