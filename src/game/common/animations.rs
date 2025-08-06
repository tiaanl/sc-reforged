use std::path::Path;

use ahash::HashMap;
use glam::{Quat, Vec3};

use crate::{
    engine::{
        assets::AssetError,
        storage::{Handle, Storage},
    },
    game::{data_dir::data_dir, model::Node},
    global,
};

#[derive(Clone, Debug, Default)]
pub struct Sample {
    pub position: Option<Vec3>,
    pub rotation: Option<Quat>,
}

#[derive(Debug)]
pub struct Track {
    /// The ID of the bone we are animating.
    bone_id: u32,
    /// The data for the bone.
    sample: Sample,
}

#[derive(Debug, Default)]
pub struct KeyFrame {
    pub time: f32,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Default)]
pub struct Animation {
    /// A set of [KeyFrame]s that make up the animation.
    key_frames: Vec<KeyFrame>,
    /// The total length of the animation.
    length: f32,
}

impl Animation {
    pub fn from_key_frames(key_frames: Vec<KeyFrame>) -> Self {
        // Assuming the key frames are sorted by time.
        let length = key_frames
            .last()
            .map_or(0.0, |key_frame| key_frame.time + Animations::TIME_PER_FRAME);

        Self { key_frames, length }
    }
}

impl Animation {
    /// Samples the pose at the given time for the given nodes (by bone_id order).
    /// Always returns fully populated Samples (no None), falling back to node defaults.
    pub fn sample_pose(&self, time: f32, nodes: &[Node], looping: bool) -> Vec<Sample> {
        if self.key_frames.is_empty() {
            // Use node defaults if no animation data
            return nodes
                .iter()
                .map(|node| Sample {
                    position: Some(node.transform.translation),
                    rotation: Some(node.transform.rotation),
                })
                .collect();
        }

        let time = if looping && self.length > 0.0 {
            time.rem_euclid(self.length)
        } else {
            time
        };

        // Find the two keyframes surrounding the time
        let (left, right) = match self
            .key_frames
            .windows(2)
            .find(|w| time >= w[0].time && time <= w[1].time)
        {
            Some([left, right]) => (left, right),
            _ if time <= self.key_frames[0].time => {
                return nodes
                    .iter()
                    .map(|node| left_sample_for_bone_with_fallback(&self.key_frames[0], node))
                    .collect();
            }
            _ if time >= self.key_frames.last().unwrap().time => {
                return nodes
                    .iter()
                    .map(|node| {
                        left_sample_for_bone_with_fallback(self.key_frames.last().unwrap(), node)
                    })
                    .collect();
            }
            _ => {
                // Should not happen, but fallback to node defaults
                return nodes
                    .iter()
                    .map(|node| Sample {
                        position: Some(node.transform.translation),
                        rotation: Some(node.transform.rotation),
                    })
                    .collect();
            }
        };

        let t = (time - left.time) / (right.time - left.time);

        nodes
            .iter()
            .map(|node| {
                let left_sample = left
                    .tracks
                    .iter()
                    .find(|track| track.bone_id == node.bone_id)
                    .map(|t| &t.sample);
                let right_sample = right
                    .tracks
                    .iter()
                    .find(|track| track.bone_id == node.bone_id)
                    .map(|t| &t.sample);

                let position = match (
                    left_sample.and_then(|s| s.position),
                    right_sample.and_then(|s| s.position),
                ) {
                    (Some(l), Some(r)) => Some(l.lerp(r, t)),
                    (Some(l), None) => Some(l),
                    (None, Some(r)) => Some(r),
                    _ => Some(node.transform.translation),
                };

                let rotation = match (
                    left_sample.and_then(|s| s.rotation),
                    right_sample.and_then(|s| s.rotation),
                ) {
                    (Some(l), Some(r)) => Some(l.slerp(r, t)),
                    (Some(l), None) => Some(l),
                    (None, Some(r)) => Some(r),
                    _ => Some(node.transform.rotation),
                };

                let rotation = rotation.map(|rot| node.transform.rotation * rot);

                Sample { position, rotation }
            })
            .collect()
    }
}

// Helper: get sample for a bone from a keyframe, or fallback to node defaults
fn left_sample_for_bone_with_fallback(keyframe: &KeyFrame, node: &Node) -> Sample {
    keyframe
        .tracks
        .iter()
        .find(|track| track.bone_id == node.bone_id)
        .map(|t| Sample {
            position: t.sample.position.or(Some(node.transform.translation)),
            rotation: t.sample.rotation.or(Some(node.transform.rotation)),
        })
        .unwrap_or(Sample {
            position: Some(node.transform.translation),
            rotation: Some(node.transform.rotation),
        })
}

pub struct Animations {
    animations: Storage<Animation>,
    lookup: HashMap<String, Handle<Animation>>,
}

impl Animations {
    pub const TIME_PER_FRAME: f32 = 1.0 / 30.0;

    pub fn new() -> Self {
        Self {
            animations: Storage::default(),
            lookup: HashMap::default(),
        }
    }

    pub fn get(&self, handle: Handle<Animation>) -> Option<&Animation> {
        self.animations.get(handle)
    }

    pub fn load(&mut self, path: impl AsRef<Path>) -> Result<Handle<Animation>, AssetError> {
        let animation = self.load_direct(path.as_ref())?;

        let handle = self.animations.insert(animation);
        self.lookup
            .insert(path.as_ref().to_string_lossy().to_string(), handle);

        Ok(handle)
    }

    pub fn load_direct(&self, path: impl AsRef<Path>) -> Result<Animation, AssetError> {
        let motion = data_dir()._load_motion(path)?;

        Ok(Animation::from_key_frames(
            motion
                .key_frames
                .iter()
                .map(|motion_key_frame| KeyFrame {
                    time: motion_key_frame.frame as f32 * Animations::TIME_PER_FRAME,
                    tracks: motion_key_frame
                        .bones
                        .iter()
                        .map(|motion_bone| Track {
                            bone_id: motion_bone.bone_id,
                            sample: Sample {
                                position: motion_bone
                                    .position
                                    .map(|position| Vec3::new(-position.x, position.y, position.z)),
                                rotation: motion_bone.rotation.map(|rotation| {
                                    Quat::from_xyzw(
                                        -rotation.x,
                                        -rotation.y,
                                        -rotation.z,
                                        rotation.w,
                                    )
                                }),
                            },
                        })
                        .collect(),
                })
                .collect(),
        ))
    }
}

global!(Animations, scoped_animations, animations);

pub mod old {
    use glam::{Quat, Vec3};

    pub trait Interpolate: Clone {
        fn interpolate(left: &Self, right: &Self, n: f32) -> Self;
    }

    impl Interpolate for f32 {
        #[inline]
        fn interpolate(left: &Self, right: &Self, n: f32) -> Self {
            left + (right - left) * n
        }
    }

    impl Interpolate for Vec3 {
        #[inline]
        fn interpolate(left: &Self, right: &Self, n: f32) -> Self {
            left.lerp(*right, n)
        }
    }

    impl Interpolate for Quat {
        fn interpolate(left: &Self, right: &Self, n: f32) -> Self {
            left.slerp(*right, n)
        }
    }

    impl<A: Interpolate, B: Interpolate> Interpolate for (A, B) {
        fn interpolate(left: &Self, right: &Self, n: f32) -> Self {
            (
                A::interpolate(&left.0, &right.0, n),
                B::interpolate(&left.1, &right.1, n),
            )
        }
    }

    pub struct KeyFrame<V: Interpolate> {
        time: f32,
        value: V,
    }

    #[derive(Default)]
    pub struct Track<V: Interpolate> {
        key_frames: Vec<KeyFrame<V>>,
    }

    impl<V: Interpolate> Track<V> {
        pub fn set_key_frame(&mut self, time: f32, value: V) {
            let pos = self
                .key_frames
                .binary_search_by(|key_frame| key_frame.time.partial_cmp(&time).unwrap())
                .unwrap_or_else(|e| e);
            self.key_frames.insert(pos, KeyFrame { time, value });
        }

        pub fn get(&self, time: f32) -> V {
            let len = self.key_frames.len();
            if len == 0 {
                panic!("No keyframes in track");
            }

            if time <= self.key_frames[0].time {
                return self.key_frames[0].value.clone();
            }

            if time >= self.key_frames[len - 1].time {
                return self.key_frames[len - 1].value.clone();
            }

            for window in self.key_frames.windows(2) {
                let (left, right) = (&window[0], &window[1]);
                if time >= left.time && time <= right.time {
                    let t = (time - left.time) / (right.time - left.time);
                    return V::interpolate(&left.value, &right.value, t);
                }
            }

            unreachable!()
        }
    }
}
