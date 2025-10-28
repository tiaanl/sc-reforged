#![allow(dead_code)]

use std::path::PathBuf;

use ahash::HashMap;
use shadow_company_tools::smf;

use crate::game::math::{BoundingBox, Ray, RaySegment};
use crate::{
    engine::{prelude::*, storage::Handle},
    game::{
        image::{Image, images},
        math::BoundingSphere,
        skeleton::{Bone, Skeleton},
    },
};
use glam::{Mat4, Quat, Vec2, Vec3};

pub type NodeIndex = u32;

type NameLookup = HashMap<String, NodeIndex>;

/// Model instance data held by each enitty.
#[derive(Clone, Debug)]
pub struct Model {
    /// The hierarchical bone structure of the model.
    pub skeleton: Skeleton,
    /// A list of all the [Mesh]s contained in this model.
    pub meshes: Vec<Mesh>,
    /// A collection of collision boxes in the model, each associated with a specific node.
    pub collision_boxes: Vec<CollisionBox>,
    /// A bounding sphere surrounding all the vertices in the model.
    pub bounding_sphere: BoundingSphere,
    /// Look up node indices according to original node names.
    pub name_lookup: NameLookup,
}

impl Model {
    pub fn from_skeleton(skeleton: Skeleton) -> Self {
        Self {
            skeleton,
            meshes: Vec::default(),
            collision_boxes: Vec::default(),
            bounding_sphere: BoundingSphere::ZERO,
            name_lookup: HashMap::default(),
        }
    }

    /// Find a node index for a mesh with the given name.
    pub fn node_index_by_name(&self, name: &str) -> Option<NodeIndex> {
        self.name_lookup.get(name).cloned()
    }

    /// Clone the meshes from another [Model]'s node to this one. *Does not recurse nodes.*
    pub fn clone_meshes(
        &mut self,
        other: &Model,
        source_node_index: NodeIndex,
        target_node_index: NodeIndex,
    ) {
        self.meshes.extend(
            other
                .meshes
                .iter()
                .filter(|mesh| mesh.node_index == source_node_index)
                .map(|mesh| Mesh {
                    node_index: target_node_index,
                    ..mesh.clone()
                }),
        );
    }

    /// Remove all the meshes from the specified node.
    pub fn clear_meshes(&mut self, node_index: NodeIndex) {
        self.meshes.retain(|mesh| mesh.node_index != node_index);
    }
}

/// Result of a model ray-segment intersection against collision boxes.
#[derive(Clone, Copy, Debug)]
pub struct ModelRayHit {
    /// Parameter along the input ray: `hit = origin + direction * t`.
    pub t: f32,
    /// World-space position of the intersection.
    pub world_position: Vec3,
    /// World-space surface normal (unit length).
    pub normal: Vec3,
    /// Node the collision box belongs to.
    pub node_index: NodeIndex,
    /// Index into `Model::collision_boxes`.
    pub collision_box_index: usize,
}

impl Model {
    /// Intersect a world-space ray segment with this model's collision boxes, applying an
    /// object-to-world transform for the model instance. Returns the closest hit, if any.
    pub fn intersect_ray_segment_with_transform(
        &self,
        object_to_world: Mat4,
        ray_segment: &RaySegment,
    ) -> Option<ModelRayHit> {
        debug_assert!(ray_segment.distance.is_finite());
        debug_assert!(!ray_segment.is_degenerate());
        debug_assert!(ray_segment.ray.direction.is_normalized());
        debug_assert!(!ray_segment.ray.direction.is_nan());

        let world_dir = ray_segment.ray.direction;
        let t_max_world = ray_segment.t_max(); // equals distance when direction is normalized
        let world_end = ray_segment.ray.origin + world_dir * t_max_world;

        let mut best: Option<ModelRayHit> = None;
        let mut best_t = f32::INFINITY;

        for (i, cb) in self.collision_boxes.iter().enumerate() {
            let node_to_world = object_to_world * self.skeleton.local_transform(cb.node_index);
            let world_to_node = node_to_world.inverse();

            // Transform the segment endpoints into node-local space.
            let local_origin = world_to_node.transform_point3(ray_segment.ray.origin);
            let local_end = world_to_node.transform_point3(world_end);
            let local_dir = local_end - local_origin;

            // Skip degenerate local rays.
            let local_len = local_dir.length();
            debug_assert!(local_len.is_finite());
            if local_len <= f32::EPSILON {
                continue;
            }

            let local_segment = RaySegment {
                ray: Ray {
                    origin: local_origin,
                    direction: local_dir,
                },
                distance: local_len,
            };

            let bbox = BoundingBox {
                min: cb.min,
                max: cb.max,
            };

            if let Some((t_enter_local, _t_exit_local, enter_normal_local)) =
                bbox.intersect_ray_segment(&local_segment)
            {
                // Compute local/world-space hit and normal.
                let local_hit = local_origin + local_dir * t_enter_local;
                let world_hit = node_to_world.transform_point3(local_hit);
                let normal_world = node_to_world
                    .transform_vector3(enter_normal_local)
                    .normalize_or_zero();

                // Recover t along the original (normalized) world ray.
                let delta_world = world_hit - ray_segment.ray.origin;
                let t_world = delta_world.dot(world_dir);

                // Keep only hits within the segment range and closest first.
                if (0.0..=t_max_world).contains(&t_world) && t_world < best_t {
                    best_t = t_world;
                    debug_assert!(normal_world.is_normalized() || normal_world.length() == 0.0);
                    best = Some(ModelRayHit {
                        t: t_world,
                        world_position: world_hit,
                        normal: normal_world,
                        node_index: cb.node_index,
                        collision_box_index: i,
                    });
                }
            }
        }

        best
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    /// An index to the node's parent.
    pub parent: NodeIndex,
    /// Local transform.
    pub transform: Transform,
    /// The ID of the bone.
    pub bone_id: u32,
    /// The name of the node.
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct Mesh {
    /// The node this mesh belongs to.
    pub node_index: u32,
    /// Name of the texture to use for the material.
    pub image_name: String,
    /// Handle to the loaded image.
    pub image: Handle<Image>,
    /// Vertex and index data.
    pub mesh: IndexedMesh<Vertex>,
}

#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub tex_coord: Vec2,
    pub node_index: u32,
}

#[derive(Clone, Debug)]
pub struct CollisionBox {
    /// An index to the [ModelNode] this mesh is attached to.
    pub node_index: NodeIndex,
    /// Minimum values for the bounding box.
    pub min: Vec3,
    /// Maximum values for the bounding box.
    pub max: Vec3,
}

impl TryFrom<smf::Model> for Model {
    type Error = AssetError;

    fn try_from(value: smf::Model) -> Result<Self, Self::Error> {
        fn smf_mesh_to_mesh(smf_mesh: &smf::Mesh, node_index: u32) -> IndexedMesh<Vertex> {
            let vertices = smf_mesh
                .vertices
                .iter()
                .map(|v| Vertex {
                    position: v.position,
                    normal: -v.normal, // Normals are inverted.
                    tex_coord: v.tex_coord,
                    node_index,
                })
                .collect();

            let indices = smf_mesh.faces.iter().flat_map(|i| i.indices).collect();

            IndexedMesh { vertices, indices }
        }

        let mut nodes = Vec::with_capacity(value.nodes.len());
        let mut meshes = Vec::default();
        let mut collision_boxes = Vec::new();
        let mut names = NameLookup::default();

        for (node_index, smf_node) in value.nodes.into_iter().enumerate() {
            names.insert(smf_node.name.clone(), node_index as u32);

            let parent_node_index = if smf_node.parent_name == "<root>" {
                // Use a sentinel for root nodes.
                NodeIndex::MAX
            } else {
                assert!(!smf_node.parent_name.eq("<root>"));
                match names.get(&smf_node.parent_name) {
                    Some(id) => *id,
                    None => {
                        let n = names.keys().cloned().collect::<Vec<_>>().join(", ");
                        return Err(AssetError::custom(
                            &value.name,
                            format!(
                                "Parent name [{}] not found, existing names: {}",
                                smf_node.parent_name, n
                            ),
                        ));
                    }
                }
            };

            // TODO: Figure out what this weird rotation on static models are so we don't use
            //       Mat4::IDENTITY here.
            nodes.push(Node {
                parent: parent_node_index,
                transform: Transform::new(smf_node.position, Quat::IDENTITY),
                bone_id: smf_node.tree_id,
                name: smf_node.name.clone(),
            });

            meshes.extend(
                smf_node
                    .meshes
                    .iter()
                    .map(|smf_mesh| -> Result<Mesh, AssetError> {
                        let mesh = smf_mesh_to_mesh(smf_mesh, node_index as u32);

                        // For now assume we're loading a shared image.
                        let path = PathBuf::from("textures")
                            .join("shared")
                            .join(&smf_mesh.texture_name);

                        let image = images().load_image(path)?;

                        Ok(Mesh {
                            node_index: node_index as u32,
                            image_name: smf_mesh.texture_name.clone(),
                            image,
                            mesh,
                        })
                    })
                    .filter_map(|mesh| {
                        mesh.inspect_err(|err| {
                            tracing::warn!("Could not load mesh: {}", err);
                        })
                        .ok()
                    }),
            );

            for smf_collision_box in smf_node.bounding_boxes.iter() {
                collision_boxes.push(CollisionBox {
                    node_index: node_index as u32,
                    min: smf_collision_box.min,
                    max: smf_collision_box.max,
                });
            }
        }

        let skeleton = Skeleton {
            bones: nodes
                .iter()
                .map(|node| Bone {
                    parent: node.parent,
                    transform: node.transform.clone(),
                    _id: node.bone_id,
                    _name: node.name.clone(),
                })
                .collect(),
        };

        let mut bounding_sphere = BoundingSphere::default();
        for mesh in meshes.iter() {
            let local = skeleton.local_transform(mesh.node_index);
            let b = BoundingSphere::from_positions_ritter(
                mesh.mesh
                    .vertices
                    .iter()
                    .map(|v| local.transform_point3(v.position)),
            );

            bounding_sphere.expand_to_include(&b);
        }

        Ok(Model {
            skeleton,
            meshes,
            collision_boxes,
            bounding_sphere,
            name_lookup: names,
        })
    }
}
