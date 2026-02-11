#![allow(dead_code)]

use bevy_ecs::prelude::*;

use std::{collections::VecDeque, path::PathBuf, str::FromStr};

use ahash::{HashMap, HashMapExt};

use crate::{
    engine::{
        assets::AssetError,
        storage::{Handle, Storage},
    },
    game::{
        AssetLoader, AssetReader, config::MotionSequencerDefs, data_dir::data_dir,
        scenes::world::animation::motion::Motion,
    },
};

#[derive(strum::EnumString)]
pub enum State {
    #[strum(serialize = "MSEQ_STATE_STAND")]
    Stand,
    #[strum(serialize = "MSEQ_STATE_CROUCH")]
    Crouch,
    #[strum(serialize = "MSEQ_STATE_PRONE")]
    Prone,
    #[strum(serialize = "MSEQ_STATE_ON_BACK")]
    OnBack,
    #[strum(serialize = "MSEQ_STATE_SIT")]
    Sit,
    #[strum(serialize = "MSEQ_STATE_SCUBA")]
    Scuba,
}

/// A motion that must play when switching between the `from` and to `state`.
pub struct TransitionSequenceDef {
    from: State,
    to: State,
    motion: Handle<Motion>,
}

/// A sequence of motions that will play.
#[derive(Debug)]
pub struct SequenceDef {
    entries: Vec<SequenceDefEntry>,
}

#[derive(Debug)]
pub struct SequenceDefEntry {
    pub motion: Handle<Motion>,
    pub immediate: bool,
    pub repeat: Repeat,
    pub callbacks: Vec<MotionCallback>,
}

#[derive(Clone, Copy, Debug)]
pub enum Repeat {
    None,
    Infinite,
    Count(i32),
}

#[derive(Clone, Debug)]
pub enum MotionCallback {
    NotifyEnd,
    Frame { name: String, frame: i32 },
}

type Lookup<T> = HashMap<String, Handle<T>>;

#[derive(Resource)]
pub struct Sequences {
    transition_sequences: HashMap<String, TransitionSequenceDef>,
    sequences: Storage<SequenceDef>,
    sequences_lookup: HashMap<String, Handle<SequenceDef>>,
}

impl Sequences {
    pub fn new(assets: &mut AssetLoader) -> Result<Self, AssetError> {
        let motion_sequence_defs = data_dir().load_config::<MotionSequencerDefs>(
            PathBuf::from("config").join("mot_sequencer_defs.txt"),
        )?;

        let transition_sequences = {
            let mut out = HashMap::default();

            for transition_sequence in motion_sequence_defs.transition_sequences.iter() {
                let (motion, _) = assets.get_or_load_motion(&transition_sequence.motion)?;
                out.entry(transition_sequence.name.clone())
                    .or_insert(TransitionSequenceDef {
                        from: State::from_str(&transition_sequence.from_state).unwrap(),
                        to: State::from_str(&transition_sequence.from_state).unwrap(),
                        motion,
                    });
            }

            for transition_sequence in motion_sequence_defs.transition_sequences.iter() {
                assets.get_or_load_motion(&transition_sequence.motion)?;
            }

            out
        };

        let (sequences, sequences_lookup) = {
            use crate::game::config::Callback as C;
            use crate::game::config::Repeat as R;

            let mut storage = Storage::with_capacity(motion_sequence_defs.sequences.len());
            let mut lookup = HashMap::with_capacity(motion_sequence_defs.sequences.len());

            for sequence in motion_sequence_defs.sequences.iter() {
                lookup.entry(sequence.name.clone()).or_insert({
                    let sequence_def = SequenceDef {
                        entries: sequence
                            .motions
                            .iter()
                            .filter_map(
                                |entry: &crate::game::config::Motion| -> Option<SequenceDefEntry> {
                                    let (motion, _) = assets
                                        .get_or_load_motion(&entry.name)
                                        .inspect_err(|err| {
                                            tracing::warn!(
                                                "Could not load motion: \"{}\". ({err})",
                                                &entry.name
                                            );
                                        })
                                        .ok()?;

                                    Some(SequenceDefEntry {
                                        motion,
                                        immediate: entry.immediate,
                                        repeat: match entry.repeat {
                                            R::None => Repeat::None,
                                            R::Infinite => Repeat::Infinite,
                                            R::Count(count) => Repeat::Count(count),
                                        },
                                        callbacks: entry
                                            .callbacks
                                            .iter()
                                            .map(|c| match c {
                                                C::NotifyEnd => MotionCallback::NotifyEnd,
                                                C::Frame { name, frame } => MotionCallback::Frame {
                                                    name: name.clone(),
                                                    frame: *frame,
                                                },
                                            })
                                            .collect(),
                                    })
                                },
                            )
                            .collect(),
                    };

                    storage.insert(sequence_def)
                });
            }

            (storage, lookup)
        };

        Ok(Self {
            transition_sequences,
            sequences,
            sequences_lookup,
        })
    }

    #[inline]
    pub fn sequence_def_by_name(&self, name: &str) -> Option<&SequenceDef> {
        if let Some(&handle) = self.sequences_lookup.get(name) {
            self.sequences.get(handle)
        } else {
            None
        }
    }
}

/// Play a sequence of motions in order.
#[derive(Component, Debug, Default)]
pub struct Sequencer {
    entries: VecDeque<SequencerEntry>,
    time: f32,
    next_sequence: Option<String>,
}

impl Sequencer {
    #[inline]
    pub fn play(&mut self, sequence: &str) {
        self.next_sequence = Some(sequence.into());
    }

    #[inline]
    pub fn next_sequence(&mut self) -> Option<String> {
        self.next_sequence.take()
    }

    pub fn enqueue(&mut self, assets: &AssetReader, sequence_def: &SequenceDef) {
        self.time = 0.0;
        self.entries = sequence_def
            .entries
            .iter()
            .filter_map(|entry| {
                let motion = assets.get_motion(entry.motion)?;

                let motion_time = motion.max_frame_num() as f32;

                Some(SequencerEntry {
                    motion: entry.motion,
                    play_time: match entry.repeat {
                        Repeat::None => motion_time,
                        Repeat::Infinite => f32::INFINITY,
                        Repeat::Count(count) => count as f32 * motion_time,
                    },
                })
            })
            .collect();
    }

    pub fn update(&mut self, delta_time: f32) {
        self.time += delta_time;

        // Advance through any entries that have finished, carrying over leftover time.
        while let Some(play_time) = self.entries.front().map(|entry| entry.play_time) {
            if self.time >= play_time {
                self.time -= play_time;
                self.next();
                continue;
            }

            break;
        }
    }

    /// Play the next entry in the sequence.
    fn next(&mut self) {
        // Just pop the front, if it was or is empty, that is fine.
        let _ = self.entries.pop_front();
    }

    /// Get the currently playing motion handle and its local time.
    #[inline]
    pub fn get(&self) -> Option<(Handle<Motion>, f32)> {
        self.entries
            .front()
            .map(|entry| (entry.motion, entry.play_time))
    }
}

#[cfg(feature = "egui")]
impl Sequencer {
    pub fn ui(&self, ui: &mut egui::Ui, assets: &AssetReader) {
        use crate::engine::egui_integration::UiExt;

        if self.entries.is_empty() {
            return;
        }

        ui.h2("Sequences");

        for entry in self.entries.iter() {
            if let Some(motion) = assets.get_motion(entry.motion) {
                ui.label(&motion.name);
            }
            ui.label(format!("Time: {}", entry.play_time));
        }
    }
}

#[derive(Debug)]
struct SequencerEntry {
    motion: Handle<Motion>,
    play_time: f32,
}
