use ahash::{HashMap, HashSet};
use bevy_ecs::prelude::*;
use glam::Vec3;

use std::{path::PathBuf, str::FromStr};

use crate::{
    engine::{assets::AssetError, storage::Handle},
    game::{AssetLoader, config::parser::ConfigLines, scenes::world::animation::motion::Motion},
};

/// Canonical motion-sequencer sequence names used by gameplay systems.
#[derive(
    Clone, Copy, Debug, Eq, Hash, PartialEq, strum::AsRefStr, strum::Display, strum::EnumString,
)]
pub enum SequenceName {
    #[strum(serialize = "MSEQ_WALK")]
    Walk,
    #[strum(serialize = "MSEQ_INTO_WALK")]
    IntoWalk,
    #[strum(serialize = "MSEQ_OUTOF_WALK")]
    OutOfWalk,
    #[strum(serialize = "MSEQ_STAND")]
    Stand,
    #[strum(serialize = "MSEQ_CROUCH")]
    Crouch,
    #[strum(serialize = "MSEQ_PRONE")]
    Prone,
}

impl SequenceName {
    /// Return the string identifier expected by the motion sequencer definition table.
    #[inline]
    pub const fn as_sequence_str(self) -> &'static str {
        match self {
            Self::Walk => "MSEQ_WALK",
            Self::IntoWalk => "MSEQ_INTO_WALK",
            Self::OutOfWalk => "MSEQ_OUTOF_WALK",
            Self::Stand => "MSEQ_STAND",
            Self::Crouch => "MSEQ_CROUCH",
            Self::Prone => "MSEQ_PRONE",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, strum::EnumString)]
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
pub struct TransitionSequence {
    from: State,
    to: State,
    motion: Handle<Motion>,
}

/// A sequence of motions that will play.
#[derive(Debug)]
pub struct Sequence {
    pub entries: Vec<SequenceEntry>,
}

#[derive(Debug)]
pub struct SequenceEntry {
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

/// A callback emitted by a [Sequencer] while playing a motion.
#[derive(Clone, Debug)]
pub struct SequencerCallbackEvent {
    /// The motion that produced the callback.
    pub motion: Handle<Motion>,
    /// The callback payload from the sequence definition.
    pub callback: MotionCallback,
    /// Local motion frame at which the callback fired.
    pub at_frame: f32,
}

type Lookup<T> = HashMap<String, Handle<T>>;

#[derive(Default, Resource)]
pub struct Sequences {
    transition_sequences: HashMap<String, TransitionSequence>,
    transition_lookup: HashMap<(State, State), Handle<Motion>>,
    default_cog_positions: HashMap<State, Vec3>,
    no_linear_velocity_motions: HashSet<Handle<Motion>>,
    sequences: HashMap<SequenceName, Sequence>,
}

impl Sequences {
    pub fn new(assets: &mut AssetLoader) -> Result<Self, AssetError> {
        // let motion_sequence_defs = data_dir().load_config::<MotionSequencerDefs>(
        //     PathBuf::from("config").join("mot_sequencer_defs.txt"),
        // )?;

        // let (transition_sequences, transition_lookup) = {
        //     let mut out = HashMap::default();
        //     let mut lookup = HashMap::default();

        //     for transition_sequence in motion_sequence_defs.transition_sequences.iter() {
        //         let (motion, _) = assets.get_or_load_motion(&transition_sequence.motion)?;
        //         let from = State::from_str(&transition_sequence.from_state).unwrap();
        //         let to = State::from_str(&transition_sequence.to_state).unwrap();
        //         out.entry(transition_sequence.name.clone())
        //             .or_insert(TransitionSequence { from, to, motion });
        //         lookup.entry((from, to)).or_insert(motion);
        //     }

        //     for transition_sequence in motion_sequence_defs.transition_sequences.iter() {
        //         assets.get_or_load_motion(&transition_sequence.motion)?;
        //     }

        //     (out, lookup)
        // };

        // let (sequences, sequences_lookup) = {
        //     use crate::game::config::Callback as C;
        //     use crate::game::config::Repeat as R;

        //     let mut storage = Storage::with_capacity(motion_sequence_defs.sequences.len());
        //     let mut lookup = HashMap::with_capacity(motion_sequence_defs.sequences.len());

        //     for sequence in motion_sequence_defs.sequences.iter() {
        //         lookup.entry(sequence.name.clone()).or_insert({
        //             let sequence_def = SequenceDef {
        //                 end_state: infer_state_from_sequence_name(&sequence.name),
        //                 entries: sequence
        //                     .motions
        //                     .iter()
        //                     .filter_map(
        //                         |entry: &crate::game::config::Motion| -> Option<SequenceDefEntry> {
        //                             let (motion, _) = assets
        //                                 .get_or_load_motion(&entry.name)
        //                                 .inspect_err(|err| {
        //                                     tracing::warn!(
        //                                         "Could not load motion: \"{}\". ({err})",
        //                                         &entry.name
        //                                     );
        //                                 })
        //                                 .ok()?;

        //                             Some(SequenceDefEntry {
        //                                 motion,
        //                                 immediate: entry.immediate,
        //                                 repeat: match entry.repeat {
        //                                     R::None => Repeat::None,
        //                                     R::Infinite => Repeat::Infinite,
        //                                     R::Count(count) => Repeat::Count(count),
        //                                 },
        //                                 callbacks: entry
        //                                     .callbacks
        //                                     .iter()
        //                                     .map(|c| match c {
        //                                         C::NotifyEnd => MotionCallback::NotifyEnd,
        //                                         C::Frame { name, frame } => MotionCallback::Frame {
        //                                             name: name.clone(),
        //                                             frame: *frame,
        //                                         },
        //                                     })
        //                                     .collect(),
        //                             })
        //                         },
        //                     )
        //                     .collect(),
        //             };

        //             storage.insert(sequence_def)
        //         });
        //     }

        //     (storage, lookup)
        // };

        let mut result = Sequences::default();
        result.load_from_config(assets)?;
        Ok(result)
    }

    #[inline]
    pub fn sequence(&self, name: SequenceName) -> Option<&Sequence> {
        self.sequences.get(&name)
    }

    /// Look up the transition motion for `from -> to` posture states.
    #[inline]
    pub fn transition_motion(&self, from: State, to: State) -> Option<Handle<Motion>> {
        self.transition_lookup.get(&(from, to)).copied()
    }

    /// Return the default COG/root-bone translation for the given posture state.
    #[inline]
    pub fn default_cog_position(&self, state: State) -> Option<Vec3> {
        self.default_cog_positions.get(&state).copied()
    }

    /// Return default COG/root-bone translation for posture sequences.
    #[inline]
    pub fn default_cog_position_for_sequence(&self, sequence_name: SequenceName) -> Option<Vec3> {
        let state = match sequence_name {
            SequenceName::Stand => State::Stand,
            SequenceName::Crouch => State::Crouch,
            SequenceName::Prone => State::Prone,
            _ => return None,
        };
        self.default_cog_position(state)
    }

    /// Return whether linear-velocity data should be used for `motion`.
    #[inline]
    pub fn uses_linear_velocity(&self, motion: Handle<Motion>) -> bool {
        !self.no_linear_velocity_motions.contains(&motion)
    }
}

impl Sequences {
    /// Parse a state label used in sequence/config entries.
    fn parse_state_label(name: &str) -> Option<State> {
        if let Ok(state) = State::from_str(name) {
            return Some(state);
        }

        let upper = name.to_ascii_uppercase();
        if upper.contains("SCUBA") {
            Some(State::Scuba)
        } else if upper.contains("ON_BACK") || upper == "MSEQ_ON_BACK" {
            Some(State::OnBack)
        } else if upper.contains("CROUCH") {
            Some(State::Crouch)
        } else if upper.contains("PRONE") || upper.contains("BELLY_CRAWL") {
            Some(State::Prone)
        } else if upper.contains("SIT") {
            Some(State::Sit)
        } else if upper.contains("STAND") {
            Some(State::Stand)
        } else {
            None
        }
    }

    fn load_from_config(&mut self, assets: &mut AssetLoader) -> Result<(), AssetError> {
        // config/mot_sequencer_defs.txt
        let data = assets.load_raw(PathBuf::from("config").join("mot_sequencer_defs.txt"))?;
        let text = String::from_utf8_lossy(&data);
        let config_lines = ConfigLines::parse(&text);
        self.parse(config_lines, assets)
    }

    /// Finalize a parsed transition-sequence block and update lookup tables.
    fn commit_transition_sequence(
        &mut self,
        assets: &mut AssetLoader,
        from_state: String,
        to_state: String,
        name: String,
        motion_name: Option<String>,
    ) -> Result<(), AssetError> {
        let Some(motion_name) = motion_name else {
            tracing::warn!("No motion specified for transition sequence: {name}");
            return Ok(());
        };

        let Ok(from) = State::from_str(&from_state) else {
            tracing::warn!("Invalid from state: {from_state}");
            return Ok(());
        };

        let Ok(to) = State::from_str(&to_state) else {
            tracing::warn!("Invalid to state: {to_state}");
            return Ok(());
        };

        let (motion, _) = assets.get_or_load_motion(&motion_name)?;
        self.transition_lookup.insert((from, to), motion);
        self.transition_sequences
            .insert(name, TransitionSequence { from, to, motion });

        Ok(())
    }

    /// Finalize a parsed sequence block and store it in the sequence map.
    fn commit_sequence(&mut self, name: SequenceName, entries: Vec<SequenceEntry>) {
        self.sequences.insert(name, Sequence { entries });
    }
}
