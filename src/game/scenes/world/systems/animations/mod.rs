#![allow(unused)]

pub mod animation;
pub mod pose;
pub mod skeleton;

pub type BoneIndex = u32;

pub const BONE_SENTINEL: BoneIndex = BoneIndex::MAX;

pub fn generate_pose(animation: &animation::Animation, time: f32, looping: bool) -> pose::Pose {
    let translations = animation
        .translations
        .iter()
        .map(|track| track.sample_sub_frame(time, looping));

    let rotations = animation
        .rotations
        .iter()
        .map(|track| track.sample_sub_frame(time, looping));

    pose::Pose {
        bones: translations
            .zip(rotations)
            .map(|(translation, rotation)| pose::PoseBone {
                translation,
                rotation,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use glam::Vec3;

    #[test]
    fn basic() {
        let bones = &[
            skeleton::Bone {
                name: String::from("root"),
                parent: BONE_SENTINEL,
            },
            skeleton::Bone {
                name: String::from("bone_l"),
                parent: 0,
            },
            skeleton::Bone {
                name: String::from("bone_r"),
                parent: 0,
            },
        ];
        let skeleton = skeleton::Skeleton::from_slice(bones);

        let mut animation = animation::Animation::from_skeleton(&skeleton);
        animation.translations[0].insert(0, Vec3::splat(0.1));
        animation.translations[0].insert(9, Vec3::splat(0.9));

        let pose = generate_pose(&animation, 0.0, false);
        assert_eq!(pose.bones[0].translation, Vec3::splat(0.1));

        let pose = generate_pose(&animation, 9.0, false);
        assert_eq!(pose.bones[0].translation, Vec3::splat(0.9));
    }
}
