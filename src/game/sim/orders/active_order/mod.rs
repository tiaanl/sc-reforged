pub mod move_to;
pub mod move_to_attack;

pub trait RuntimeOrder {
    fn execute(&mut self) -> ExecuteOutcome;
}

#[derive(Debug)]
pub enum ActiveOrder {
    MoveTo(move_to::MoveToOrder),
    MoveToAttack(move_to_attack::MoveToAttackOrder),
}

impl RuntimeOrder for ActiveOrder {
    fn execute(&mut self) -> ExecuteOutcome {
        match self {
            ActiveOrder::MoveTo(order) => order.execute(),
            ActiveOrder::MoveToAttack(order) => order.execute(),
        }
    }
}

pub enum ExecuteOutcome {
    /// The order should continue executing.
    Executing,
    /// The order is complete.
    Complete,
}

impl ActiveOrder {
    /// Executes the active order and returns its outcome.
    pub fn execute(&mut self) -> ExecuteOutcome {
        match self {
            ActiveOrder::MoveTo(order) => order.execute(),
            ActiveOrder::MoveToAttack(order) => order.execute(),
        }
    }
}
