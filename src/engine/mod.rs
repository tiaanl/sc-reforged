pub mod assets;
pub mod gizmos;
mod global;
pub mod input;
pub mod mesh;
pub mod renderer;
pub mod scene;
pub mod shaders;
pub mod storage;

pub mod dirty;
pub mod growing_buffer;
pub mod tracked;
pub mod transform;

pub mod prelude {
    pub use super::assets::*;
    pub use super::dirty::*;
    pub use super::input::*;
    pub use super::mesh::*;
    pub use super::renderer::*;
    pub use super::scene::*;
    pub use super::transform::*;
    pub use glam::{Mat4, Quat, UVec2, Vec2, Vec3};
}

#[cfg(feature = "egui")]
pub mod egui_integration;
