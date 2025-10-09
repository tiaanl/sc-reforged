use crate::{
    engine::{gizmos::GizmosRenderer, prelude::*},
    game::scenes::world::sim_world::SimWorld,
};

pub struct ObjectsSystem {}

impl ObjectsSystem {
    pub fn new(_renderer: &Renderer) -> Self {
        Self {}
    }

    pub fn render_gizmos(&self, sim_world: &mut SimWorld) {
        for (_, object) in sim_world.objects.objects.iter() {
            sim_world
                .gizmo_vertices
                .extend(GizmosRenderer::create_iso_sphere(
                    object.transform.to_mat4(),
                    object.bounding_sphere.radius,
                    6,
                ));
        }
    }
}
