use glam::{UVec2, Vec2, Vec3, Vec4, uvec2};
use slab::Slab;

use crate::{
    engine::gizmos::GizmoVertex,
    game::{
        math::{BoundingBox, Frustum},
        scenes::world::new_terrain::NewTerrain,
    },
};

pub type NodeId = usize;

#[derive(Debug)]
pub struct Node {
    /// Minimum X and Y for this [Node].
    min: Vec2,
    /// Maximum X and Y for this [Node].
    max: Vec2,
    /// Minimum height for this [Node].
    min_z: f32,
    /// Maximum height for this [Node].
    max_z: f32,
    /// If the node is a leaf (no children).
    pub is_leaf: bool,
    /// Each of the possible 4 children that makes up the quad tree.
    children: [Option<NodeId>; 4],
    /// If this node is a leaf and wraps a single terrain chunk, it holds the chunk coord.
    pub chunk_coord: Option<UVec2>,
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
}

impl QuadTree {
    pub fn from_new_terrain(terrain: &NewTerrain) -> Self {
        let mut result = Self::default();

        result.root = result.build_new_node(terrain, UVec2::ZERO, terrain.chunk_dim);

        result
    }

    fn build_new_node(
        &mut self,
        terrain: &NewTerrain,
        chunk_min: UVec2,
        chunk_max: UVec2,
    ) -> NodeId {
        let size = UVec2::new(
            (chunk_max.x - chunk_min.x).max(1),
            (chunk_max.y - chunk_min.y).max(1),
        );

        let is_leaf = size.x == 1 && size.y == 1;

        let mut min = Vec3::INFINITY;
        let mut max = Vec3::NEG_INFINITY;

        for y in chunk_min.y..chunk_max.y {
            for x in chunk_min.x..chunk_max.x {
                if let Some(chunk) = terrain.chunk_at(uvec2(x, y)) {
                    min = min.min(chunk.bounding_box.min);
                    max = max.max(chunk.bounding_box.max);
                }
            }
        }

        let mut node = Node {
            min: min.truncate(),
            max: max.truncate(),
            min_z: min.z,
            max_z: max.z,
            children: [None; 4],
            is_leaf,
            chunk_coord: is_leaf.then_some(chunk_min),
        };

        if !is_leaf {
            let mid = (chunk_min + chunk_max) / 2;

            let child_rects = [
                [uvec2(chunk_min.x, mid.y), uvec2(mid.x, chunk_max.y)], // Top-left
                [uvec2(mid.x, mid.y), uvec2(chunk_max.x, chunk_max.y)], // Top-right
                [uvec2(chunk_min.x, chunk_min.y), uvec2(mid.x, mid.y)], // Bottom-left
                [uvec2(mid.x, chunk_min.y), uvec2(chunk_max.x, mid.y)], // Bottom-right
            ];

            for (i, child) in node.children.iter_mut().enumerate() {
                let [chunk_min, chunk_max] = child_rects[i];
                let child_id = self.build_new_node(terrain, chunk_min, chunk_max);
                *child = Some(child_id);
            }
        }

        self.nodes.insert(node)
    }

    pub fn with_nodes_in_frustum<F>(&self, frustum: &Frustum, mut f: F)
    where
        F: FnMut(&Node),
    {
        self.collect_visible_nodes(self.root, frustum, &mut f);
    }

    fn collect_visible_nodes<F>(&self, node_id: NodeId, frustum: &Frustum, f: &mut F)
    where
        F: FnMut(&Node),
    {
        let node = &self.nodes[node_id];

        if !frustum.intersects_bounding_box(&node.bounding_box()) {
            return;
        }

        f(node);

        if !node.is_leaf {
            for child_id in node.children.iter().flatten() {
                self.collect_visible_nodes(*child_id, frustum, f);
            }
        }
    }

    fn _render_node(&self, node: &Node, gizmo_vertices: &mut Vec<GizmoVertex>) {
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
            self._render_node(node, gizmo_vertices);
        }

        node.children
            .iter()
            .filter_map(|child_id| *child_id)
            .filter_map(|child_id| self.nodes.get(child_id))
            .for_each(|node| self._render_node(node, gizmo_vertices));
    }

    pub fn _render_gizmos_in_frustum(
        &self,
        frustum: &Frustum,
        gizmo_vertices: &mut Vec<GizmoVertex>,
    ) {
        for (_, node) in self.nodes.iter() {
            if !frustum.intersects_bounding_box(&node.bounding_box()) {
                continue;
            }
            self._render_node(node, gizmo_vertices);
        }
    }
}
