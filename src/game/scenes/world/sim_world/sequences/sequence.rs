use std::sync::Arc;

use super::{motion_info::MotionInfo, state::State};

#[derive(Debug)]
pub struct Sequence {
    pub name: String,
    pub hash: u32,
    pub begin_state: State,
    pub end_state: State,
    pub motions: Vec<Arc<MotionInfo>>,
}
