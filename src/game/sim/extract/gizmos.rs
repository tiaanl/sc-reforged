use bevy_ecs::prelude::*;

use crate::game::{
    render::world::WorldRenderSnapshot, sim::ecs::GizmoVertices,
};

pub fn extract_gizmos(
    mut snapshot: ResMut<WorldRenderSnapshot>,
    gizmo_vertices: Res<GizmoVertices>,
) {
    snapshot.gizmos.vertices = gizmo_vertices.vertices.clone();
}
