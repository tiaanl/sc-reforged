#![allow(dead_code)]

use bevy_ecs::prelude::*;
use glam::{UVec2, Vec3, Vec4};

use crate::{
    engine::gizmos::{GizmoVertex, create_bounding_box},
    game::math::BoundingBox,
};

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

    pub fn draw_line(&mut self, start: Vec3, end: Vec3, color: Vec4) {
        self.vertices
            .extend_from_slice(&[GizmoVertex::new(start, color), GizmoVertex::new(end, color)]);
    }

    pub fn draw_bounding_box(&mut self, bounding_box: &BoundingBox, color: Vec4) {
        self.vertices
            .extend(create_bounding_box(bounding_box, color));
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
