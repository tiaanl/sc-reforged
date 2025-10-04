use crate::game::scenes::world::systems::{System, UpdateContext};

/// Calculate visible elements for the current frame.
pub struct CullSystem;

impl System for CullSystem {
    fn update(&mut self, context: &mut UpdateContext) {}
}
