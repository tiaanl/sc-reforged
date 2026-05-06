use bevy_ecs::prelude::*;

use crate::game::sim::ecs::GizmoVertices;

pub fn clear_gizmo_vertices(mut gizmo_vertices: ResMut<GizmoVertices>) {
    gizmo_vertices.clear();
}
