use bevy_ecs::prelude::*;

use crate::game::scenes::world::sim_world::ecs::GizmoVertices;

pub fn clear_gizmo_vertices(mut gizmo_vertices: ResMut<GizmoVertices>) {
    gizmo_vertices.clear();
}
