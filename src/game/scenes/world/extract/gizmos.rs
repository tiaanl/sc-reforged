use crate::game::scenes::world::{
    render::GizmoRenderSnapshot,
    sim_world::{SimWorld, ecs::GizmoVertices},
};

#[derive(Default)]
pub struct GizmoExtract {}

impl GizmoExtract {
    pub fn extract(&mut self, sim_world: &mut SimWorld, snapshot: &mut GizmoRenderSnapshot) {
        snapshot.vertices.clear();

        sim_world
            .ecs
            .resource_mut::<GizmoVertices>()
            .swap(&mut snapshot.vertices);
    }
}
