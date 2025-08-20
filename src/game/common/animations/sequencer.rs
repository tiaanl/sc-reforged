use std::collections::VecDeque;

use egui::{RichText, Widget};

use crate::engine::storage::Handle;

use super::{Animation, Sequence, animations, sequences};

#[derive(Clone, Copy)]
pub struct AnimationState {
    pub animation: Handle<Animation>,
    pub time: f32,
}

enum Play {
    Single(Handle<Animation>),
    Count(Handle<Animation>, i32),
    Loop(Handle<Animation>),
}

/// Keeps track of a playing sequence.
#[derive(Default)]
pub struct Sequencer {
    sequence: VecDeque<Play>,
    time: f32,
}

impl Sequencer {
    pub fn update(&mut self, delta_time: f32) {
        let Some(front) = self.sequence.front_mut() else {
            // There is nothing to play.
            return;
        };

        self.time += delta_time;

        match *front {
            Play::Single(animation) => {
                let animation = animations().get(animation).expect("Missing animation!");
                let duration = animation.last_key_frame().unwrap_or_default() as f32;
                if self.time >= duration {
                    self.time -= duration;
                }
            }

            Play::Count(animation, ref mut count) => {
                let animation = animations().get(animation).expect("Missing animation!");
                let duration = animation.last_key_frame().unwrap_or_default() as f32;
                if self.time >= duration {
                    self.time -= duration;
                    *count -= 1;

                    if *count == 0 {
                        self.sequence.pop_front();
                    }
                }
            }

            Play::Loop(animation) => {
                let animation = animations().get(animation).expect("Missing animation!");
                let duration = animation.last_key_frame().unwrap_or_default() as f32;
                self.time = self.time.rem_euclid(duration);
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
            use crate::game::config::Repeat;

            match clip.repeat {
                Repeat::None => self.sequence.push_back(Play::Single(clip.animation)),
                Repeat::Infinite => self.sequence.push_back(Play::Loop(clip.animation)),
                Repeat::Count(count) => self.sequence.push_back(Play::Count(clip.animation, count)),
            }
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
            Play::Single(animation) | Play::Count(animation, _) | Play::Loop(animation) => {
                Some(AnimationState {
                    animation: *animation,
                    time: self.time,
                })
            }
        }
    }

    pub fn debug_panel(&mut self, ui: &mut egui::Ui) {
        for play in self.sequence.iter() {
            match play {
                Play::Single(animation) => {
                    let key_frame_count = animations()
                        .get(*animation)
                        .and_then(|a| a.last_key_frame())
                        .unwrap_or_default();
                    play_panel("Single", *animation, key_frame_count, ui);
                }
                Play::Count(animation, _) => {
                    let key_frame_count = animations()
                        .get(*animation)
                        .and_then(|a| a.last_key_frame())
                        .unwrap_or_default();
                    play_panel("Count", *animation, key_frame_count, ui);
                }

                Play::Loop(animation) => {
                    let key_frame_count = animations()
                        .get(*animation)
                        .and_then(|a| a.last_key_frame())
                        .unwrap_or_default();
                    play_panel("Loop", *animation, key_frame_count, ui);
                }
            }
        }
    }
}

fn play_panel(typ: &str, animation: Handle<Animation>, key_frame_count: u32, ui: &mut egui::Ui) {
    egui::Frame::group(ui.style())
        .corner_radius(5.0)
        .stroke(egui::Stroke::new(1.0, ui.visuals().text_color()))
        .inner_margin(5.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                egui::Label::new(RichText::new(typ).strong()).ui(ui);
                ui.vertical(|ui| {
                    egui::Label::new(format!("Animation: {animation}")).ui(ui);
                    egui::Label::new(format!("Frames: {key_frame_count}")).ui(ui);
                });
            });
        });
}
