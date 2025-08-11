#![allow(unused)]

use ahash::HashMap;
use glam::Vec3;

use crate::game::config::parser::ConfigLines;

#[derive(Debug)]
pub struct TransitionSequence {
    pub from_state: String,
    pub to_state: String,
    pub name: String,
    pub motion: String,
}

#[derive(Debug)]
pub enum Callback {
    NotifyEnd,
    Frame { name: String, frame: i32 },
}

#[derive(Debug, Default)]
pub struct Motion {
    name: String,
    is_immediate: bool,
    is_loop: bool,
    rep: Option<i32>,
    callbacks: Vec<Callback>,
}

impl Motion {
    pub fn from_name(name: String) -> Self {
        Self {
            name,
            ..Self::default()
        }
    }
}

#[derive(Debug, Default)]
pub struct Sequence {
    name: String,
    motions: Vec<Motion>,
}

impl Sequence {
    pub fn from_name(name: String) -> Self {
        Self {
            name,
            motions: Vec::default(),
        }
    }
}

#[derive(Debug, Default)]
pub struct MotionSequencesDefs {
    pub transition_sequences: Vec<TransitionSequence>,
    pub sequences: Vec<Sequence>,
    pub default_cog_positions: HashMap<String, Vec3>,
    pub z_index_motions: Vec<String>,
    pub sped_motions: Vec<String>,
    pub no_live_motions: Vec<String>,
    pub skip_last_frame_motions: Vec<String>,
}

impl From<ConfigLines> for MotionSequencesDefs {
    fn from(value: ConfigLines) -> Self {
        let mut motion_sequence_defs = MotionSequencesDefs::default();

        let mut transition_sequence: Option<TransitionSequence> = None;
        let mut sequence: Option<Sequence> = None;

        for line in value.into_iter() {
            match line.key.as_str() {
                "BEGIN_TRANSITION_SEQ" => {
                    if let Some(transition_sequence) = transition_sequence.take() {
                        motion_sequence_defs
                            .transition_sequences
                            .push(transition_sequence);
                    }

                    transition_sequence = Some(TransitionSequence {
                        from_state: line.string(0),
                        to_state: line.string(1),
                        name: line.string(2),
                        motion: Default::default(),
                    });
                }

                "MOTION" => {
                    if let Some(ref mut transition_sequence) = transition_sequence {
                        transition_sequence.motion = line.string(0);
                    } else if let Some(ref mut sequence) = sequence {
                        let mut motion = Motion::from_name(line.string(0));

                        // [IMMEDIATE] [LOOP] [REP=<count>]
                        match line.string(1).as_str() {
                            "IMMEDIATE" => motion.is_immediate = true,
                            "LOOP" => motion.is_loop = true,
                            s if s.starts_with("REP") => {
                                let rep = s
                                    .split("=")
                                    .nth(1)
                                    .unwrap()
                                    .parse::<i32>()
                                    .unwrap_or_default();
                                motion.rep = Some(rep);
                            }
                            _ => {}
                        }

                        sequence.motions.push(motion);
                    } else {
                        panic!("No active motion sequence for MOTION");
                    }
                }

                "END_SEQUENCE" => {
                    if let Some(transition_sequence) = transition_sequence.take() {
                        motion_sequence_defs
                            .transition_sequences
                            .push(transition_sequence);
                    } else if let Some(sequence) = sequence.take() {
                        motion_sequence_defs.sequences.push(sequence);
                    } else {
                        panic!("No active motion sequence for END_SEQUENCE");
                    }
                }

                "DEFAULT_COG_POSITION" => {
                    let name = line.string(0);
                    let position = Vec3::new(line.param(1), line.param(2), line.param(3));
                    motion_sequence_defs
                        .default_cog_positions
                        .insert(name, position);
                }

                "BEGIN_SEQUENCE" => {
                    if let Some(sequence) = sequence.take() {
                        motion_sequence_defs.sequences.push(sequence);
                    }

                    sequence = Some(Sequence::from_name(line.string(0)));
                }

                "CALLBACK" => {
                    if let Some(ref mut sequence) = sequence {
                        let motion = sequence
                            .motions
                            .last_mut()
                            .expect("Callback without a Motion");

                        motion.callbacks.push(if line.string(0) == "NOTIFY_END" {
                            Callback::NotifyEnd
                        } else {
                            Callback::Frame {
                                name: line.param(1),
                                frame: line.param(0),
                            }
                        });
                    }
                }

                "DECLARE_Z_IND_MOTION" => {
                    motion_sequence_defs.z_index_motions.push(line.string(0));
                }

                "DECLARE_SPED_MOTION" => {
                    motion_sequence_defs.sped_motions.push(line.string(0));
                }

                "DECLARE_NO_LVE_MOTION" => {
                    motion_sequence_defs.no_live_motions.push(line.string(0));
                }

                "DECLARE_SKIP_LAST_FRAME" => {
                    motion_sequence_defs
                        .skip_last_frame_motions
                        .push(line.string(0));
                }

                "::" => {
                    // Due to an error in the game config file, "::" is used as a
                    // heading marker, which is wrong, so we just ignore it here.
                }

                _ => panic!("Invalid key for MotionSequencesDefs: {}", line.key),
            }
        }

        motion_sequence_defs
    }
}
