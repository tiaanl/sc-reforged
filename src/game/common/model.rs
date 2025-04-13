use std::{collections::HashMap, path::PathBuf};

use bevy_ecs::component::Component;
use shadow_company_tools::smf;

use crate::engine::{
    assets::resources::Resources,
    prelude::*,
    storage::{Handle, Storage},
};

use super::{
    asset_loader::{Asset, AssetError},
    image::Image,
    render::RenderTexture,
};

pub type NodeIndex = usize;

type NameLookup = HashMap<String, NodeIndex>;

/// Model instance data held by each enitty.
#[derive(Component, Debug)]
pub struct Model {
    /// A list of [Node]s contained in this [Model]. Parent nodes are guarranteed to be before its
    /// child nodes. Hierarchy is based on indices.
    pub nodes: Vec<ModelNode>,
    /// A list of meshes for the [Model].
    pub meshes: Vec<ModelMesh>,
    /// A list of [BoundingBox]es contained in this [Model].
    pub bounding_boxes: Vec<ModelBoundingBox>,
    /// A map of node names to their indices in `nodes`.
    names: NameLookup,
}

impl Asset for smf::Model {}

impl Model {
    /// Calculate the global transform for the given node.
    fn global_transform(&self, node_index: NodeIndex) -> Mat4 {
        let mut transform = Mat4::IDENTITY;
        let mut current = node_index;
        while current != NodeIndex::MAX {
            let node = &self.nodes[current];
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
    pub texture: Handle<RenderTexture>,
    pub mesh: GpuIndexedMesh,
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
pub struct ModelBoundingBox {
    /// An index to the [ModelNode] this mesh is attached to.
    pub node_index: NodeIndex,
    /// Minimum values for the bounding box.
    pub min: Vec3,
    /// Maximum values for the bounding box.
    pub max: Vec3,
    // Precalculated model transform.
    pub model_transform: Mat4,
}

struct MeshCollection {
    texture: Handle<RenderTexture>,
    mesh: IndexedMesh<ModelVertex>,
}

pub fn smf_to_model(
    smf: &smf::Model,
    renderer: &Renderer,
    resources: &Resources,
    texture_storage: &mut Storage<RenderTexture>,
) -> Result<Model, AssetError> {
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
    let mut mesh_lookup: HashMap<PathBuf, IndexedMesh<ModelVertex>> = HashMap::default();
    let mut bounding_boxes = Vec::new();
    let mut names = NameLookup::default();

    for (node_index, smf_node) in smf.nodes.iter().enumerate() {
        names.insert(smf_node.name.clone(), node_index);

        let parent_node_index = if smf_node.parent_name == "<root>" {
            // Use a sentinel for root nodes.
            NodeIndex::MAX
        } else {
            assert!(!smf_node.parent_name.eq("<root>"));
            match names.get(&smf_node.parent_name) {
                Some(id) => *id,
                None => {
                    let n = names.keys().cloned().collect::<Vec<_>>().join(", ");
                    return Err(AssetError::Custom(format!(
                        "Parent name [{}] not found, existing names: {}",
                        smf_node.parent_name, n
                    )));
                }
            }
        };

        nodes.push(ModelNode {
            parent: parent_node_index,
            transform: Transform::new(smf_node.position, Quat::IDENTITY),
        });

        for smf_mesh in smf_node.meshes.iter() {
            let texture_path = PathBuf::from("textures")
                .join("shared")
                .join(&smf_mesh.texture_name);
            let mesh = mesh_lookup.entry(texture_path.clone()).or_default();
            mesh.extend(std::iter::once(smf_mesh_to_mesh(
                smf_mesh,
                node_index as u32,
            )));
        }

        for smf_bounding_box in smf_node.bounding_boxes.iter() {
            bounding_boxes.push(ModelBoundingBox {
                node_index,
                min: smf_bounding_box.min,
                max: smf_bounding_box.max,
                model_transform: Mat4::IDENTITY,
            });
        }
    }

    let sampler = renderer.create_sampler(
        "model sampler",
        wgpu::AddressMode::Repeat,
        wgpu::FilterMode::Nearest,
        wgpu::FilterMode::Nearest,
    );

    let meshes = mesh_lookup
        .drain()
        .map(|(texture_path, mesh)| {
            let image = resources
                .request::<Image>(&texture_path)
                .unwrap_or_else(|_| panic!("Could not load texture. {}", texture_path.display()));
            let texture_view = renderer.create_texture_view("object texture", &image.data);
            let bind_group =
                renderer.create_texture_bind_group("object texture", &texture_view, &sampler);
            let texture = texture_storage.insert(RenderTexture {
                texture_view,
                bind_group,
            });

            let mesh = mesh.to_gpu(renderer);

            ModelMesh { texture, mesh }
        })
        .collect();

    Ok(Model {
        nodes,
        meshes,
        bounding_boxes,
        names,
    })
}
