use bevy_ecs::prelude::*;

use super::*;

#[derive(Debug)]
pub struct MoveToAttackOrder {
    pub entity: Entity,
}

impl RuntimeOrder for MoveToAttackOrder {
    fn execute(&mut self) -> ExecuteOutcome {
        ExecuteOutcome::Complete
    }
}
