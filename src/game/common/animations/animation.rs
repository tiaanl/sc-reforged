use std::path::Path;

use ahash::HashMap;
use glam::{Quat, Vec3};

use crate::{
    engine::{
        assets::AssetError,
        prelude::Transform,
        storage::{Handle, Storage},
    },
    game::{data_dir::data_dir, skeleton::Skeleton},
    global,
};

use super::track::Track;

#[derive(Debug, Default)]
pub struct Animation {
    positions: HashMap<u32, Track<Vec3>>,
    rotations: HashMap<u32, Track<Quat>>,
}

impl Animation {
    pub fn last_key_frame(&self) -> Option<u32> {
        let max_pos = self.positions.values().filter_map(|t| t.last_frame()).max();
        let max_rot = self.rotations.values().filter_map(|t| t.last_frame()).max();
        max_pos.into_iter().chain(max_rot).max()
    }

    pub fn sample_pose(&self, time: f32, fps: f32, skeleton: &Skeleton, looping: bool) -> Skeleton {
        let f = time * fps;

        let bones = skeleton
            .bones
            .iter()
            .map(|bone| {
                let mut bone = bone.clone();

                let translation = match self.positions.get(&bone.id) {
                    Some(t) => t.sample_sub_frame(f, looping),
                    None => bone.transform.translation,
                };
                let rotation = match self.rotations.get(&bone.id) {
                    Some(t) => t.sample_sub_frame(f, looping),
                    None => bone.transform.rotation,
                };

                bone.transform = Transform {
                    translation,
                    rotation,
                };

                bone
            })
            .collect();

        Skeleton { bones }
    }
}

pub struct Animations {
    animations: Storage<Animation>,
    lookup: HashMap<String, Handle<Animation>>,
}

impl Animations {
    pub fn new() -> Self {
        Self {
            animations: Storage::default(),
            lookup: HashMap::default(),
        }
    }

    pub fn _add(&mut self, animation: Animation) -> Handle<Animation> {
        self.animations.insert(animation)
    }

    pub fn get(&self, handle: Handle<Animation>) -> Option<&Animation> {
        self.animations.get(handle)
    }

    pub fn load(&mut self, path: impl AsRef<Path>) -> Result<Handle<Animation>, AssetError> {
        if let Some(animation) = self
            .lookup
            .get(&path.as_ref().to_string_lossy().to_string())
        {
            return Ok(*animation);
        }

        let animation = self.load_direct(path.as_ref())?;
        let handle = self.animations.insert(animation);
        self.lookup
            .insert(path.as_ref().to_string_lossy().to_string(), handle);
        Ok(handle)
    }

    pub fn load_direct(&self, path: impl AsRef<Path>) -> Result<Animation, AssetError> {
        let motion = data_dir().load_motion(path)?;

        fn convert_position(p: Vec3) -> Vec3 {
            Vec3::new(-p.x, p.y, p.z)
        }
        fn convert_rotation(q: Quat) -> Quat {
            Quat::from_xyzw(-q.x, -q.y, -q.z, q.w)
        }

        let mut animation = Animation::default();

        for kf in &motion.key_frames {
            let f = kf.frame;
            for b in &kf.bones {
                if let Some(p) = b.position {
                    animation
                        .positions
                        .entry(b.bone_id)
                        .or_default()
                        .insert(f, convert_position(p));
                }
                if let Some(r) = b.rotation {
                    animation
                        .rotations
                        .entry(b.bone_id)
                        .or_default()
                        .insert(f, convert_rotation(r));
                }
            }
        }

        Ok(animation)
    }
}

global!(Animations, scoped_animations, animations);
