use std::{collections::HashMap, path::PathBuf};

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
    pub nodes: Vec<ModelNode>,
    /// Meshes are combined based on their texture name, but each vertex still has a link to it's
    /// original node.
    pub meshes: Vec<ModelMesh>,
    /// A collection of collision boxes in the model, each associated with a specific node.
    pub collision_boxes: Vec<ModelCollisionBox>,
    /// Look up node indices according to original node names.
    names: NameLookup,
}

impl Model {
    /// Calculate the global transform for the given node.
    fn global_transform(&self, node_index: NodeIndex) -> Mat4 {
        let mut transform = Mat4::IDENTITY;
        let mut current = node_index;
        while current != NodeIndex::MAX {
            let node = &self.nodes[current as usize];
            transform *= node.transform.to_mat4();
            current = node.parent;
        }
        transform
    }
}

#[derive(Debug)]
pub struct ModelNode {
    /// An index to the node's parent.
    pub parent: NodeIndex,
    /// Local transform.
    pub transform: Transform,
}

#[derive(Debug)]
pub struct ModelMesh {
    pub texture_name: String,
    pub mesh: IndexedMesh<ModelVertex>,
}

#[derive(Clone, Copy, Debug, bytemuck::NoUninit)]
#[repr(C)]
pub struct ModelVertex {
    position: Vec3,
    normal: Vec3,
    tex_coord: Vec2,
    node_index: u32,
}

impl BufferLayout for ModelVertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x3,
            2 => Float32x2,
            3 => Uint32,
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRS,
        }
    }
}

#[derive(Debug)]
pub struct ModelCollisionBox {
    /// An index to the [ModelNode] this mesh is attached to.
    pub node_index: NodeIndex,
    /// Minimum values for the bounding box.
    pub min: Vec3,
    /// Maximum values for the bounding box.
    pub max: Vec3,
}

fn smf_to_model(smf: smf::Model) -> Result<Model, AssetError> {
    fn smf_mesh_to_mesh(smf_mesh: &smf::Mesh, node_index: u32) -> IndexedMesh<ModelVertex> {
        let vertices = smf_mesh
            .vertices
            .iter()
            .map(|v| ModelVertex {
                position: v.position,
                normal: -v.normal, // Normals are inverted.
                tex_coord: v.tex_coord,
                node_index,
            })
            .collect();

        let indices = smf_mesh.faces.iter().flat_map(|i| i.indices).collect();

        IndexedMesh { vertices, indices }
    }

    let mut nodes = Vec::with_capacity(smf.nodes.len());
    let mut mesh_lookup: HashMap<String, IndexedMesh<ModelVertex>> = HashMap::default();
    let mut collision_boxes = Vec::new();
    let mut names = NameLookup::default();

    for (node_index, smf_node) in smf.nodes.iter().enumerate() {
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
                        PathBuf::from(&smf.name),
                        format!(
                            "Parent name [{}] not found, existing names: {}",
                            smf_node.parent_name, n
                        ),
                    ));
                }
            }
        };

        nodes.push(ModelNode {
            parent: parent_node_index,
            transform: Transform::new(smf_node.position, Quat::IDENTITY),
        });

        for smf_mesh in smf_node.meshes.iter() {
            let mesh = mesh_lookup
                .entry(smf_mesh.texture_name.clone())
                .or_default();
            mesh.extend(&smf_mesh_to_mesh(smf_mesh, node_index as u32));
        }

        for smf_collision_box in smf_node.bounding_boxes.iter() {
            collision_boxes.push(ModelCollisionBox {
                node_index: node_index as u32,
                min: smf_collision_box.min,
                max: smf_collision_box.max,
            });
        }
    }

    let meshes = mesh_lookup
        .drain()
        .map(|(texture_name, mesh)| ModelMesh { texture_name, mesh })
        .collect();

    Ok(Model {
        nodes,
        meshes,
        collision_boxes,
        names,
    })
}

impl AssetType for Model {
    type Options = ();

    fn from_raw_with_options(
        raw: &[u8],
        _options: Self::Options,
        context: &AssetLoadContext,
    ) -> Result<Self, AssetError> {
        // Parse the .smf data.
        let smf = smf::Model::read(&mut std::io::Cursor::new(raw))
            .map_err(|err| AssetError::from_io_error(err, context.path))?;

        smf_to_model(smf)
    }
}
