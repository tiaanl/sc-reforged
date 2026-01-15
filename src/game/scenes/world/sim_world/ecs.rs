#![allow(dead_code)]

use bevy_ecs::prelude::*;
use glam::UVec2;

use crate::{
    engine::gizmos::GizmoVertex,
    game::{
        math::BoundingBox,
        scenes::world::{
            render::{BoxRenderSnapshot, ModelRenderSnapshot, TerrainRenderSnapshot},
            systems::camera_system::CameraEnvSnapshot,
        },
    },
};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum UpdateSet {
    Input,
    Update,
}

#[derive(Component)]
pub struct ActiveCamera;

#[derive(Component)]
pub struct BoundingBoxComponent(pub BoundingBox);

#[derive(Resource)]
pub struct GizmoVertices(Vec<GizmoVertex>);

impl GizmoVertices {
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub fn swap(&mut self, other: &mut Vec<GizmoVertex>) {
        std::mem::swap(&mut self.0, other);
    }

    #[inline]
    pub fn push(&mut self, vertex: GizmoVertex) {
        self.0.push(vertex)
    }

    #[inline]
    pub fn extend(&mut self, iter: impl IntoIterator<Item = GizmoVertex>) {
        self.0.extend(iter)
    }
}

#[derive(Default, Resource)]
pub struct Viewport {
    pub size: UVec2,
}

impl Viewport {
    #[inline]
    pub fn resize(&mut self, size: UVec2) {
        self.size = size;
    }
}

#[derive(Default, Resource)]
pub struct Snapshots {
    pub camera_env_snapshot: CameraEnvSnapshot,

    pub box_render_snapshot: BoxRenderSnapshot,
    pub terrain_render_snapshot: TerrainRenderSnapshot,
    pub model_render_snapshot: ModelRenderSnapshot,
}

impl Snapshots {
    pub fn clear(&mut self) {
        self.box_render_snapshot.clear();
    }
}
