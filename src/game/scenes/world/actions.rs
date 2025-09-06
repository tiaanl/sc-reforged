use bevy_ecs::prelude as ecs;
use glam::Vec3;

#[derive(Clone, Copy, ecs::Event)]
pub enum PlayerAction {
    /// User asked to clear selection.
    ClearSelection,
    /// User clicked on the given object, at the given position.
    ObjectClicked { position: Vec3, id: u32 },
    /// User clicked on the terrain at the given position.
    TerrainClicked { position: Vec3 },
}
