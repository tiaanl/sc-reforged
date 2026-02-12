use std::sync::Arc;

use super::motion::Motion;

#[derive(Clone, Debug)]
pub struct MotionInfo {
    pub hash: u32,
    pub motion: Arc<Motion>,

    pub repeat_count: i32,
    pub looping: bool,
    pub transition_guard: bool,
    pub immediate: bool,
    pub enabled: bool,

    pub start_time_ticks: u32,
    pub base_ticks_per_frame: u32,
}
