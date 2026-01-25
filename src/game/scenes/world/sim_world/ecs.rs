#![allow(dead_code)]

use bevy_ecs::prelude::*;
use glam::UVec2;

use crate::{engine::gizmos::GizmoVertex, game::math::BoundingBox};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum UpdateSet {
    Start,
    Input,
    Update,
}

#[derive(Component)]
pub struct ActiveCamera;

#[derive(Component)]
pub struct BoundingBoxComponent(pub BoundingBox);

#[derive(Resource)]
pub struct GizmoVertices {
    pub vertices: Vec<GizmoVertex>,
}

impl GizmoVertices {
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            vertices: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.vertices.clear()
    }

    #[inline]
    pub fn push(&mut self, vertex: GizmoVertex) {
        self.vertices.push(vertex)
    }

    #[inline]
    pub fn extend(&mut self, iter: impl IntoIterator<Item = GizmoVertex>) {
        self.vertices.extend(iter)
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
