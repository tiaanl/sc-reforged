use glam::Vec3;

pub enum Order {
    MoveTo { location: Vec3 },
}

pub enum ExecuteOutcome {
    /// The order should continue executing.
    Executing,
    /// The order is complete.
    Complete,
}

impl Order {
    pub fn execute(&mut self) -> ExecuteOutcome {
        ExecuteOutcome::Complete
    }
}
