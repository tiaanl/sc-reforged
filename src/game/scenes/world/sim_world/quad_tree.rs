use glam::{IVec2, Vec2, Vec3, ivec2};

use crate::{
    engine::storage::Handle,
    game::math::{BoundingBox, BoundingSphere, Frustum, RaySegment},
};

use super::{objects::Object, terrain::Terrain};

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
    /// The level in the hierarchy.
    pub level: usize,
    /// Each of the possible 4 children that makes up the quad tree.
    children: [Option<NodeId>; 4],
    /// If this node is a leaf and wraps a single terrain chunk, it holds the chunk coord.
    pub chunk_coord: Option<IVec2>,
    /// A list of objects who's bounding spheres are fully contained inside this node.
    pub objects: Vec<Handle<Object>>,
}

impl Node {
    pub fn bounding_box(&self) -> BoundingBox {
        BoundingBox {
            min: self.min.extend(self.min_z),
            max: self.max.extend(self.max_z),
        }
    }
}

#[derive(Debug, Default)]
pub struct QuadTree {
    nodes: Vec<Node>,
    root: NodeId,
    pub max_level: usize,
}

impl QuadTree {
    pub fn from_terrain(terrain: &Terrain) -> Self {
        let mut result = Self::default();

        result.root = result.build_new_node(terrain, IVec2::ZERO, terrain.chunk_dim.as_ivec2(), 0);

        result
    }

    pub fn insert_object(&mut self, object: Handle<Object>, bounding_sphere: &BoundingSphere) {
        self.insert_object_at(self.root, object, bounding_sphere);
    }

    fn insert_object_at(
        &mut self,
        node_id: NodeId,
        object: Handle<Object>,
        bounding_sphere: &BoundingSphere,
    ) {
        // Expand the current node Z to cover the object span so frustum culling of this node stays
        // correct.
        self.expand_node_z_to_fit_sphere(node_id, bounding_sphere);

        let target_child = {
            let node = &self.nodes[node_id];
            if node.is_leaf {
                None
            } else {
                let mut target: Option<NodeId> = None;
                for child_id in node.children.iter().flatten() {
                    let child_min = self.nodes[*child_id].min;
                    let child_max = self.nodes[*child_id].max;

                    let center = bounding_sphere.center;
                    let radius = bounding_sphere.radius;

                    if center.x - radius >= child_min.x
                        && center.x + radius <= child_max.x
                        && center.y - radius >= child_min.y
                        && center.y + radius <= child_max.y
                    {
                        target = Some(*child_id);
                        break;
                    }
                }
                target
            }
        };

        // If we found a child that fits the sphere, try to insert it there, otherwise, insert the
        // sphere into this node.
        if let Some(child_id) = target_child {
            self.insert_object_at(child_id, object, bounding_sphere);
        } else {
            self.nodes[node_id].objects.push(object);
        }
    }

    /// Expand the Z bounds of the node to fully fit the specified [BoundingSphere].
    fn expand_node_z_to_fit_sphere(&mut self, node_id: NodeId, sphere: &BoundingSphere) {
        let (min_z, max_z) = (
            sphere.center.z - sphere.radius,
            sphere.center.z + sphere.radius,
        );

        let node = &mut self.nodes[node_id];
        node.min_z = node.min_z.min(min_z);
        node.max_z = node.max_z.max(max_z);
    }

    fn build_new_node(
        &mut self,
        terrain: &Terrain,
        chunk_min: IVec2,
        chunk_max: IVec2,
        level: usize,
    ) -> NodeId {
        let size = ivec2(
            (chunk_max.x - chunk_min.x).max(1),
            (chunk_max.y - chunk_min.y).max(1),
        );

        let is_leaf = size.x == 1 && size.y == 1;

        let mut min = Vec3::INFINITY;
        let mut max = Vec3::NEG_INFINITY;

        for y in chunk_min.y..chunk_max.y {
            for x in chunk_min.x..chunk_max.x {
                if let Some(chunk) = terrain.chunk_at(ivec2(x, y)) {
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
            level,
            chunk_coord: is_leaf.then_some(chunk_min),
            objects: Vec::default(),
        };

        if !is_leaf {
            let mid = (chunk_min + chunk_max) / 2;

            let child_rects = [
                [ivec2(chunk_min.x, mid.y), ivec2(mid.x, chunk_max.y)], // Top-left
                [ivec2(mid.x, mid.y), ivec2(chunk_max.x, chunk_max.y)], // Top-right
                [ivec2(chunk_min.x, chunk_min.y), ivec2(mid.x, mid.y)], // Bottom-left
                [ivec2(mid.x, chunk_min.y), ivec2(chunk_max.x, mid.y)], // Bottom-right
            ];

            for (i, child) in node.children.iter_mut().enumerate() {
                let [chunk_min, chunk_max] = child_rects[i];
                let child_id = self.build_new_node(terrain, chunk_min, chunk_max, level + 1);
                *child = Some(child_id);
            }
        }

        self.max_level = self.max_level.max(level);

        let id = self.nodes.len();
        self.nodes.push(node);
        id
    }

    pub fn with_nodes_in_frustum<F>(&self, frustum: &Frustum, mut f: F)
    where
        F: FnMut(&Node),
    {
        self.traverse_nodes_in_frustum(self.root, frustum, &mut f);
    }

    fn traverse_nodes_in_frustum<F>(&self, node_id: NodeId, frustum: &Frustum, f: &mut F)
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
                self.traverse_nodes_in_frustum(*child_id, frustum, f);
            }
        }
    }

    pub fn with_nodes_ray_segment<F>(&self, ray_segment: &RaySegment, mut f: F)
    where
        F: FnMut(&Node),
    {
        self.traverse_nodes_ray_segment(self.root, ray_segment, &mut f);
    }

    fn traverse_nodes_ray_segment<F>(&self, node_id: NodeId, ray_segment: &RaySegment, f: &mut F)
    where
        F: FnMut(&Node),
    {
        let node = &self.nodes[node_id];

        if node
            .bounding_box()
            .intersect_ray_segment(ray_segment)
            .is_none()
        {
            return;
        }

        f(node);

        if !node.is_leaf {
            for child_id in node.children.iter().flatten() {
                self.traverse_nodes_ray_segment(*child_id, ray_segment, f);
            }
        }
    }

    pub fn _print_nodes(&self) {
        fn print_internal(nodes: &[Node], node_id: NodeId, level: usize) {
            let node = &nodes[node_id];

            for _ in 0..level {
                print!("  ");
            }
            println!("objects: {}", node.objects.len());

            for child_id in node.children.iter().flatten().cloned() {
                print_internal(nodes, child_id, level + 1);
            }
        }
        print_internal(&self.nodes, self.root, 0);
    }
}
