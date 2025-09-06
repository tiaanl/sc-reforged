use std::{collections::hash_map::Iter, path::PathBuf};

use ahash::{HashMap, HashMapExt};

use crate::{
    engine::{
        assets::AssetError,
        storage::{Handle, Storage},
    },
    game::{config, data_dir::data_dir},
    global,
};

use super::animations;
use super::{Clip, Sequence};

pub struct Sequences {
    sequences: Storage<Sequence>,
    lookup: HashMap<String, Handle<Sequence>>,
}

impl Sequences {
    pub fn new() -> Result<Self, AssetError> {
        let motion_sequences = data_dir().load_config::<config::MotionSequencerDefs>(
            PathBuf::from("config").join("mot_sequencer_defs.txt"),
        )?;

        let mut sequences = Storage::with_capacity(motion_sequences.sequences.len());
        let mut lookup = HashMap::with_capacity(motion_sequences.sequences.len());

        for config_sequence in motion_sequences.sequences.iter() {
            let mut sequence = Sequence {
                name: config_sequence.name.clone(),
                clips: Vec::default(),
            };

            for config_motion in config_sequence.motions.iter() {
                sequence.clips.push(Clip {
                    animation: animations().load(
                        PathBuf::from("motions")
                            .join(config_motion.name.clone())
                            .with_extension("bmf"),
                    )?,
                    _immediate: config_motion.immediate,
                    repeat: config_motion.repeat,
                    _callbacks: config_motion.callbacks.clone(),
                });
            }

            let handle = sequences.insert(sequence);
            lookup.insert(config_sequence.name.clone(), handle);
        }

        Ok(Self { sequences, lookup })
    }

    pub fn lookup(&self) -> Iter<'_, String, Handle<Sequence>> {
        self.lookup.iter()
    }

    #[inline]
    pub fn get(&self, handle: Handle<Sequence>) -> Option<&Sequence> {
        self.sequences.get(handle)
    }

    #[inline]
    pub fn get_by_name(&self, name: &str) -> Option<Handle<Sequence>> {
        self.lookup.get(name).cloned()
    }
}

global!(Sequences, scoped_sequences, sequences);
