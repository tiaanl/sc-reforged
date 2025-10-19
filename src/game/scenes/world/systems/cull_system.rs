use glam::vec4;

use crate::{engine::gizmos, game::scenes::world::sim_world::SimWorld};

#[derive(Default)]
pub enum DebugQuadTreeOptions {
    /// No debugging.
    #[default]
    None,
    /// Only render gizmos at the specified level.
    Level(usize),
    /// Render gizmos for *all* nodes.
    All,
}

/// Calculate visible elements for the current frame.
#[derive(Default)]
pub struct CullSystem {
    pub debug_quad_tree: DebugQuadTreeOptions,
}

impl CullSystem {
    pub fn calculate_visible_chunks(&mut self, sim_world: &mut SimWorld) {
        let frustum = &sim_world.computed_camera.frustum;

        sim_world.visible_chunks.clear();
        sim_world.visible_objects.clear();

        sim_world.quad_tree.with_nodes_in_frustum(frustum, |node| {
            if let Some(chunk_coord) = node.chunk_coord {
                sim_world.visible_chunks.push(chunk_coord);
            }

            sim_world
                .visible_objects
                .extend(node.objects.iter().map(|entry| entry.handle));
        });

        self.debug_quad_tree(sim_world);
    }

    fn debug_quad_tree(&self, sim_world: &mut SimWorld) {
        let frustum = &sim_world.computed_camera.frustum;

        match self.debug_quad_tree {
            DebugQuadTreeOptions::None => {}
            DebugQuadTreeOptions::Level(level) => {
                sim_world.quad_tree.with_nodes_in_frustum(frustum, |node| {
                    if node.level != level {
                        return;
                    }

                    let bounding_box = node.bounding_box();

                    sim_world.gizmo_vertices.extend(gizmos::create_bounding_box(
                        &bounding_box,
                        vec4(1.0, 0.0, 0.0, 1.0),
                    ));
                })
            }
            DebugQuadTreeOptions::All => {
                sim_world.quad_tree.with_nodes_in_frustum(frustum, |node| {
                    let bounding_box = node.bounding_box();

                    sim_world.gizmo_vertices.extend(gizmos::create_bounding_box(
                        &bounding_box,
                        vec4(0.0, 1.0, 0.0, 1.0),
                    ));
                })
            }
        }
    }
}
