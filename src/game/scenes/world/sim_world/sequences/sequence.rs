use std::sync::Arc;

use super::{motion_info::MotionInfo, state::State};

#[derive(Debug)]
pub struct Sequence {
    pub _name: String,
    pub _hash: u32,
    pub begin_state: State,
    pub _end_state: State,
    pub motions: Vec<Arc<MotionInfo>>,
}
