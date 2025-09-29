use glam::{IVec2, UVec2, Vec2, Vec3, Vec4};
use slab::Slab;

use crate::{
    engine::gizmos::GizmoVertex,
    game::{
        math::{BoundingBox, Frustum},
        scenes::world::terrain::Terrain,
    },
};

pub type NodeId = usize;

#[derive(Debug)]
struct Node {
    /// Minimum X and Y for this [Node].
    min: Vec2,
    /// Maximum X and Y for this [Node].
    max: Vec2,

    /// Minimum height for this [Node].
    min_z: f32,
    /// Maximum height for this [Node].
    max_z: f32,

    children: [Option<NodeId>; 4],

    /// If this is a leaf node, stores the terrain chunk index it wraps.
    _chunk_index: Option<usize>,

    /// Set to true if this is a leaf node.
    is_leaf: bool,

    _level: u32,
}

impl Node {
    fn bounding_box(&self) -> BoundingBox {
        BoundingBox {
            min: self.min.extend(self.min_z),
            max: self.max.extend(self.max_z),
        }
    }
}

#[derive(Default)]
pub struct QuadTree {
    nodes: Slab<Node>,
    root: NodeId,
    /// Size of the terrain in chunks.
    chunks_size: UVec2,
}

impl QuadTree {
    pub fn from_terrain(terrain: &Terrain) -> Self {
        let chunks_size = terrain.height_map.size / Terrain::CELLS_PER_CHUNK;

        let mut result = Self {
            chunks_size,
            ..Default::default()
        };

        result.root = result.build_node(
            UVec2::ZERO,
            chunks_size,
            terrain.nominal_edge_size,
            0,
            &|chunk_min, chunk_max| {
                // TODO: Cache the results of these calculations.

                let cell_min = chunk_min * Terrain::CELLS_PER_CHUNK;
                let cell_max = chunk_max * Terrain::CELLS_PER_CHUNK;

                let mut min_z = f32::INFINITY;
                let mut max_z = f32::NEG_INFINITY;

                for y in cell_min.y..=cell_max.y {
                    for x in cell_min.x..=cell_max.x {
                        let z = terrain
                            .height_map
                            .node_elevation(IVec2::new(x as i32, y as i32));
                        min_z = min_z.min(z);
                        max_z = max_z.max(z);
                    }
                }

                (min_z, max_z)
            },
        );

        result
    }

    pub fn _chunk_indices_in_frustum(&self, frustum: &Frustum) -> Vec<usize> {
        let mut out = Vec::new();
        self._collect_visible_chunks(self.root, frustum, &mut out);
        out
    }

    fn _collect_visible_chunks(&self, node_id: NodeId, frustum: &Frustum, out: &mut Vec<usize>) {
        let node = &self.nodes[node_id];

        let aabb = BoundingBox {
            min: node.min.extend(node.min_z),
            max: node.max.extend(node.max_z),
        };

        if !frustum.intersects_bounding_box(&aabb) {
            return;
        }

        if node.is_leaf {
            if let Some(idx) = node._chunk_index {
                out.push(idx);
            }
        } else {
            for child_id in node.children.iter().flatten() {
                self._collect_visible_chunks(*child_id, frustum, out);
            }
        }
    }

    fn build_node(
        &mut self,
        chunk_min: UVec2,
        chunk_max: UVec2,
        nominal_edge_size: f32,
        level: u32,
        height_for: &dyn Fn(UVec2, UVec2) -> (f32, f32),
    ) -> NodeId {
        let size = UVec2::new(
            (chunk_max.x - chunk_min.x).max(1),
            (chunk_max.y - chunk_min.y).max(1),
        );

        let is_leaf = size.x == 1 && size.y == 1;

        let min = Vec2::new(chunk_min.x as f32, chunk_min.y as f32)
            * Terrain::CELLS_PER_CHUNK as f32
            * nominal_edge_size;
        let max = Vec2::new(chunk_max.x as f32, chunk_max.y as f32)
            * Terrain::CELLS_PER_CHUNK as f32
            * nominal_edge_size;

        let (min_z, max_z) = height_for(chunk_min, chunk_max);

        let mut node = Node {
            min,
            max,
            min_z,
            max_z,
            children: [None, None, None, None],
            _chunk_index: if is_leaf {
                Some((chunk_min.y * self.chunks_size.x + chunk_min.x) as usize)
            } else {
                None
            },
            is_leaf,
            _level: level,
        };

        if !is_leaf {
            let mid = (chunk_min + chunk_max) / 2;

            let child_rects = [
                // Top-left
                (
                    UVec2::new(chunk_min.x, mid.y),
                    UVec2::new(mid.x, chunk_max.y),
                ),
                // Top-right
                (
                    UVec2::new(mid.x, mid.y),
                    UVec2::new(chunk_max.x, chunk_max.y),
                ),
                // Bottom-left
                (
                    UVec2::new(chunk_min.x, chunk_min.y),
                    UVec2::new(mid.x, mid.y),
                ),
                // Bottom-right
                (
                    UVec2::new(mid.x, chunk_min.y),
                    UVec2::new(chunk_max.x, mid.y),
                ),
            ];

            for (i, child) in node.children.iter_mut().enumerate() {
                let (chunk_min, chunk_max) = child_rects[i];
                let child_id = self.build_node(
                    chunk_min,
                    chunk_max,
                    nominal_edge_size,
                    level + 1,
                    height_for,
                );
                *child = Some(child_id);
            }
        }

        self.nodes.insert(node)
    }

    fn render_node(&self, node: &Node, gizmo_vertices: &mut Vec<GizmoVertex>) {
        let color_min = Vec4::new(1.0, 0.0, 0.0, 1.0);
        let color_max = Vec4::new(0.0, 0.0, 1.0, 1.0);

        let vertices = [
            // min
            Vec3::new(node.min.x, node.min.y, node.min_z),
            Vec3::new(node.max.x, node.min.y, node.min_z),
            Vec3::new(node.max.x, node.max.y, node.min_z),
            Vec3::new(node.min.x, node.max.y, node.min_z),
            // max
            Vec3::new(node.min.x, node.min.y, node.max_z),
            Vec3::new(node.max.x, node.min.y, node.max_z),
            Vec3::new(node.max.x, node.max.y, node.max_z),
            Vec3::new(node.min.x, node.max.y, node.max_z),
        ];

        gizmo_vertices.push(GizmoVertex::new(vertices[0], color_min));
        gizmo_vertices.push(GizmoVertex::new(vertices[1], color_min));

        gizmo_vertices.push(GizmoVertex::new(vertices[1], color_min));
        gizmo_vertices.push(GizmoVertex::new(vertices[2], color_min));

        gizmo_vertices.push(GizmoVertex::new(vertices[2], color_min));
        gizmo_vertices.push(GizmoVertex::new(vertices[3], color_min));

        gizmo_vertices.push(GizmoVertex::new(vertices[3], color_min));
        gizmo_vertices.push(GizmoVertex::new(vertices[0], color_min));

        gizmo_vertices.push(GizmoVertex::new(vertices[4], color_max));
        gizmo_vertices.push(GizmoVertex::new(vertices[5], color_max));

        gizmo_vertices.push(GizmoVertex::new(vertices[5], color_max));
        gizmo_vertices.push(GizmoVertex::new(vertices[6], color_max));

        gizmo_vertices.push(GizmoVertex::new(vertices[6], color_max));
        gizmo_vertices.push(GizmoVertex::new(vertices[7], color_max));

        gizmo_vertices.push(GizmoVertex::new(vertices[7], color_max));
        gizmo_vertices.push(GizmoVertex::new(vertices[4], color_max));

        gizmo_vertices.push(GizmoVertex::new(vertices[0], color_min));
        gizmo_vertices.push(GizmoVertex::new(vertices[4], color_max));

        gizmo_vertices.push(GizmoVertex::new(vertices[1], color_min));
        gizmo_vertices.push(GizmoVertex::new(vertices[5], color_max));

        gizmo_vertices.push(GizmoVertex::new(vertices[2], color_min));
        gizmo_vertices.push(GizmoVertex::new(vertices[6], color_max));

        gizmo_vertices.push(GizmoVertex::new(vertices[3], color_min));
        gizmo_vertices.push(GizmoVertex::new(vertices[7], color_max));
    }

    pub fn _render_gizmos(&self, gizmo_vertices: &mut Vec<GizmoVertex>) {
        let node = self.nodes.get(self.root).unwrap();

        if node.is_leaf {
            self.render_node(node, gizmo_vertices);
        }

        node.children
            .iter()
            .filter_map(|child_id| *child_id)
            .filter_map(|child_id| self.nodes.get(child_id))
            .for_each(|node| self.render_node(node, gizmo_vertices));
    }

    pub fn render_gizmos_in_frustum(
        &self,
        frustum: &Frustum,
        gizmo_vertices: &mut Vec<GizmoVertex>,
    ) {
        for (_, node) in self.nodes.iter() {
            if !frustum.intersects_bounding_box(&node.bounding_box()) {
                continue;
            }
            self.render_node(node, gizmo_vertices);
        }
    }
}
