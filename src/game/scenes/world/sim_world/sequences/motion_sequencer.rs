use std::{
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use ahash::HashMap;
use bevy_ecs::prelude::*;
use glam::Vec3;

use crate::{
    engine::assets::AssetError,
    game::{Asset, AssetLoadContext, AssetLoader, config::parser::ConfigLines, hash},
};

use super::{
    motion::{Motion, MotionFlags},
    motion_controller::MotionController,
    motion_info::MotionInfo,
    sequence::Sequence,
    state::State,
};

#[derive(Clone, Debug, Event)]
pub struct MotionSequenceRequest {
    /// The entity that should play the sequence.
    pub entity: Entity,
    /// The hash of the [Sequence] to request.
    pub sequence_hash: u32,
    /// The speed multiplier to play the motions at.
    pub playback_speed: f32,
    /// Deduplicate by comparing the requested sequence's first motion with the
    /// most recently queued motion on the [MotionController].
    pub dedupe: bool,
    /// Force the queue on the [MotionController] to be cleared before adding
    /// the requested [Sequence].
    pub force_clear_queue: bool,
    /// Don't queue any state transition motions.
    pub skip_state_transitions: bool,
    /// Start tick override applied to the first queued motion in the requested sequence.
    pub first_entry_start_time_ticks: u32,
}

impl Default for MotionSequenceRequest {
    fn default() -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
            sequence_hash: u32::MAX,
            playback_speed: 1.0,
            dedupe: false,
            force_clear_queue: false,
            skip_state_transitions: false,
            first_entry_start_time_ticks: 0,
        }
    }
}

#[derive(Debug, Default, Resource)]
pub struct MotionSequencer {
    motions: HashMap<u32, Arc<Motion>>,
    sequences: HashMap<u32, Arc<Sequence>>,
    transition_sequences: HashMap<(State, State), Arc<Sequence>>,
    default_cog_positions: HashMap<State, Vec3>,
}

impl MotionSequencer {
    /// Return the default COG/root-bone translation for a posture state.
    pub fn default_cog_position(&self, state: State) -> Option<Vec3> {
        self.default_cog_positions.get(&state).copied()
    }

    pub fn request_sequence(
        &self,
        request: MotionSequenceRequest,
        motion_controller: &mut MotionController,
    ) -> bool {
        if request.entity == Entity::PLACEHOLDER {
            return false;
        }

        if request.sequence_hash == u32::MAX {
            return false;
        }

        let Some(sequence) = self.find_sequence(request.sequence_hash) else {
            return false;
        };

        if request.dedupe {
            let first_to_play = sequence.motions.first().map(|motion| motion.hash);
            let queued = motion_controller
                .get_most_recent_motion()
                .map(|motion| motion.hash);

            if let (Some(first_to_play), Some(queued)) = (first_to_play, queued) {
                if first_to_play == queued {
                    // No failure, but short circuit to avoid duplicates.
                    return true;
                }

                if request.force_clear_queue {
                    motion_controller.reset();
                }
            }
        }

        if !request.skip_state_transitions {
            let from_state = motion_controller.transition_check_state();
            let to_state = sequence.begin_state;

            if from_state != to_state
                && let Some(transition_sequence) =
                    self.transition_sequences.get(&(from_state, to_state))
            {
                transition_sequence.motions.iter().for_each(|motion_info| {
                    motion_controller
                        .push_motion_info(Arc::clone(motion_info), request.playback_speed);
                });
            }
        }

        for (index, motion_info) in sequence.motions.iter().enumerate() {
            if index == 0 {
                let mut first_motion_info = (*motion_info.as_ref()).clone();
                first_motion_info.start_time_ticks = request.first_entry_start_time_ticks;
                motion_controller
                    .push_motion_info(Arc::new(first_motion_info), request.playback_speed);
                continue;
            }

            motion_controller.push_motion_info(Arc::clone(motion_info), request.playback_speed);
        }

        true
    }

    pub fn get_or_load_motion(
        &mut self,
        assets: &mut AssetLoader,
        name: &str,
    ) -> Result<Arc<Motion>, AssetError> {
        let hash = crate::game::hash(name);

        if let Some(motion) = self.motions.get(&hash) {
            return Ok(Arc::clone(motion));
        }

        let path = PathBuf::from("motions").join(name).with_extension("bmf");

        let data = assets.load_raw(&path)?;

        let mut context = AssetLoadContext { loader: assets };
        let motion = Arc::new(Motion::from_memory(&mut context, path, &data)?);

        self.motions.insert(hash, Arc::clone(&motion));

        Ok(motion)
    }

    fn find_sequence(&self, hash: u32) -> Option<Arc<Sequence>> {
        self.sequences.get(&hash).cloned()
    }
}

enum ParseState {
    None,
    TransitionSequence {
        begin_state: State,
        end_state: State,
        name: String,
        motions: Vec<Arc<MotionInfo>>,
    },
    Sequence {
        name: String,
        motions: Vec<Arc<MotionInfo>>,
    },
}

impl MotionSequencer {
    pub fn load_motion_sequencer_defs(
        &mut self,
        assets: &mut AssetLoader,
        path: impl AsRef<Path>,
    ) -> Result<(), AssetError> {
        let data = assets.load_raw(path.as_ref())?;
        let text = String::from_utf8_lossy(&data);
        let config_lines = ConfigLines::parse(&text);
        self.parse(config_lines, assets)
    }

    fn parse(
        &mut self,
        config_lines: ConfigLines,
        assets: &mut AssetLoader,
    ) -> Result<(), AssetError> {
        let mut parse_state = ParseState::None;

        for line in config_lines.into_iter() {
            match line.key.as_str() {
                "BEGIN_TRANSITION_SEQ" => {
                    let current = std::mem::replace(&mut parse_state, ParseState::None);
                    self.commit_parse_state(current);

                    let begin_state_name = line.string(0);
                    let end_state_name = line.string(1);

                    let Ok(begin_state) = State::from_str(&begin_state_name) else {
                        tracing::warn!(
                            "Invalid BEGIN_TRANSITION_SEQ begin state label: {}",
                            begin_state_name
                        );
                        continue;
                    };

                    let Ok(end_state) = State::from_str(&end_state_name) else {
                        tracing::warn!(
                            "Invalid BEGIN_TRANSITION_SEQ end state label: {}",
                            end_state_name
                        );
                        continue;
                    };

                    parse_state = ParseState::TransitionSequence {
                        begin_state,
                        end_state,
                        name: line.string(2),
                        motions: Vec::default(),
                    };
                }

                "MOTION" => match &mut parse_state {
                    ParseState::None => {
                        tracing::warn!("No sequence available to set MOTION.");
                        continue;
                    }
                    ParseState::TransitionSequence { motions, .. }
                    | ParseState::Sequence { motions, .. } => {
                        let motion_name = line.string(0);

                        let Ok(motion) = self.get_or_load_motion(assets, &motion_name) else {
                            // Skip this motion info if the motion fails to load.
                            continue;
                        };

                        let mut immediate = false;
                        let mut looping = false;
                        let mut repeat_count = 0;

                        // [IMMEDIATE] [LOOP] [REPS=<count>]
                        for modifier in line.params().iter().skip(1) {
                            let modifier = String::from(modifier.clone());
                            match modifier.as_str() {
                                "IMMEDIATE" => immediate = true,
                                "LOOP" => looping = true,
                                s if s.starts_with("REPS=") || s.starts_with("REP=") => {
                                    repeat_count = s
                                        .split_once('=')
                                        .map(|(_, c)| c.parse::<i32>().unwrap_or_default())
                                        .unwrap_or_default();
                                }
                                _ => {}
                            }
                        }

                        let base_ticks_per_frame = motion.base_ticks_per_frame.max(1);

                        motions.push(Arc::new(MotionInfo {
                            hash: hash(motion.name.as_str()),
                            repeat_count,
                            looping,
                            motion,
                            // Keep current behavior aligned with original parsing for now:
                            // LOOP drives both playback wrapping intent and transition guard.
                            transition_guard: looping,
                            immediate,
                            enabled: true,
                            start_time_ticks: 0,
                            base_ticks_per_frame,
                        }));
                    }
                },

                "END_SEQUENCE" => {
                    let current = std::mem::replace(&mut parse_state, ParseState::None);
                    match current {
                        ParseState::None => tracing::warn!("No sequence to end."),
                        other => self.commit_parse_state(other),
                    }
                }

                "DEFAULT_COG_POSITION" => {
                    let state_name = line.string(0);

                    let position = Vec3::new(
                        line.param::<f32>(1),
                        line.param::<f32>(2),
                        line.param::<f32>(3),
                    );

                    if let Ok(state) = State::from_str(&state_name) {
                        self.default_cog_positions.insert(state, position);
                    } else {
                        tracing::warn!("Invalid DEFAULT_COG_POSITION state label: {}", state_name);
                    }
                }
                "BEGIN_SEQUENCE" => {
                    let current = std::mem::replace(&mut parse_state, ParseState::None);
                    self.commit_parse_state(current);

                    let name = line.string(0);
                    parse_state = ParseState::Sequence {
                        name,
                        motions: Vec::default(),
                    };
                }

                "CALLBACK" => {
                    tracing::debug!(
                        "Skipping CALLBACK parsing until callback support is implemented."
                    );
                }

                "DECLARE_Z_IND_MOTION" => {
                    let motion_name = line.string(0);
                    if motion_name.is_empty() {
                        tracing::warn!("DECLARE_Z_IND_MOTION missing motion name.");
                        continue;
                    }

                    if let Ok(motion) = self.get_or_load_motion(assets, &motion_name) {
                        motion.add_flags(MotionFlags::Z_IND_MOTION);
                    }
                }

                "DECLARE_SPED_MOTION" => {
                    let motion_name = line.string(0);
                    if motion_name.is_empty() {
                        tracing::warn!("DECLARE_SPED_MOTION missing motion name.");
                        continue;
                    }

                    if let Ok(motion) = self.get_or_load_motion(assets, &motion_name) {
                        motion.add_flags(MotionFlags::SPED_MOTION);
                    }
                }

                "DECLARE_SKIP_LAST_FRAME" => {
                    let motion_name = line.string(0);
                    if motion_name.is_empty() {
                        tracing::warn!("DECLARE_SKIP_LAST_FRAME missing motion name.");
                        continue;
                    }

                    if let Ok(motion) = self.get_or_load_motion(assets, &motion_name) {
                        motion.add_flags(MotionFlags::SKIP_LAST_FRAME);
                    }
                }

                "DECLARE_NO_LVE_MOTION" => {
                    let motion_name = line.string(0);
                    if motion_name.is_empty() {
                        tracing::warn!("DECLARE_NO_LVE_MOTION missing motion name.");
                        continue;
                    }

                    if let Ok(motion) = self.get_or_load_motion(assets, &motion_name) {
                        motion.add_flags(MotionFlags::NO_LVE_MOTION);
                    }
                }
                "::" => {
                    // Due to an error in the game config file, "::" is used as a
                    // heading marker, which is wrong, so we just ignore it here.
                }

                key => {
                    tracing::warn!("Invalid key for sequence definitions: {key}");
                    continue;
                }
            }
        }

        self.commit_parse_state(parse_state);

        Ok(())
    }

    fn create_sequence(
        name: String,
        begin_state: State,
        end_state: State,
        motions: Vec<Arc<MotionInfo>>,
    ) -> Arc<Sequence> {
        Arc::new(Sequence {
            hash: hash(name.as_str()),
            name,
            begin_state,
            end_state,
            motions,
        })
    }

    fn commit_parse_state(&mut self, parse_state: ParseState) {
        match parse_state {
            ParseState::None => {}
            ParseState::TransitionSequence {
                begin_state,
                end_state,
                name,
                motions,
            } => {
                self.transition_sequences.insert(
                    (begin_state, end_state),
                    Self::create_sequence(name, begin_state, end_state, motions),
                );
            }
            ParseState::Sequence { name, motions } => {
                let (begin_state, end_state) = Self::infer_sequence_state_bounds(&motions);
                self.sequences.insert(
                    hash(name.as_str()),
                    Self::create_sequence(name, begin_state, end_state, motions),
                );
            }
        }
    }

    fn infer_sequence_state_bounds(motions: &[Arc<MotionInfo>]) -> (State, State) {
        let Some(first_motion) = motions.first() else {
            return (State::None, State::None);
        };

        let begin_state = first_motion.motion.from_state;
        let end_state = motions
            .last()
            .map(|motion_info| motion_info.motion.to_state)
            .unwrap_or(begin_state);
        (begin_state, end_state)
    }
}
