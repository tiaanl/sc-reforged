#![allow(dead_code)]

use bevy_ecs::prelude::*;
use glam::UVec2;

use crate::{engine::gizmos::GizmoVertex, game::math::BoundingBox};

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
