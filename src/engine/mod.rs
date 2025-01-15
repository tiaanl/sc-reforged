pub mod assets;
pub mod depth_buffer;
pub mod egui_integration;
pub mod gizmos;
pub mod input;
pub mod mesh;
pub mod renderer;
pub mod scene;
pub mod shaders;
pub mod utils;

pub mod buffers;
pub mod dirty;
pub mod tracked;
pub mod transform;

pub mod prelude {
    #![allow(unused_imports)]
    pub use super::assets::{Asset, AssetStore, Handle};
    pub use super::buffers::*;
    pub use super::depth_buffer::*;
    pub use super::dirty::*;
    pub use super::input::*;
    pub use super::mesh::*;
    pub use super::renderer::*;
    pub use super::scene::*;
    pub use super::shaders::*;
    pub use super::tracked::*;
    pub use super::transform::*;
    pub use glam::{Mat4, Quat, Vec2, Vec3};
}

mod mip_maps;
