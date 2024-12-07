use shadow_company_tools::smf;

use crate::engine::prelude::*;

use super::{
    asset_loader::{AssetError, AssetLoader},
    mesh_renderer::TexturedMesh,
};

pub type NodeIndex = usize;

type NameLookup = ahash::HashMap<String, NodeIndex>;

/// Model instance data held by each enitty.
#[derive(Debug)]
pub struct Model {
    /// A list of [Node]s contained in this [Model]. Parent nodes are guarranteed to be before its
    /// child nodes. Hierarchy is based on indices.
    pub nodes: Vec<ModelNode>,
    /// A list of [Mesh]es contained in this [Model]. They link back to [Node]s by index.
    pub meshes: Vec<ModelMesh>,
    /// A list of [BoundingBox]es contained in this [Model].
    pub bounding_boxes: Vec<ModelBoundingBox>,
    /// A map of node names to their indices in `nodes`.
    names: NameLookup,
}

impl Asset for Model {}

impl Model {
    /// Calculate the global transform for the given node.
    pub fn global_transform(&self, node_index: NodeIndex) -> Mat4 {
        let mut transform = Mat4::IDENTITY;
        let mut current = node_index;
        while current != NodeIndex::MAX {
            let node = &self.nodes[current];
            transform *= node.transform.to_mat4();
            current = node.parent;
        }
        transform
    }

    pub fn from_smf(smf: &smf::Model, renderer: &Renderer, asset_loader: &AssetLoader) -> Self {
        Self::smf_to_model(renderer, asset_loader, smf).unwrap()
    }

    fn smf_to_model(
        renderer: &Renderer,
        asset_loader: &AssetLoader,
        smf: &smf::Model,
    ) -> Result<Model, AssetError> {
        let mut names = NameLookup::default();
        let mut nodes = Vec::with_capacity(smf.nodes.len());
        let mut meshes = Vec::new();
        let mut bounding_boxes = Vec::new();

        for (node_index, smf_node) in smf.nodes.iter().enumerate() {
            names.insert(smf_node.name.clone(), node_index);

            nodes.push(ModelNode {
                parent: if smf_node.parent_name == "<root>" {
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
                },
                transform: Transform::new(smf_node.position, smf_node.rotation),
            });

            for smf_mesh in smf_node.meshes.iter() {
                let mesh = Self::smf_mesh_to_mesh(renderer, asset_loader, smf_mesh)?;
                meshes.push(ModelMesh {
                    node_id: node_index,
                    mesh: asset_loader.asset_manager().add(mesh),
                });
            }

            for smf_bounding_box in smf_node.bounding_boxes.iter() {
                bounding_boxes.push(ModelBoundingBox {
                    node_id: node_index,
                    min: smf_bounding_box.min,
                    max: smf_bounding_box.max,
                });
            }
        }

        Ok(Model {
            nodes,
            meshes,
            bounding_boxes,
            names,
        })
    }

    fn smf_mesh_to_mesh(
        renderer: &Renderer,
        asset_loader: &AssetLoader,
        smf_mesh: &smf::Mesh,
    ) -> Result<TexturedMesh, AssetError> {
        let vertices = smf_mesh
            .vertices
            .iter()
            .map(|v| crate::engine::mesh::Vertex {
                position: v.position,
                normal: v.normal,
                tex_coord: v.tex_coord,
            })
            .collect();

        let indices = smf_mesh.faces.iter().flat_map(|i| i.indices).collect();

        let gpu_mesh = crate::engine::mesh::IndexedMesh { vertices, indices }.to_gpu(renderer);

        let texture_path = std::path::PathBuf::from("textures")
            .join("shared")
            .join(&smf_mesh.texture_name);

        // TODO: Avoid uploding duplicate textures to the GPU.

        let image = asset_loader.load_bmp(&texture_path)?;
        let texture_view = renderer.create_texture_view(texture_path.to_str().unwrap(), image);

        // TODO: Reuse samplers.
        let sampler = renderer.create_sampler(
            "texture_sampler",
            wgpu::AddressMode::ClampToEdge,
            wgpu::FilterMode::Linear,
            wgpu::FilterMode::Linear,
        );

        let bind_group = renderer.create_texture_bind_group(
            texture_path.to_str().unwrap(),
            &texture_view,
            &sampler,
        );

        let mesh = TexturedMesh {
            gpu_mesh,
            texture: bind_group,
        };

        Ok(mesh)
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
    /// An index to the [ModelNode] this mesh is attached to.
    pub node_id: NodeIndex,
    /// Local transform.
    pub mesh: Handle<TexturedMesh>,
}

#[derive(Debug)]
pub struct ModelBoundingBox {
    /// An index to the [ModelNode] this mesh is attached to.
    pub node_id: NodeIndex,
    /// Minimum values for the bounding box.
    pub min: Vec3,
    /// Maximum values for the bounding box.
    pub max: Vec3,
}
