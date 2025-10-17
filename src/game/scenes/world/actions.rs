#![allow(unused)]

use glam::Vec3;

#[derive(Clone, Copy)]
pub enum PlayerAction {
    /// User asked to clear selection.
    ClearSelection,
    /// User clicked on the given object, at the given position.
    ObjectClicked { _position: Vec3, id: u32 },
    /// User clicked on the terrain at the given position.
    TerrainClicked { _position: Vec3 },
}
