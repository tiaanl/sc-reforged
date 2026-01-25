use bevy_ecs::prelude::*;

use crate::game::scenes::world::{extract::RenderSnapshot, sim_world::ecs::GizmoVertices};

pub fn extract_gizmos(mut snapshot: ResMut<RenderSnapshot>, gizmo_vertices: Res<GizmoVertices>) {
    snapshot.gizmos.vertices = gizmo_vertices.vertices.clone();
}
