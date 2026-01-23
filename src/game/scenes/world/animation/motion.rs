use ahash::{HashMap, HashMapExt};
use glam::{Quat, Vec3};
use shadow_company_tools::bmf;

use crate::{
    engine::assets::AssetError,
    game::{Asset, AssetLoadContext, skeleton::Skeleton, track::Track},
};

/// Holds key frames for a skeleton. Vec indices correspond to [BoneIndex] in the skeleton.
#[derive(Default)]
pub struct Motion {
    pub name: String,
    pub bone_ids: Vec<super::BoneIndex>,
    pub translations: HashMap<u32, Track<Vec3>>,
    pub rotations: HashMap<u32, Track<Quat>>,
}

impl Motion {
    /// Create an [Animation] with the bone structure of the given [Skeleton] and empty tracks.
    pub fn from_skeleton(name: String, skeleton: &Skeleton) -> Self {
        Self {
            name,
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

impl Asset for Motion {
    fn from_memory(
        context: &mut AssetLoadContext,
        path: std::path::PathBuf,
        data: &[u8],
    ) -> Result<Self, AssetError> {
        use glam::{Quat, Vec3};

        let bmf = bmf::Motion::read(&mut std::io::Cursor::new(data))
            .map_err(|err| AssetError::from_io_error(err, path.as_ref()))?;

        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string());

        let bones_len = bmf.bone_ids.len();
        let mut motion = Motion {
            name: name.unwrap_or_default(),
            bone_ids: bmf.bone_ids.iter().map(|&id| id as _).collect(),
            translations: HashMap::with_capacity(bones_len),
            rotations: HashMap::with_capacity(bones_len),
        };

        fn convert_position(p: Vec3) -> Vec3 {
            Vec3::new(-p.x, p.y, p.z)
        }

        fn convert_rotation(q: Quat) -> Quat {
            Quat::from_xyzw(-q.x, -q.y, -q.z, q.w)
        }

        for bmf_frame in bmf.key_frames.iter() {
            let frame_num = bmf_frame.frame;

            for bone_index in 0..bmf_frame.bones.len() {
                let bone_id = bmf.bone_ids[bone_index];
                let bone = &bmf_frame.bones[bone_index];

                if let Some(translation) = bone.position {
                    motion
                        .translations
                        .entry(bone_id)
                        .or_default()
                        .insert(frame_num, convert_position(translation));
                }

                if let Some(rotation) = bone.rotation {
                    motion
                        .rotations
                        .entry(bone_id)
                        .or_default()
                        .insert(frame_num, convert_rotation(rotation));
                }
            }
        }

        Ok(motion)
    }
}
