use glam::Vec3;

pub enum PlayerAction {
    /// User clicked on the given object, at the given position.
    Object { position: Vec3, id: u32 },
    /// User clicked on the terrain at the given position.
    Terrain { position: Vec3 },
}
