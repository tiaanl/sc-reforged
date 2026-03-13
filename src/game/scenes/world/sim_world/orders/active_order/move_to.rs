use glam::Vec3;

use super::*;

#[derive(Debug)]
pub struct MoveToOrder {
    pub location: Vec3,
}

impl RuntimeOrder for MoveToOrder {
    fn execute(&mut self) -> ExecuteOutcome {
        ExecuteOutcome::Complete
    }
}
