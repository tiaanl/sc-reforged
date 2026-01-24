use bevy_ecs::prelude::*;

use crate::game::scenes::world::{render::GizmoRenderSnapshot, sim_world::ecs::GizmoVertices};

#[derive(Default)]
pub struct GizmoExtract {}

impl GizmoExtract {
    pub fn world(_sim_world: &mut World) -> Self {
        Self {}
    }

    pub fn extract(&mut self, world: &mut World, snapshot: &mut GizmoRenderSnapshot) {
        snapshot.vertices.clear();

        world
            .resource_mut::<GizmoVertices>()
            .swap(&mut snapshot.vertices);
    }
}
