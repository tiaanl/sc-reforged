use glam::vec4;

use crate::{engine::gizmos::GizmosRenderer, game::scenes::world::sim_world::SimWorld};

/// Calculate visible elements for the current frame.
#[derive(Default)]
pub struct CullSystem {
    debug_visible_terrain_chunks: bool,
}

impl CullSystem {
    pub fn calculate_visible_chunks(&mut self, sim_world: &mut SimWorld) {
        let frustum = &sim_world.computed_camera.frustum;

        sim_world.visible_chunks.clear();

        sim_world.quad_tree.with_nodes_in_frustum(frustum, |node| {
            if let Some(chunk_coord) = node.chunk_coord {
                sim_world.visible_chunks.push(chunk_coord);
            }
        });

        if self.debug_visible_terrain_chunks {
            sim_world.quad_tree.with_nodes_in_frustum(frustum, |node| {
                let bb = node.bounding_box();

                sim_world
                    .gizmo_vertices
                    .extend(GizmosRenderer::create_bounding_box(
                        &bb,
                        vec4(1.0, 0.0, 0.0, 1.0),
                    ));
            });
        }

        sim_world.visible_objects = sim_world
            .objects
            .objects
            .iter()
            .map(|(handle, _)| handle)
            .collect();
    }
}
