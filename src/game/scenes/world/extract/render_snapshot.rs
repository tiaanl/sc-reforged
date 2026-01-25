use bevy_ecs::prelude::*;
use glam::{IVec2, Mat4, Vec2, Vec3, Vec4};

use crate::{
    engine::{gizmos::GizmoVertex, storage::Handle},
    game::{math::Frustum, model::Model},
};

/// Camera information.
#[derive(Default)]
pub struct Camera {
    /// Position of the camera.
    pub position: Vec3,
    /// Forward direction of the camera.
    pub forward: Vec3,
    /// Near clip distance.
    pub _near: f32,
    /// Far clip distance.
    pub far: f32,
    /// Calculated projection -> view matrix.
    pub proj_view: Mat4,
    /// The frustum of the calculated `proj_view` matrix.
    pub frustum: Frustum,
}

/// Details about the environment being rendered.
#[derive(Default)]
pub struct Environment {
    /// The elapsed time in seconds since the simulation started.
    pub sim_time: f32,

    /// Direction of the sun.
    pub sun_dir: Vec3,
    /// Color of the sun.
    pub sun_color: Vec3,
    /// Ambient term for lighting.
    pub ambient_color: Vec3,

    /// Color of the fog.
    pub fog_color: Vec3,
    /// Far distance of the fog.
    pub fog_distance: f32,
    /// Near distance for the fog calculation as a fraction of the far distance.
    pub fog_near_fraction: f32,
}

#[derive(Default)]
pub struct Terrain {
    pub chunks: Vec<TerrainChunk>,
    pub strata: Vec<TerrainChunk>,
    pub strata_side_count: [u32; 4],
}

#[derive(Clone, Copy, Default)]
pub struct TerrainChunk {
    pub coord: IVec2,
    pub lod: u32,
    pub flags: u32,
}

#[derive(Default)]
pub struct Models {
    /// A list of any new models that needs to be prepared before rendering.
    pub models_to_prepare: Vec<Handle<Model>>,
    /// A list of models to render.
    pub models: Vec<ModelToRender>,
}

#[derive(Clone)]
pub struct ModelToRender {
    pub model: Handle<Model>,
    pub transform: Mat4,
    pub highlighted: bool,
}

pub struct UiRect {
    pub min: Vec2,
    pub max: Vec2,
    pub color: Vec4,
}

#[derive(Default)]
pub struct Ui {
    pub proj_view: Mat4,
    pub ui_rects: Vec<UiRect>,
}

#[derive(Default)]
pub struct Gizmos {
    pub vertices: Vec<GizmoVertex>,
}

#[derive(Default, Resource)]
pub struct RenderSnapshot {
    /// The camera we are rendering the scene from.
    pub camera: Camera,
    /// The environment details.
    pub environment: Environment,
    /// Terrain to render.
    pub terrain: Terrain,
    /// Models to render.
    pub models: Models,
    /// UI to render.
    pub ui: Ui,
    /// Gizmos to render.
    pub gizmos: Gizmos,
}
