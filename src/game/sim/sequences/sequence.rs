use std::sync::Arc;

use crate::game::assets::motion::State;

use super::motion_info::MotionInfo;

#[derive(Debug)]
#[allow(dead_code)]
pub struct Sequence {
    pub name: String,
    pub hash: u32,
    pub begin_state: State,
    pub end_state: State,
    pub motions: Vec<Arc<MotionInfo>>,
}
