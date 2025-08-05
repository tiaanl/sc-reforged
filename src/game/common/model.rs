use std::path::PathBuf;

use ahash::HashMap;
use shadow_company_tools::smf;

use crate::engine::prelude::*;

pub type NodeIndex = u32;

type NameLookup = HashMap<String, NodeIndex>;

/// Model instance data held by each enitty.
#[derive(Debug)]
pub struct Model {
    /// A list of [ModelNode]s that define the hierarchy of this [Model]. Each node's parent is
    /// guaranteed to appear earlier in the list than the node itself, ensuring a top-down order for
    /// traversal.
    pub nodes: Vec<Node>,
    /// A list of all the [Mesh]s contained in this model.
    pub meshes: Vec<Mesh>,
    /// A collection of collision boxes in the model, each associated with a specific node.
    pub _collision_boxes: Vec<CollisionBox>,
    /// Look up node indices according to original node names.
    _names: NameLookup,

    // Possibly ground radius and weight?
    pub scale: Vec3,
}

impl Model {
    pub fn local_transform(&self, node_index: u32) -> Mat4 {
        let node = &self.nodes[node_index as usize];
        if node.parent == NodeIndex::MAX {
            node.transform.to_mat4()
        } else {
            self.local_transform(node.parent) * node.transform.to_mat4()
        }
    }
}

#[derive(Debug)]
pub struct Node {
    /// An index to the node's parent.
    pub parent: NodeIndex,
    /// Local transform.
    pub transform: Transform,
    /// The name of the node.
    pub name: String,
}

#[derive(Debug)]
pub struct Mesh {
    /// The node this mesh belongs to.
    pub node_index: u32,
    /// Name of the texture to use for the material.
    pub texture_name: String,
    /// Vertex and index data.
    pub mesh: IndexedMesh<Vertex>,
    /// A bounding sphere surrounding all the vertices in the mesh as tightly as possible.
    pub bounding_sphere: BoundingSphere,
}

// TODO: Don't use this as a bytemuck to write to the GPU.
#[derive(Clone, Copy, Debug, bytemuck::NoUninit)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub tex_coord: Vec2,
    pub node_index: u32,
}

impl BufferLayout for Vertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x3,
            2 => Float32x2,
            3 => Uint32,
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRS,
        }
    }
}

#[derive(Debug)]
pub struct CollisionBox {
    /// An index to the [ModelNode] this mesh is attached to.
    pub _node_index: NodeIndex,
    /// Minimum values for the bounding box.
    pub _min: Vec3,
    /// Maximum values for the bounding box.
    pub _max: Vec3,
}

#[derive(Debug, Default)]
pub struct BoundingSphere {
    pub center: Vec3,
    pub radius: f32,
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
                        return Err(AssetError::Unknown(
                            PathBuf::from(&value.name),
                            format!(
                                "Parent name [{}] not found, existing names: {}",
                                smf_node.parent_name, n
                            ),
                        ));
                    }
                }
            };

            nodes.push(Node {
                parent: parent_node_index,
                transform: Transform::new(smf_node.position, Quat::IDENTITY),
                name: smf_node.name.clone(),
            });

            meshes.extend(smf_node.meshes.iter().map(|smf_mesh| {
                let mesh = smf_mesh_to_mesh(smf_mesh, node_index as u32);

                let bounding_sphere = {
                    let center = mesh
                        .vertices
                        .iter()
                        .fold(Vec3::ZERO, |acc, v| acc + v.position)
                        / mesh.vertices.len() as f32;
                    let radius = mesh
                        .vertices
                        .iter()
                        .map(|v| (v.position - center).length())
                        .fold(0.0, f32::max);

                    BoundingSphere { center, radius }
                };

                Mesh {
                    node_index: node_index as u32,
                    texture_name: smf_mesh.texture_name.clone(),
                    mesh,
                    bounding_sphere,
                }
            }));

            for smf_collision_box in smf_node.bounding_boxes.iter() {
                collision_boxes.push(CollisionBox {
                    _node_index: node_index as u32,
                    _min: smf_collision_box.min,
                    _max: smf_collision_box.max,
                });
            }
        }

        let scale = value.scale;

        Ok(Model {
            nodes,
            meshes,
            _collision_boxes: collision_boxes,
            _names: names,
            scale,
        })
    }
}
