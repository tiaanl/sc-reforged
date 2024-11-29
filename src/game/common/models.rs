use glam::{Mat4, Quat, Vec3};
use shadow_company_tools::smf;
use wgpu::{util::DeviceExt, ShaderStages};

use crate::engine::{
    assets::{Asset, AssetError, AssetLoader, Assets, Handle},
    mesh::{GpuMesh, RenderPassMeshExt},
    renderer::{RenderPipelineConfig, Renderer},
    shaders::Shaders,
};

/// Index referring to a [Node] inside a [Model].
pub type NodeIndex = usize;

#[derive(Debug)]
pub struct Mesh {
    /// Index to the [Node] this mesh belongs to.
    pub node_index: NodeIndex,

    pub gpu_mesh: GpuMesh,
    pub texture: wgpu::BindGroup,
}

#[derive(Debug)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
}

impl Transform {
    fn new(translation: Vec3, rotation: Quat) -> Self {
        Self {
            translation,
            rotation,
        }
    }

    fn to_mat4(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.rotation, self.translation)
    }
}

#[derive(Debug)]
pub struct Node {
    /// An index to the index of this [Node]'s parent [Node].
    pub parent: NodeIndex,
    /// Local transform.
    pub transform: Transform,
}

type NameLookup = ahash::HashMap<String, NodeIndex>;

#[derive(Debug)]
pub struct Model {
    /// A list of [Node]s contained in this [Model]. Parent nodes are guarranteed to be before its
    /// child nodes. Hierarchy is based on indices.
    pub nodes: Vec<Node>,
    /// A list of [Mesh]es contained in this [Model]. They link back to [Node]s by index.
    pub meshes: Vec<Mesh>,
    /// A buffer holding the global transforms for each [Mesh] in the model.
    transforms_bind_group: wgpu::BindGroup,
    /// A map of node names to their indices in `nodes`.
    names: NameLookup,
}

impl Model {
    /// Return a list of global transforms for each [Mesh] in the model.
    fn calculate_global_transforms(meshes: &[Mesh], nodes: &[Node]) -> Vec<[[f32; 4]; 4]> {
        meshes
            .iter()
            .map(|mesh| {
                let transform = nodes[mesh.node_index].transform.to_mat4();
                transform.to_cols_array_2d()
            })
            .collect()
    }
}

impl Asset for Model {}

#[derive(Debug)]
pub struct RenderJob {
    pub position: Vec3,
    pub rotation: Vec3,
    pub handle: Handle<Model>,
}

impl RenderJob {
    pub fn new(position: Vec3, rotation: Vec3, handle: Handle<Model>) -> Self {
        Self {
            position,
            rotation,
            handle,
        }
    }
}

pub struct ModelRenderer {
    render_pipeline: wgpu::RenderPipeline,
    transforms_bind_group_layout: wgpu::BindGroupLayout,
    models: Assets<Model>,
}

impl ModelRenderer {
    pub fn new(renderer: &Renderer, shaders: &mut Shaders) -> Self {
        let shader_module = shaders.create_shader(
            renderer,
            "model_renderer",
            include_str!("model.wgsl"),
            "model.wgsl",
        );

        let transforms_bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("model_transforms_bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None, // TODO: Specify this for validation?
                        },
                        count: None,
                    }],
                });

        let render_pipeline = renderer.create_render_pipeline(
            RenderPipelineConfig::<crate::engine::mesh::Vertex>::new(
                "model_renderer",
                &shader_module,
            )
            .primitive(wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                ..Default::default()
            })
            .blend_state(wgpu::BlendState::ALPHA_BLENDING)
            // u_camera
            .bind_group_layout(renderer.uniform_bind_group_layout())
            // u_transforms
            .bind_group_layout(&transforms_bind_group_layout)
            // u_texture
            .bind_group_layout(renderer.texture_bind_group_layout()),
        );

        Self {
            render_pipeline,
            transforms_bind_group_layout,
            models: Assets::default(),
        }
    }

    pub fn add(
        &mut self,
        renderer: &Renderer,
        assets: &AssetLoader,
        smf: &smf::Model,
    ) -> Handle<Model> {
        let model = self
            .smf_to_model(renderer, assets, smf)
            .expect("Could not load model! FIX THIS");

        self.models.add(model)
    }

    pub fn render_multiple(
        &self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        camera_bind_group: &wgpu::BindGroup,
        batch: &[RenderJob],
        load: wgpu::LoadOp<wgpu::Color>,
    ) {
        let mut render_pass = Self::create_render_pass(renderer, encoder, output, load);
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        batch.iter().for_each(|job| {
            let Some(model) = self.models.get(&job.handle) else {
                return;
            };

            for mesh in model.meshes.iter() {
                let node_index = mesh.node_index as u32;
                render_pass.set_bind_group(1, &model.transforms_bind_group, &[]);
                render_pass.set_bind_group(2, &mesh.texture, &[]);
                render_pass.draw_mesh(&mesh.gpu_mesh, node_index..node_index + 1);
            }
        });
    }

    fn create_render_pass<'encoder>(
        renderer: &Renderer,
        encoder: &'encoder mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        load: wgpu::LoadOp<wgpu::Color>,
    ) -> wgpu::RenderPass<'encoder> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("model_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: renderer
                .render_pass_depth_stencil_attachment(wgpu::LoadOp::Clear(1.0)),
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }
}

impl ModelRenderer {
    fn smf_to_model(
        &mut self,
        renderer: &Renderer,
        assets: &AssetLoader,
        smf: &smf::Model,
    ) -> Result<Model, AssetError> {
        let mut names = NameLookup::default();
        let mut nodes = Vec::with_capacity(smf.nodes.len());
        let mut meshes = Vec::new();

        for (node_index, smf_node) in smf.nodes.iter().enumerate() {
            names.insert(smf_node.name.clone(), node_index);

            nodes.push(Node {
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
                let mesh = self.smf_mesh_to_mesh(renderer, assets, node_index, smf_mesh)?;
                meshes.push(mesh);
            }
        }

        let transforms = {
            let transforms = Model::calculate_global_transforms(&meshes, &nodes);
            let buffer = renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("model_transforms_buffer"),
                    contents: bytemuck::cast_slice(&transforms),
                    usage: wgpu::BufferUsages::STORAGE,
                });

            renderer
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("model_transforms_bind_group"),
                    layout: &self.transforms_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                })
        };

        Ok(Model {
            nodes,
            meshes,
            transforms_bind_group: transforms,
            names,
        })
    }

    fn smf_mesh_to_mesh(
        &mut self,
        renderer: &Renderer,
        assets: &AssetLoader,
        node_index: NodeIndex,
        mesh: &smf::Mesh,
    ) -> Result<Mesh, AssetError> {
        let vertices = mesh
            .vertices
            .iter()
            .map(|v| crate::engine::mesh::Vertex {
                position: v.position,
                normal: v.normal,
                tex_coord: v.tex_coord,
            })
            .collect();

        let indices = mesh.faces.iter().flat_map(|i| i.indices).collect();

        let gpu_mesh = crate::engine::mesh::Mesh { vertices, indices }.to_gpu(renderer);

        let texture_path = std::path::PathBuf::from("textures")
            .join("shared")
            .join(&mesh.texture_name);

        // TODO: Avoid uploding duplicate textures to the GPU.

        let image = assets.load_bmp(&texture_path)?;
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

        Ok(Mesh {
            node_index,
            gpu_mesh,
            texture: bind_group,
        })
    }
}
