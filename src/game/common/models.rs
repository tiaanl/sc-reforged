use std::{collections::HashMap, ops::Range, path::PathBuf};

use glam::Mat4;
use wgpu::util::DeviceExt;

use crate::engine::{
    assets::{Asset, AssetError},
    mesh::{GpuIndexedMesh, IndexedMesh},
    prelude::{BufferLayout, Frame, Renderer},
    shaders::Shaders,
    storage::{Handle, Storage},
};

use super::{
    assets::DataDir,
    geometry_buffers::GeometryBuffers,
    model::{Model, ModelVertex},
};

/// Contains all loaded models and their GPU counterparts.
pub struct ModelManager {
    /// Store all loaded models.
    models: Storage<RenderModel>,
    /// Store all textures used by the models.
    textures: Storage<RenderTexture>,
    /// A lookup for model names to existing loaded models.
    models_cache: HashMap<String, Handle<RenderModel>>,
    /// The pipeline used to render models.
    pipeline: wgpu::RenderPipeline,
    /// Bind group layout used for binding the nodes data for draw calls.
    nodes_buffer_bind_group_layout: wgpu::BindGroupLayout,
}

impl ModelManager {
    /// Create a new manager for models with the given [DataDir] for loading models.
    pub fn new(
        renderer: &Renderer,
        shaders: &mut Shaders,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let models = Storage::default();
        let textures = Storage::default();

        let models_cache = HashMap::default();

        let nodes_buffer_bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("nodes_buffer_bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let pipeline = create_pipeline(
            renderer,
            shaders,
            camera_bind_group_layout,
            &nodes_buffer_bind_group_layout,
        );

        Self {
            models,
            textures,
            models_cache,
            pipeline,
            nodes_buffer_bind_group_layout,
        }
    }

    pub fn load_object(
        &mut self,
        renderer: &Renderer,
        name: &str,
    ) -> Result<Handle<RenderModel>, AssetError> {
        if let Some(model) = self.models_cache.get(name) {
            return Ok(*model);
        }

        let model = DataDir::load_object_model(name)?;

        // Build a single mesh and generate a list of draw commands per texture.
        let mut indexed_mesh = IndexedMesh::default();
        let mut draw_commands = Vec::with_capacity(model.meshes.len());
        for mesh in model.meshes.iter() {
            let texture = self.load_image(renderer, &mesh.texture_name)?;

            let index_range = indexed_mesh.extend(&mesh.mesh);

            draw_commands.push(DrawCommand {
                index_range,
                texture,
            });
        }

        if indexed_mesh.indices.is_empty() {
            tracing::warn!("Empty mesh: {name}");
            return Err(AssetError::Decode(PathBuf::from(name)));
        }

        let mesh = indexed_mesh.to_gpu(renderer);

        let nodes_buffer_bind_group = {
            #[derive(Clone, Copy, bytemuck::NoUninit)]
            #[repr(C)]
            struct NodeData {
                parent: [u32; 4],
                transform: [[f32; 4]; 4],
            }
            let node_data = model
                .nodes
                .iter()
                .map(|node| NodeData {
                    parent: [node.parent, 0, 0, 0],
                    transform: Mat4::from_translation(node.transform.translation)
                        .to_cols_array_2d(),
                })
                .collect::<Vec<_>>();

            let buffer = renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("model_nodes_buffer"),
                    contents: bytemuck::cast_slice(&node_data),
                    usage: wgpu::BufferUsages::STORAGE,
                });

            renderer
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("nodes_buffer_bind_group"),
                    layout: &self.nodes_buffer_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                })
        };

        let handle = self.models.insert(RenderModel {
            model,
            mesh,
            nodes_buffer_bind_group,
            draw_commands,
        });

        debug_assert!(!self.models_cache.contains_key(name));

        self.models_cache.insert(String::from(name), handle);

        Ok(handle)
    }

    fn load_image(
        &mut self,
        renderer: &Renderer,
        name: &str,
    ) -> Result<Handle<RenderTexture>, AssetError> {
        let texture_path = PathBuf::from("textures").join("shared").join(name);
        let image = DataDir::load_image(&texture_path)?;

        let label = &texture_path.display().to_string();

        let texture_view = renderer.create_texture_view(label, &image.data);

        let sampler = renderer.create_sampler(
            label,
            wgpu::AddressMode::Repeat,
            wgpu::FilterMode::Linear,
            wgpu::FilterMode::Linear,
        );

        let bind_group = renderer.create_texture_bind_group(label, &texture_view, &sampler);

        Ok(self.textures.insert(RenderTexture {
            texture_view,
            bind_group,
        }))
    }

    pub fn new_render_set(&self) -> RenderModelSet {
        RenderModelSet::default()
    }

    pub fn render_model_set(
        &self,
        frame: &mut Frame,
        geometry_buffers: &GeometryBuffers,
        camera_bind_group: &wgpu::BindGroup,
        set: RenderModelSet,
    ) {
        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("models_render_pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &geometry_buffers.colors.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &geometry_buffers.positions.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &geometry_buffers.normals.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &geometry_buffers.ids.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &frame.depth_buffer.texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);

        for (model, instances) in set.instances {
            let Some(model) = self.models.get(model) else {
                tracing::warn!("Model not found to be rendered!");
                continue;
            };

            render_pass.set_bind_group(2, &model.nodes_buffer_bind_group, &[]);

            // Create a buffer for all the instances.
            let instances_buffer = instances
                .iter()
                .map(|i| i.transform.to_cols_array_2d())
                .collect::<Vec<_>>();

            let instance_buffer =
                frame
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("models_instance_buffer"),
                        contents: bytemuck::cast_slice(&instances_buffer),
                        usage: wgpu::BufferUsages::VERTEX,
                    });

            for draw_command in model.draw_commands.iter() {
                let Some(texture) = self.textures.get(draw_command.texture) else {
                    tracing::warn!("Texture not found, skipping draw command!");
                    continue;
                };

                render_pass.set_bind_group(1, &texture.bind_group, &[]);

                render_pass
                    .set_index_buffer(model.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.set_vertex_buffer(0, model.mesh.vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
                render_pass.draw_indexed(
                    draw_command.index_range.clone(),
                    0,
                    0..(instances.len() as u32),
                );
            }
        }
    }
}

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
struct RenderModelInstance {
    transform: Mat4,
}

#[derive(Default)]
pub struct RenderModelSet {
    instances: HashMap<Handle<RenderModel>, Vec<RenderModelInstance>>,
}

impl RenderModelSet {
    pub fn push(&mut self, model: Handle<RenderModel>, transform: Mat4) {
        let instances = self.instances.entry(model).or_default();
        instances.push(RenderModelInstance { transform });
    }
}

struct RenderTexture {
    texture_view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
}

#[derive(Debug)]
struct DrawCommand {
    index_range: Range<u32>,
    texture: Handle<RenderTexture>,
}

pub struct RenderModel {
    /// A reference back to the original [Model].
    model: Asset<Model>,
    /// Contains the buffers for all the mesh data for the model.
    mesh: GpuIndexedMesh,
    /// Node data stored in a buffer for rendering.
    nodes_buffer_bind_group: wgpu::BindGroup,
    /// The draw commands that have to be issues per texture.
    draw_commands: Vec<DrawCommand>,
}

fn create_pipeline(
    renderer: &Renderer,
    shaders: &mut Shaders,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    nodes_buffer_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let module = shaders.create_shader(
        renderer,
        "model_renderer_shader",
        include_str!("model.wgsl"),
        "model.wgsl",
        Default::default(),
    );

    let layout = renderer
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("model_renderer_pipeline_layout"),
            bind_group_layouts: &[
                camera_bind_group_layout,
                renderer.texture_bind_group_layout(),
                nodes_buffer_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

    renderer
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("model_renderer_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: None,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    ModelVertex::layout(),
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<RenderModelInstance>()
                            as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![
                            4 => Float32x4,
                            5 => Float32x4,
                            6 => Float32x4,
                            7 => Float32x4,
                        ],
                    },
                ],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(
                renderer
                    .depth_buffer
                    .depth_stencil_state(wgpu::CompareFunction::LessEqual, true),
            ),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: None,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: GeometryBuffers::targets(),
            }),
            multiview: None,
            cache: None,
        })
}
