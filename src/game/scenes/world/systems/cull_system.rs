use super::{PreUpdateContext, System};

/// Calculate visible elements for the current frame.
pub struct CullSystem;

impl System for CullSystem {
    fn pre_update(&mut self, context: &mut PreUpdateContext) {
        let sim_world = &mut context.sim_world;

        let frustum = &sim_world.computed_camera.frustum;

        sim_world.visible_chunks.clear();

        sim_world.quad_tree.with_nodes_in_frustum(frustum, |node| {
            if let Some(chunk_coord) = node.chunk_coord {
                sim_world.visible_chunks.push(chunk_coord);
            }
        });
    }
}
