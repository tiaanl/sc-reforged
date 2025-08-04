use std::path::Path;

use crate::{
    engine::{prelude::*, storage::Handle},
    game::{geometry_buffers::GeometryBuffers, image::BlendMode, model},
};

use ahash::{HashMap, HashSet};
pub use gpu::ModelHandle;
use slab::Slab;
use wgpu::util::DeviceExt;

struct ModelInstance {
    model_handle: ModelHandle,
    transform: Mat4,
    entity_id: u32,
}

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
struct Instance {
    model: Mat4,
    id: u32,
    _padding: [u32; 3],
}

struct InstanceBuffer {
    buffer: wgpu::Buffer,
}

#[derive(Clone, Copy, Debug)]
pub struct ModelInstanceHandle(usize);

pub struct ModelRenderer {
    textures: gpu::Textures,
    models: gpu::Models,

    /// Keep a list of each model we have to render.
    model_instances: Slab<ModelInstance>,
    /// Keeps a list of transforms for each model instance to render.
    instance_buffers: HashMap<ModelHandle, InstanceBuffer>,
    /// A list of changed instance buffers that has to be uploaded before the next render.
    dirty_instance_buffers: HashSet<ModelHandle>,

    opaque_pipeline: wgpu::RenderPipeline,
    alpha_pipeline: wgpu::RenderPipeline,
}

impl ModelRenderer {
    pub fn new(
        shaders: &mut Shaders,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        environment_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let textures = gpu::Textures::new();
        let models = gpu::Models::new();

        let module = shaders.create_shader(
            "model_renderer",
            include_str!("model_renderer.wgsl"),
            "model_renderer.wgsl",
            Default::default(),
        );

        let layout = renderer()
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("model_renderer_pipeline_layout"),
                bind_group_layouts: &[
                    camera_bind_group_layout,
                    environment_bind_group_layout,
                    &textures.texture_bind_group_layout,
                    &models.nodes_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let buffers = &[
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<gpu::Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![
                    0 => Float32x3,
                    1 => Float32x3,
                    2 => Float32x2,
                    3 => Uint32
                ],
            },
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Instance>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![
                    4 => Float32x4,
                    5 => Float32x4,
                    6 => Float32x4,
                    7 => Float32x4,
                    8 => Uint32,
                ],
            },
        ];

        let opaque_pipeline =
            renderer()
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("model_renderer_opaque_render_pipeline"),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: None,
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers,
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        front_face: wgpu::FrontFace::Cw,
                        cull_mode: Some(wgpu::Face::Back),
                        polygon_mode: wgpu::PolygonMode::Fill,
                        ..wgpu::PrimitiveState::default()
                    },
                    depth_stencil: Some(GeometryBuffers::depth_stencil_state(
                        wgpu::CompareFunction::LessEqual,
                        true,
                    )),
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some("fragment_opaque"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: GeometryBuffers::opaque_targets(),
                    }),
                    multiview: None,
                    cache: None,
                });

        let alpha_pipeline =
            renderer()
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("model_renderer_alpha_render_pipeline"),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: None,
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers,
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        front_face: wgpu::FrontFace::Cw,
                        cull_mode: Some(wgpu::Face::Back),
                        polygon_mode: wgpu::PolygonMode::Fill,
                        ..wgpu::PrimitiveState::default()
                    },
                    depth_stencil: Some(GeometryBuffers::depth_stencil_state(
                        wgpu::CompareFunction::LessEqual,
                        true,
                    )),
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some("fragment_alpha"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: GeometryBuffers::alpha_targets(),
                    }),
                    multiview: None,
                    cache: None,
                });

        Self {
            textures,
            models,

            model_instances: Slab::default(),
            instance_buffers: HashMap::default(),
            dirty_instance_buffers: HashSet::default(),

            opaque_pipeline,
            alpha_pipeline,
        }
    }

    pub fn add_model(
        &mut self,
        model_handle: Handle<model::Model>,
    ) -> Result<ModelHandle, AssetError> {
        self.models.add_model(&mut self.textures, model_handle)
    }

    pub fn add_model_instance(
        &mut self,
        model_handle: Handle<model::Model>,
        transform: Mat4,
        entity_id: u32,
    ) -> Result<ModelInstanceHandle, AssetError> {
        let model_handle = self.models.add_model(&mut self.textures, model_handle)?;

        let model_instance_handle =
            ModelInstanceHandle(self.model_instances.insert(ModelInstance {
                transform,
                model_handle,
                entity_id,
            }));

        self.dirty_instance_buffers.insert(model_handle);

        Ok(model_instance_handle)
    }

    pub fn set_instance_transform(
        &mut self,
        instance_handle: ModelInstanceHandle,
        transform: Mat4,
    ) {
        let Some(instance) = self.model_instances.get_mut(instance_handle.0) else {
            tracing::warn!("Invalid model instance handle");
            return;
        };

        instance.transform = transform;
        // Make sure the buffer for the model is uploaded before we render again.
        self.dirty_instance_buffers.insert(instance.model_handle);
    }

    pub fn get_model(&self, model_instance_handle: ModelInstanceHandle) -> Option<&gpu::Model> {
        self.model_instances
            .get(model_instance_handle.0)
            .and_then(|instance| self.models.get(instance.model_handle))
    }

    pub fn render(
        &mut self,
        frame: &mut Frame,
        geometry_buffers: &GeometryBuffers,
        camera_bind_group: &wgpu::BindGroup,
        environment_bind_group: &wgpu::BindGroup,
    ) {
        // Make sure all the instance buffers are up to date.
        {
            for model_handle in self.dirty_instance_buffers.drain() {
                // Build all the transforms for the model handle.
                // TODO: This is n^2?!?!one
                let instances = self
                    .model_instances
                    .iter()
                    .filter(|(_, instance)| instance.model_handle == model_handle)
                    .map(|(_, instance)| Instance {
                        model: instance.transform,
                        id: instance.entity_id,
                        _padding: [0; 3],
                    })
                    .collect::<Vec<_>>();

                let buffer =
                    renderer()
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("model_renderer_instance_buffer"),
                            contents: bytemuck::cast_slice(&instances),
                            usage: wgpu::BufferUsages::VERTEX,
                        });

                if let Some(instance_buffer) = self.instance_buffers.get_mut(&model_handle) {
                    instance_buffer.buffer = buffer;
                } else {
                    self.instance_buffers
                        .insert(model_handle, InstanceBuffer { buffer });
                }
            }
        }

        {
            // Opaque
            {
                let mut render_pass =
                    frame
                        .encoder
                        .begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("model_renderer_render_pass"),
                            color_attachments: &geometry_buffers.opaque_color_attachments(),
                            depth_stencil_attachment: Some(
                                wgpu::RenderPassDepthStencilAttachment {
                                    view: &geometry_buffers.depth.view,
                                    depth_ops: Some(wgpu::Operations {
                                        load: wgpu::LoadOp::Load,
                                        store: wgpu::StoreOp::Store,
                                    }),
                                    stencil_ops: None,
                                },
                            ),
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });

                render_pass.set_pipeline(&self.opaque_pipeline);
                render_pass.set_bind_group(0, camera_bind_group, &[]);
                render_pass.set_bind_group(1, environment_bind_group, &[]);

                for (model_handle, instance_buffer) in self.instance_buffers.iter() {
                    let Some(model) = self.models.get(*model_handle) else {
                        continue;
                    };

                    render_pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));

                    model.render(&mut render_pass, &self.textures, BlendMode::Opaque);
                }
            }

            // Alpha
            {
                let mut render_pass =
                    frame
                        .encoder
                        .begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("model_renderer_render_pass"),
                            color_attachments: &geometry_buffers.alpha_color_attachments(),
                            depth_stencil_attachment: Some(
                                wgpu::RenderPassDepthStencilAttachment {
                                    view: &geometry_buffers.depth.view,
                                    depth_ops: Some(wgpu::Operations {
                                        load: wgpu::LoadOp::Load,
                                        store: wgpu::StoreOp::Discard,
                                    }),
                                    stencil_ops: None,
                                },
                            ),
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });

                render_pass.set_pipeline(&self.alpha_pipeline);
                render_pass.set_bind_group(0, camera_bind_group, &[]);
                render_pass.set_bind_group(1, environment_bind_group, &[]);

                for (model_handle, instance_buffer) in self.instance_buffers.iter() {
                    let Some(model) = self.models.get(*model_handle) else {
                        continue;
                    };

                    render_pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));

                    model.render(&mut render_pass, &self.textures, BlendMode::Alpha);
                }
            }
        }
    }
}

mod gpu {
    use super::*;

    use std::{ops::Range, path::PathBuf};

    use ahash::HashMap;
    use glam::{Mat4, Vec2, Vec3};
    use wgpu::util::DeviceExt;

    use crate::{
        engine::assets::AssetError,
        game::{image::images, models::models},
    };

    type NodeIndex = u32;

    #[derive(Clone, Copy)]
    struct TextureHandle(usize);

    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    pub struct ModelHandle(usize);

    struct Texture {
        blend_mode: BlendMode,
        bind_group: wgpu::BindGroup,
    }

    #[derive(Clone, Copy, bytemuck::NoUninit)]
    #[repr(C)]
    pub struct Vertex {
        pub position: Vec3,
        pub normal: Vec3,
        pub tex_coord: Vec2,
        pub node_index: NodeIndex,
    }

    #[derive(Clone, Copy, bytemuck::NoUninit)]
    #[repr(C)]
    struct Node {
        transform: [f32; 16],
        parent_node_index: NodeIndex,
        _padding: [u32; 3],
    }

    struct Mesh {
        indices: Range<u32>,
        texture_handle: TextureHandle,
    }

    pub struct Model {
        /// Contains the vertices for the entire model.
        vertex_buffer: wgpu::Buffer,
        /// Contains the indices for the entire model.
        index_buffer: wgpu::Buffer,
        /// Contains the node data for the entire model.
        node_buffer: wgpu::Buffer,
        /// For binding the nodes to the shader.
        nodes_bind_group: wgpu::BindGroup,
        /// All the meshes (sets of indices) that the model consists of.
        meshes: Vec<Mesh>,

        pub scale: Vec3,
    }

    impl Model {
        pub fn render(
            &self,
            render_pass: &mut wgpu::RenderPass,
            textures: &Textures,
            blend_mode: BlendMode,
        ) {
            for mesh in self.meshes.iter() {
                let Some(texture) = textures.textures.get(mesh.texture_handle.0) else {
                    tracing::warn!("Texture not in cache");
                    continue;
                };

                if texture.blend_mode != blend_mode {
                    continue;
                }

                render_pass.set_bind_group(2, &texture.bind_group, &[]);
                render_pass.set_bind_group(3, &self.nodes_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(mesh.indices.clone(), 0, 0..1);
            }
        }
    }

    /// A store/cache for textures used by the [super::ModelRenderer].
    pub struct Textures {
        textures: Slab<Texture>,
        lookup: HashMap<PathBuf, TextureHandle>,

        pub texture_bind_group_layout: wgpu::BindGroupLayout,
        sampler: wgpu::Sampler,
    }

    impl Textures {
        pub fn new() -> Self {
            let texture_bind_group_layout =
                renderer()
                    .device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: Some("model_renderer_texture_bind_group_layout"),
                        entries: &[
                            wgpu::BindGroupLayoutEntry {
                                binding: 0,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: true,
                                    },
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    multisampled: false,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 1,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                                count: None,
                            },
                        ],
                    });

            let sampler = renderer().device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("model_renderer_sampler"),
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });

            Self {
                textures: Slab::default(),
                lookup: HashMap::default(),
                texture_bind_group_layout,
                sampler,
            }
        }

        fn create_from_path(
            &mut self,
            path: impl AsRef<Path>,
        ) -> Result<TextureHandle, AssetError> {
            // If the path exists, return the existing handle.
            if let Some(texture_handle) = self.lookup.get(path.as_ref()) {
                return Ok(*texture_handle);
            }

            // We don't have the path in the cache, load it now.
            let image = images().load_image_direct(path.as_ref())?;

            let label = format!("texture_({})", path.as_ref().display());
            let size = wgpu::Extent3d {
                width: image.size.x,
                height: image.size.y,
                depth_or_array_layers: 1,
            };

            let texture = renderer().device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label.as_str()),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            renderer().queue.write_texture(
                wgpu::TexelCopyTextureInfoBase {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::default(),
                    aspect: wgpu::TextureAspect::All,
                },
                &image.data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(image.size.x * 4),
                    rows_per_image: Some(image.size.y),
                },
                size,
            );

            let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            let bind_group = renderer()
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("model_renderer_bind_group"),
                    layout: &self.texture_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                    ],
                });

            let texture_handle = self.cache_texture(Texture {
                blend_mode: image.blend_mode,
                bind_group,
            });

            Ok(texture_handle)
        }

        fn cache_texture(&mut self, texture: Texture) -> TextureHandle {
            TextureHandle(self.textures.insert(texture))
        }
    }

    pub struct Models {
        models: Slab<Model>,
        lookup: HashMap<PathBuf, ModelHandle>,

        pub nodes_bind_group_layout: wgpu::BindGroupLayout,
    }

    impl Models {
        pub fn new() -> Self {
            let nodes_bind_group_layout =
                renderer()
                    .device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: Some("model_renderer_nodes_bind_group_layout"),
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

            Self {
                models: Slab::default(),
                lookup: HashMap::default(),

                nodes_bind_group_layout,
            }
        }

        pub fn add_model(
            &mut self,
            textures: &mut Textures,
            model_handle: Handle<model::Model>,
        ) -> Result<ModelHandle, AssetError> {
            let model = models()
                .get(model_handle)
                .expect("Model should have been loaded byt his time.");

            let mut meshes = Vec::default();

            let mut vertices = Vec::default();
            let mut indices = Vec::default();

            let mut first_vertex_index = 0;

            for mesh in model.meshes.iter() {
                let texture_handle = textures.create_from_path(
                    PathBuf::from("textures")
                        .join("shared")
                        .join(&mesh.texture_name),
                )?;

                mesh.mesh
                    .vertices
                    .iter()
                    .map(|v| Vertex {
                        position: v.position,
                        normal: v.normal,
                        tex_coord: v.tex_coord,
                        node_index: v.node_index,
                    })
                    .for_each(|v| vertices.push(v));

                let first_index = indices.len() as u32;

                mesh.mesh
                    .indices
                    .iter()
                    .map(|index| index + first_vertex_index)
                    .for_each(|i| indices.push(i));

                let last_index = indices.len() as u32;

                meshes.push(Mesh {
                    indices: first_index..last_index,
                    texture_handle,
                });

                first_vertex_index = vertices.len() as u32;
            }

            let nodes: Vec<Node> = model
                .nodes
                .iter()
                .map(|node| Node {
                    // TODO: This should include the rotation?
                    transform: Mat4::from_translation(node.transform.translation).to_cols_array(),
                    parent_node_index: node.parent,
                    _padding: [0; 3],
                })
                .collect();

            let vertex_buffer =
                renderer()
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("model_vertex_buffer"),
                        contents: bytemuck::cast_slice(&vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    });

            let index_buffer =
                renderer()
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("model_index_buffer"),
                        contents: bytemuck::cast_slice(&indices),
                        usage: wgpu::BufferUsages::INDEX,
                    });

            let node_buffer =
                renderer()
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("model_node_buffer"),
                        contents: bytemuck::cast_slice(&nodes),
                        usage: wgpu::BufferUsages::STORAGE,
                    });

            let nodes_bind_group =
                renderer()
                    .device
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("model_renderer_nodes_bind_group"),
                        layout: &self.nodes_bind_group_layout,
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: &node_buffer,
                                offset: 0,
                                size: None,
                            }),
                        }],
                    });

            Ok(ModelHandle(self.models.insert(Model {
                vertex_buffer,
                index_buffer,
                node_buffer,
                nodes_bind_group,
                meshes,

                scale: model.scale,
            })))
        }

        pub fn get(&self, model_handle: ModelHandle) -> Option<&Model> {
            self.models.get(model_handle.0)
        }
    }
}
