use glam::{IVec2, Vec2, Vec3, ivec2};
use slab::Slab;

use crate::{
    engine::storage::Handle,
    game::{
        math::{BoundingBox, BoundingSphere, Frustum, RaySegment},
        scenes::world::{objects::Object, terrain::Terrain},
    },
};

pub type NodeId = usize;

#[derive(Debug)]
pub struct ObjectEntry {
    pub handle: Handle<Object>,
    pub bounding_sphere: BoundingSphere,
}

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
    pub objects: Vec<ObjectEntry>,
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
    nodes: Slab<Node>,
    root: NodeId,
    pub max_level: usize,
}

impl QuadTree {
    pub fn from_new_terrain(terrain: &Terrain) -> Self {
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
            self.nodes[node_id].objects.push(ObjectEntry {
                handle: object,
                bounding_sphere: *bounding_sphere,
            });
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

    pub fn _ray_cast_all_segment(&self, ray_segment: &RaySegment) -> Vec<RayCastHit> {
        let mut hits = Vec::new();
        self.traverse_segment(self.root, ray_segment, false, &mut hits);
        hits.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
        hits
    }

    pub fn ray_cast_first_segment(&self, segment: &RaySegment) -> Option<RayCastHit> {
        let mut hits = Vec::new();
        self.traverse_segment(self.root, segment, true, &mut hits);
        hits.into_iter()
            .min_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap())
    }

    fn traverse_segment(
        &self,
        node_id: NodeId,
        ray_segment: &RaySegment,
        first_only: bool,
        out_hits: &mut Vec<RayCastHit>,
    ) {
        let bb = self.nodes[node_id].bounding_box();

        let Some((t_entry_node, _t_exit_node, _n)) = bb.intersect_ray_segment(ray_segment) else {
            return;
        };

        // Early pruning when we already have a nearer hit and only want the first.
        if first_only {
            if let Some(best) = out_hits
                .iter()
                .min_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap())
            {
                let entry_distance = t_entry_node * ray_segment.ray.direction.length();
                if entry_distance > best.distance {
                    return;
                }
            }
        }

        let node = &self.nodes[node_id];

        // If leaf, consider the terrain chunk AABB as a candidate hit.
        if node.is_leaf {
            if let Some((t_hit, _t_exit, normal)) = bb.intersect_ray_segment(ray_segment) {
                let hit_point = ray_segment.ray.origin + ray_segment.ray.direction * t_hit;
                let distance = t_hit * ray_segment.ray.direction.length();
                out_hits.push(RayCastHit {
                    _t_fraction: if ray_segment.t_max() > 0.0 {
                        t_hit / ray_segment.t_max()
                    } else {
                        0.0
                    },
                    distance,
                    _world_point: hit_point,
                    _world_normal: normal,
                    _target: RayCastTarget::TerrainChunk {
                        _chunk_coord: node.chunk_coord.unwrap(),
                    },
                });
                if first_only {
                    return;
                }
            }
        } else {
            // Visit children front-to-back by entry t.
            let mut children_hits: Vec<(NodeId, f32)> = Vec::new();
            for child_id in node.children.iter().flatten().copied() {
                let child_box = self.nodes[child_id].bounding_box();
                if let Some((t_enter, _t_exit, _)) = child_box.intersect_ray_segment(ray_segment) {
                    children_hits.push((child_id, t_enter));
                }
            }
            children_hits.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            for (child_id, _) in children_hits {
                self.traverse_segment(child_id, ray_segment, first_only, out_hits);
                if first_only && !out_hits.is_empty() {
                    return;
                }
            }
        }

        if !node.objects.is_empty() {
            for object in node.objects.iter() {
                let sphere = object.bounding_sphere;
                if let Some((t_hit, normal)) = sphere.intersect_ray_segment(ray_segment) {
                    let hit_point = ray_segment.ray.origin + ray_segment.ray.direction * t_hit;
                    let distance = t_hit * ray_segment.ray.direction.length();

                    if first_only {
                        if let Some(best) = out_hits
                            .iter()
                            .min_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap())
                        {
                            if distance >= best.distance {
                                continue;
                            }
                        }
                    }

                    out_hits.push(RayCastHit {
                        _t_fraction: if ray_segment.t_max() > 0.0 {
                            t_hit / ray_segment.t_max()
                        } else {
                            0.0
                        },
                        distance,
                        _world_point: hit_point,
                        _world_normal: normal,
                        _target: RayCastTarget::Object {
                            _object: object.handle,
                        },
                    });

                    if first_only {
                        return;
                    }
                }
            }
        }
    }

    pub fn _print_nodes(&self) {
        fn print_internal(nodes: &slab::Slab<Node>, node_id: NodeId, level: usize) {
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

#[derive(Debug)]
pub enum RayCastTarget {
    TerrainChunk { _chunk_coord: IVec2 },
    Object { _object: Handle<Object> },
}

#[derive(Debug)]
pub struct RayCastHit {
    /// Fraction along the segment in [0,1]. Useful for depth-sorting UI, etc.
    pub _t_fraction: f32,
    /// World distance from segment.ray.origin to the hit point.
    pub distance: f32,
    pub _world_point: Vec3,
    pub _world_normal: Vec3,
    pub _target: RayCastTarget,
}
