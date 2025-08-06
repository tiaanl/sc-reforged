use std::path::Path;

use crate::{
    engine::{prelude::*, storage::Handle},
    game::{
        animations::{Animation, Animations, animations},
        geometry_buffers::GeometryBuffers,
        image::BlendMode,
        model,
        models::models,
    },
};

use ahash::{HashMap, HashSet};
pub use gpu::ModelHandle;
use slab::Slab;
use wgpu::util::DeviceExt;

struct AnimationDescription {
    handle: Handle<Animation>,
    time: f32,
    repeat: bool,
}

struct ModelInstance {
    model_handle: Handle<model::Model>,
    gpu_model_handle: ModelHandle,
    transform: Mat4,
    entity_id: u32,
    animation_description: Option<AnimationDescription>,
}

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
struct Instance {
    model: Mat4,
    id: u32,
    _padding: [u32; 3],
}

/// A gpu buffer holding a set of [Instance]'s to render per model.
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

    pub fn add_model_instance(
        &mut self,
        model_handle: Handle<model::Model>,
        transform: Mat4,
        entity_id: u32,
    ) -> Result<ModelInstanceHandle, AssetError> {
        let gpu_model_handle = self.models.add_model(&mut self.textures, model_handle)?;

        let model_instance_handle =
            ModelInstanceHandle(self.model_instances.insert(ModelInstance {
                model_handle,
                transform,
                gpu_model_handle,
                entity_id,
                animation_description: None,
            }));

        self.dirty_instance_buffers.insert(gpu_model_handle);

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
        self.dirty_instance_buffers
            .insert(instance.gpu_model_handle);
    }

    pub fn set_instance_animation(
        &mut self,
        instance_handle: ModelInstanceHandle,
        animation: Handle<Animation>,
    ) {
        let Some(instance) = self.model_instances.get_mut(instance_handle.0) else {
            tracing::warn!("Invalid model instance handle");
            return;
        };

        instance.animation_description = Some(AnimationDescription {
            handle: animation,
            time: 0.0,
            repeat: true,
        });

        // Make sure the buffer for the model is uploaded before we render again.
        self.dirty_instance_buffers
            .insert(instance.gpu_model_handle);
    }

    pub fn clear_instance_animation(&mut self, instance_handle: ModelInstanceHandle) {
        let Some(instance) = self.model_instances.get_mut(instance_handle.0) else {
            tracing::warn!("Invalid model instance handle");
            return;
        };

        instance.animation_description = None;
        // Make sure the buffer for the model is uploaded before we render again.
        self.dirty_instance_buffers
            .insert(instance.gpu_model_handle);
    }

    pub fn get_model(&self, model_instance_handle: ModelInstanceHandle) -> Option<&gpu::Model> {
        self.model_instances
            .get(model_instance_handle.0)
            .and_then(|instance| self.models.get(instance.gpu_model_handle))
    }

    pub fn update(&mut self, delta_time: f32) {
        self.model_instances.iter_mut().for_each(|(_, instance)| {
            if instance.animation_description.is_none() {
                if let Some(gpu_model) = self.models.get_mut(instance.gpu_model_handle) {
                    gpu_model.animated_nodes_bind_group = None;
                }
                return;
            }

            let animation_description = instance.animation_description.as_mut().unwrap();

            animation_description.time += delta_time * Animations::ANIMATION_RATE;

            // SAFETY: We can unwrap the animation, because we're filtering the list already.
            if let Some(animation) = animations().get(animation_description.handle) {
                if let Some(model) = models().get(instance.model_handle) {
                    let pose = animation.sample_pose(animation_description.time, &model.nodes);

                    debug_assert!(pose.len() == model.nodes.len());

                    fn local_transform(nodes: &[gpu::Node], node_index: u32) -> Mat4 {
                        let node = &nodes[node_index as usize];
                        if node.parent_node_index == u32::MAX {
                            Mat4::from_cols_array(&node.transform)
                        } else {
                            local_transform(nodes, node.parent_node_index)
                                * Mat4::from_cols_array(&node.transform)
                        }
                    }

                    let temp_nodes: Vec<gpu::Node> = pose
                        .iter()
                        .enumerate()
                        .map(|(node_index, sample)| gpu::Node {
                            transform: Mat4::from_rotation_translation(
                                sample.rotation.unwrap(),
                                sample.position.unwrap(),
                            )
                            .to_cols_array(),
                            parent_node_index: model.nodes[node_index].parent,
                            _padding: [0; 3],
                        })
                        .collect();

                    let nodes: Vec<gpu::Node> = temp_nodes
                        .iter()
                        .enumerate()
                        .map(|(node_index, node)| gpu::Node {
                            transform: local_transform(&temp_nodes, node_index as u32)
                                .to_cols_array(),
                            parent_node_index: node.parent_node_index,
                            _padding: [0; 3],
                        })
                        .collect();

                    // Create a new nodes buffer for the instance.
                    let node_buffer =
                        renderer()
                            .device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("model_renderer_animated_node_buffer"),
                                contents: bytemuck::cast_slice(&nodes),
                                usage: wgpu::BufferUsages::STORAGE,
                            });

                    let bind_group =
                        renderer()
                            .device
                            .create_bind_group(&wgpu::BindGroupDescriptor {
                                label: Some("model_renderer_animated_node_buffer_bind_group"),
                                layout: &self.models.nodes_bind_group_layout,
                                entries: &[wgpu::BindGroupEntry {
                                    binding: 0,
                                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                        buffer: &node_buffer,
                                        offset: 0,
                                        size: None,
                                    }),
                                }],
                            });

                    if let Some(gpu_model) = self.models.get_mut(instance.gpu_model_handle) {
                        gpu_model.animated_nodes_bind_group = Some(bind_group);
                    }
                }
            }
        });
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
                    .filter(|(_, instance)| instance.gpu_model_handle == model_handle)
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
                            color_attachments: &geometry_buffers.color_attachments(),
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
                    model.render(&mut render_pass, &self.textures, BlendMode::ColorKeyed);
                }
            }

            // Alpha
            {
                let mut render_pass =
                    frame
                        .encoder
                        .begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("model_renderer_render_pass"),
                            color_attachments: &geometry_buffers.color_attachments(),
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
    use glam::{Vec2, Vec3};
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
    pub struct Node {
        pub transform: [f32; 16],
        pub parent_node_index: NodeIndex,
        pub _padding: [u32; 3],
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
        _node_buffer: wgpu::Buffer,
        /// For binding the nodes to the shader.
        nodes_bind_group: wgpu::BindGroup,

        pub animated_nodes_bind_group: Option<wgpu::BindGroup>,

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
                if let Some(ref animated_bind_group) = self.animated_nodes_bind_group {
                    render_pass.set_bind_group(3, animated_bind_group, &[]);
                } else {
                    render_pass.set_bind_group(3, &self.nodes_bind_group, &[]);
                }
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
        _lookup: HashMap<PathBuf, ModelHandle>,

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
                _lookup: HashMap::default(),

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
                .enumerate()
                .map(|(node_index, node)| {
                    let transform = model.local_transform(node_index as u32);
                    Node {
                        transform: transform.to_cols_array(),
                        parent_node_index: node.parent,
                        _padding: [0; 3],
                    }
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

            let _node_buffer =
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
                                buffer: &_node_buffer,
                                offset: 0,
                                size: None,
                            }),
                        }],
                    });

            Ok(ModelHandle(self.models.insert(Model {
                vertex_buffer,
                index_buffer,
                _node_buffer,
                nodes_bind_group,
                animated_nodes_bind_group: None,
                meshes,

                scale: model.scale,
            })))
        }

        pub fn get(&self, model_handle: ModelHandle) -> Option<&Model> {
            self.models.get(model_handle.0)
        }

        pub fn get_mut(&mut self, model_handle: ModelHandle) -> Option<&mut Model> {
            self.models.get_mut(model_handle.0)
        }
    }
}
