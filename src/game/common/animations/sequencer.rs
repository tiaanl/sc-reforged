use crate::engine::storage::Handle;

use super::{Animation, Sequence, animations, sequences};

#[derive(Clone, Copy)]
pub struct AnimationState {
    pub animation: Handle<Animation>,
    pub time: f32,
}

/// Keeps track of a playing sequence.
#[derive(Default)]
pub struct Sequencer {
    sequence: Option<Handle<Sequence>>,
    time: f32,
}

impl Sequencer {
    pub fn update(&mut self, delta_time: f32) {
        self.time += delta_time;
    }

    pub fn is_playing(&self) -> bool {
        self.sequence.is_some()
    }

    pub fn play(&mut self, sequence: Handle<Sequence>) {
        self.sequence = Some(sequence);
        self.time = 0.0;
    }

    pub fn stop(&mut self) {
        self.sequence = None;
    }

    pub fn get_animation_state(&self) -> Option<AnimationState> {
        let sequence = sequences().get(self.sequence?)?;

        if sequence.clips.is_empty() {
            return None;
        }

        let mut time = self.time;

        for clip in sequence.clips.iter() {
            let animation = animations().get(clip.animation)?;
            let duration = animation.last_key_frame()? as f32;

            if time > duration as f32 {
                time -= duration;
            } else {
                return Some(AnimationState {
                    animation: clip.animation,
                    time,
                });
            }
        }

        None
    }
}
