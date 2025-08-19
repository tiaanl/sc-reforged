use std::collections::VecDeque;

use crate::engine::storage::Handle;

use super::{Animation, Sequence, animations, sequences};

#[derive(Clone, Copy)]
pub struct AnimationState {
    pub animation: Handle<Animation>,
    pub time: f32,
}

enum Play {
    Single(Handle<Animation>),
}

/// Keeps track of a playing sequence.
#[derive(Default)]
pub struct Sequencer {
    sequence: VecDeque<Play>,
    time: f32,
}

impl Sequencer {
    pub fn update(&mut self, delta_time: f32) {
        let Some(front) = self.sequence.front() else {
            // There is nothing to play.
            return;
        };

        self.time += delta_time;

        match front {
            Play::Single(animation) => {
                let animation = animations().get(*animation).expect("Missing animation!");
                let duration = animation
                    .last_key_frame()
                    .expect("Playing empty animation!") as f32;
                if self.time >= duration {
                    self.time -= duration;
                    self.sequence.pop_front();
                }
            }
        }
    }

    pub fn is_playing(&self) -> bool {
        !self.sequence.is_empty()
    }

    pub fn play_sequence(&mut self, sequence: Handle<Sequence>) {
        // Always clear, even if we can't find the new sequence to play.
        self.sequence.clear();

        let Some(sequence) = sequences().get(sequence) else {
            tracing::warn!("Trying to play missing sequence: ({sequence})");
            return;
        };

        tracing::info!("Playing sequence: {}", sequence.name);

        for clip in sequence.clips.iter() {
            self.sequence.push_back(Play::Single(clip.animation));
        }

        // self.sequence = Some(sequence);
        // self.time = 0.0;
    }

    pub fn play_animation(&mut self, animation: Handle<Animation>) {
        self.sequence.clear();
        self.sequence.push_back(Play::Single(animation));
    }

    pub fn stop(&mut self) {
        self.sequence.clear();
    }

    pub fn get_animation_state(&self) -> Option<AnimationState> {
        let Some(front) = self.sequence.front() else {
            // Sequence is empty.
            return None;
        };

        match front {
            Play::Single(animation) => Some(AnimationState {
                animation: *animation,
                time: self.time,
            }),
        }
    }

    pub fn debug_panel(&mut self, ui: &mut egui::Ui) {
        for play in self.sequence.iter() {
            match play {
                Play::Single(a) => {
                    let time_str = if let Some(animation) = animations().get(*a) {
                        animation
                            .last_key_frame()
                            .map(|t| t.to_string())
                            .unwrap_or_default()
                    } else {
                        String::new()
                    };
                    ui.label(format!("Single: {a} ({time_str})"));
                }
            }
        }
    }
}
