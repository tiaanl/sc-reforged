use std::{
    collections::{VecDeque, hash_map::Entry},
    path::PathBuf,
    str::FromStr,
};

use ahash::{HashMap, HashMapExt};

use crate::{
    engine::{
        assets::AssetError,
        storage::{Handle, Storage},
    },
    game::{
        config::MotionSequencerDefs,
        data_dir::data_dir,
        scenes::world::{animation::motion::Motion, systems::Time},
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
pub struct SequenceDef {
    entries: Vec<SequenceDefEntry>,
}

pub struct SequenceDefEntry {
    pub motion: Handle<Motion>,
    pub immediate: bool,
    pub repeat: Repeat,
    pub callbacks: Vec<MotionCallback>,
}

#[derive(Clone, Copy)]
pub enum Repeat {
    None,
    Infinite,
    Count(i32),
}

#[derive(Clone)]
pub enum MotionCallback {
    NotifyEnd,
    Frame { name: String, frame: i32 },
}

type Lookup<T> = HashMap<String, Handle<T>>;

pub struct Sequences {
    motions: Storage<Motion>,
    transition_sequences: HashMap<String, TransitionSequenceDef>,
    sequences: Storage<SequenceDef>,
    sequences_lookup: HashMap<String, Handle<SequenceDef>>,
}

impl Sequences {
    pub fn new() -> Result<Self, AssetError> {
        let motion_sequence_defs = data_dir().load_config::<MotionSequencerDefs>(
            PathBuf::from("config").join("mot_sequencer_defs.txt"),
        )?;

        let mut motions = Storage::default();
        let mut motions_lookup: HashMap<String, Handle<Motion>> = HashMap::default();

        let mut get_or_insert_motion = |name: String| -> Result<Handle<Motion>, AssetError> {
            let motion = match motions_lookup.entry(name) {
                Entry::Occupied(entry) => *entry.get(),
                Entry::Vacant(entry) => {
                    let handle = motions.insert(data_dir().load_motion(entry.key())?);
                    entry.insert(handle);
                    handle
                }
            };
            Ok(motion)
        };

        let transition_sequences = {
            let mut out = HashMap::default();

            for transition_sequence in motion_sequence_defs.transition_sequences.iter() {
                out.entry(transition_sequence.name.clone())
                    .or_insert(TransitionSequenceDef {
                        from: State::from_str(&transition_sequence.from_state).unwrap(),
                        to: State::from_str(&transition_sequence.from_state).unwrap(),
                        motion: get_or_insert_motion(transition_sequence.motion.clone())?,
                    });
            }

            for transition_sequence in motion_sequence_defs.transition_sequences.iter() {
                let name = transition_sequence.motion.clone();
                get_or_insert_motion(name)?;
            }

            out
        };

        let (sequences, sequences_lookup) =
            {
                use crate::game::config::Callback as C;
                use crate::game::config::Repeat as R;

                let mut storage = Storage::with_capacity(motion_sequence_defs.sequences.len());
                let mut lookup = HashMap::with_capacity(motion_sequence_defs.sequences.len());

                for sequence in motion_sequence_defs.sequences.iter() {
                    lookup.entry(sequence.name.clone()).or_insert({
                        let sequence_def = SequenceDef { entries: sequence
                            .motions
                            .iter()
                            .filter_map(
                                |entry: &crate::game::config::Motion| -> Option<SequenceDefEntry> {
                                    Some(SequenceDefEntry {
                                        motion: get_or_insert_motion(entry.name.clone()).ok()?,
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
                            .collect() };

                        let handle = storage.insert(sequence_def);

                        handle
                    });
                }

                (storage, lookup)
            };

        Ok(Self {
            motions,

            transition_sequences,
            sequences,
            sequences_lookup,
        })
    }

    #[inline]
    pub fn get_motion(&self, handle: Handle<Motion>) -> Option<&Motion> {
        self.motions.get(handle)
    }
}

/// Play a sequence of motions in order.
#[derive(Default)]
pub struct Sequencer {
    entries: VecDeque<SequencerEntry>,
    time: f32,
}

impl Sequencer {
    pub fn enqueue(&mut self, sequences: &Sequences, sequence_def: &SequenceDef) {
        self.time = 0.0;
        self.entries = sequence_def
            .entries
            .iter()
            .filter_map(|entry| {
                let Some(motion) = sequences.get_motion(entry.motion) else {
                    return None;
                };

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

    pub fn update(&mut self, time: &Time) {
        self.time += time.delta_time;

        // Advance through any entries that have finished, carrying over leftover time.
        while let Some(play_time) = self.entries.front().map(|entry| entry.play_time) {
            if play_time <= 0.0 {
                self.next();
                continue;
            }

            if self.time < play_time {
                break;
            }

            self.time -= play_time;
            self.next();
        }
    }

    /// Play the next entry in the sequence.
    fn next(&mut self) {
        // Just pop the front, if it was or is empty, that is fine.
        let _ = self.entries.pop_front();
    }

    pub fn get(&self) -> Option<(Handle<Motion>, f32)> {
        None
    }
}

#[cfg(feature = "egui")]
impl Sequencer {
    pub fn ui(&mut self, ui: &mut egui::Ui, sequences: &Sequences) {
        use crate::engine::egui_integration::UiExt;

        if self.entries.is_empty() {
            return;
        }

        ui.h2("Sequences");

        for motion in self
            .entries
            .iter()
            .map(|e| sequences.get_motion(e.motion).unwrap())
        {
            ui.h3(&motion.name);
        }
    }
}

struct SequencerEntry {
    motion: Handle<Motion>,
    play_time: f32,
}
