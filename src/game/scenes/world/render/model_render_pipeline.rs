use std::sync::Arc;

use ahash::HashMap;
use glam::Mat4;

use crate::{
    engine::{
        assets::AssetError,
        growing_buffer::GrowingBuffer,
        renderer::{Frame, RenderContext},
        shader_cache::ShaderCache,
        storage::Handle,
    },
    game::{
        assets::{model::Model, models::Models},
        render::textures::{Texture, Textures},
        scenes::world::{
            extract::RenderSnapshot,
            render::{
                GeometryBuffer, RenderBindings, RenderLayouts, RenderModel, RenderVertex,
                camera_render_pipeline::CameraEnvironmentLayout, model_render_pipeline,
                per_frame::PerFrame, render_models::RenderMesh, render_pipeline::RenderPipeline,
            },
        },
    },
};

use super::render_models::RenderModels;

bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct ModelRenderFlags : u32 {
        const HIGHLIGHTED = 1 << 0;
        const CUSTOM_POSE = 1 << 1;
    }
}

pub struct RenderModelToRender {
    pub render_model: Handle<RenderModel>,
    pub transform: Mat4,
    pub first_node_index: u32,
    pub flags: ModelRenderFlags,
}

struct Batch {
    render_model: Handle<RenderModel>,
    range: std::ops::Range<u32>,
}

pub struct ModelRenderPipeline {
    textures: Arc<Textures>,
    asset_models: Arc<Models>,

    models: RenderModels,

    /// Cache of model handles to render model handles.
    model_to_render_model: HashMap<Handle<Model>, Handle<RenderModel>>,

    /// Layout used for per-texture bind groups (texture + sampler).
    texture_bind_group_layout: wgpu::BindGroupLayout,
    /// Sampler shared across all model textures.
    sampler: wgpu::Sampler,
    /// Per-texture bind groups, lazily created when a texture is first used.
    texture_bind_groups: HashMap<Handle<Texture>, wgpu::BindGroup>,

    /// Pipeline used for `BlendMode::Opaque` meshes.
    opaque_pipeline: wgpu::RenderPipeline,
    /// Pipeline used for `BlendMode::ColorKeyed` meshes (opaque pass + discard).
    keyed_pipeline: wgpu::RenderPipeline,
    /// Pipeline used for `BlendMode::Alpha` meshes.
    alpha_pipeline: wgpu::RenderPipeline,

    /// Local cache for render models to render.
    render_models_cache: Vec<RenderModelToRender>,

    /// Local cache where model instance data is built from the snapshot.
    model_instances_cache: Vec<gpu::ModelInstanceData>,
    model_instances: PerFrame<GrowingBuffer<gpu::ModelInstanceData>>,

    /// Local cache where custom Pose data is stored per frame.
    poses_bind_group_layout: wgpu::BindGroupLayout,
    poses_cache: Vec<gpu::Bone>,
    poses: PerFrame<(GrowingBuffer<gpu::Bone>, wgpu::BindGroup)>,

    batches: Vec<Batch>,
}

impl ModelRenderPipeline {
    pub fn new(
        context: &RenderContext,
        layouts: &mut RenderLayouts,
        shader_cache: &mut ShaderCache,
        textures: Arc<Textures>,
        asset_models: Arc<Models>,
    ) -> Self {
        let device = &context.device;

        let models = RenderModels::new(context);

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("model_texture_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
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

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("model_texture_sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let poses_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("poses_bind_group_layout"),
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

        let module = shader_cache.get_or_create(
            &context.device,
            crate::engine::shader_cache::ShaderSource::Models,
        );

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("models_pipeline_layout"),
            bind_group_layouts: &[
                layouts.get::<CameraEnvironmentLayout>(context),
                &texture_bind_group_layout,
                &models.nodes_bind_group_layout,
                &poses_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let buffers = &[
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<RenderVertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![
                    0 => Float32x3,  // position
                    1 => Float32x3,  // normal
                    2 => Float32x2,  // tex_coord
                    3 => Uint32,     // node_index
                ],
            },
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<model_render_pipeline::gpu::ModelInstanceData>()
                    as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![
                    4 => Float32x4,  // model_mat_0
                    5 => Float32x4,  // model_mat_1
                    6 => Float32x4,  // model_mat_2
                    7 => Float32x4,  // model_mat_3
                    8 => Uint32,     // first_node_index
                    9 => Uint32,     // flags
                ],
            },
        ];

        let primitive = wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Cw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            ..Default::default()
        };

        let opaque_depth = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };

        let alpha_depth = wgpu::DepthStencilState {
            depth_write_enabled: false,
            ..opaque_depth.clone()
        };

        let opaque_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("models_opaque_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module,
                entry_point: Some("vertex_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers,
            },
            primitive,
            depth_stencil: Some(opaque_depth.clone()),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module,
                entry_point: Some("fragment_opaque"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: GeometryBuffer::opaque_targets(),
            }),
            multiview: None,
            cache: None,
        });

        let keyed_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("models_keyed_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module,
                entry_point: Some("vertex_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers,
            },
            primitive,
            depth_stencil: Some(opaque_depth),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module,
                entry_point: Some("fragment_opaque_keyed"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: GeometryBuffer::opaque_targets(),
            }),
            multiview: None,
            cache: None,
        });

        let alpha_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("models_alpha_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module,
                entry_point: Some("vertex_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers,
            },
            primitive,
            depth_stencil: Some(alpha_depth),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module,
                entry_point: Some("fragment_alpha"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: GeometryBuffer::alpha_targets(),
            }),
            multiview: None,
            cache: None,
        });

        let model_instances = PerFrame::new(|index| {
            GrowingBuffer::new(
                context,
                1 << 7,
                wgpu::BufferUsages::VERTEX,
                format!("model_instances:{index}"),
            )
        });

        let poses = PerFrame::new(|index| {
            let buffer = GrowingBuffer::new(
                context,
                1 << 7,
                wgpu::BufferUsages::STORAGE,
                format!("poses:{index}"),
            );

            let bind_group = context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(&format!("poses_bind_group:{index}")),
                    layout: &poses_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.buffer().as_entire_binding(),
                    }],
                });

            (buffer, bind_group)
        });

        Self {
            textures,
            asset_models,

            models,

            model_to_render_model: HashMap::default(),

            texture_bind_group_layout,
            sampler,
            texture_bind_groups: HashMap::default(),

            opaque_pipeline,
            keyed_pipeline,
            alpha_pipeline,

            render_models_cache: Vec::default(),

            model_instances_cache: Vec::default(),
            model_instances,

            poses_bind_group_layout,
            poses_cache: Vec::default(),
            poses,

            batches: Vec::default(),
        }
    }

    pub fn get_or_create_render_model(
        &mut self,
        context: &RenderContext,
        model_handle: Handle<Model>,
    ) -> Result<Handle<RenderModel>, AssetError> {
        if let Some(render_model_handle) = self.model_to_render_model.get(&model_handle) {
            return Ok(*render_model_handle);
        }

        let render_model_handle =
            self.models
                .add(&self.textures, &self.asset_models, context, model_handle)?;

        // Create per-texture bind groups for any new textures introduced by this model.
        let new_textures: Vec<Handle<Texture>> = self
            .models
            .get(render_model_handle)
            .map(|render_model| {
                render_model
                    .opaque_meshes
                    .iter()
                    .chain(render_model.keyed_meshes.iter())
                    .chain(render_model.alpha_meshes.iter())
                    .map(|mesh| mesh.texture)
                    .collect()
            })
            .unwrap_or_default();

        for texture in new_textures {
            self.ensure_texture_bind_group(&context.device, texture);
        }

        self.model_to_render_model
            .insert(model_handle, render_model_handle);

        Ok(render_model_handle)
    }

    fn ensure_texture_bind_group(&mut self, device: &wgpu::Device, texture: Handle<Texture>) {
        if self.texture_bind_groups.contains_key(&texture) {
            return;
        }

        let Some(texture_data) = self.textures.get(texture) else {
            return;
        };

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model_texture_bind_group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_data.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        self.texture_bind_groups.insert(texture, bind_group);
    }

    #[inline]
    pub fn render_model_for_model(&self, model: Handle<Model>) -> Option<Handle<RenderModel>> {
        self.model_to_render_model.get(&model).cloned()
    }
}

impl RenderPipeline for ModelRenderPipeline {
    fn prepare(
        &mut self,
        context: &RenderContext,
        _bindings: &mut RenderBindings,
        snapshot: &RenderSnapshot,
    ) {
        if !snapshot.models.models_to_prepare.is_empty() {
            let models_to_prepare = &snapshot.models.models_to_prepare;
            tracing::info!("Preparing {} models for the GPU.", models_to_prepare.len());

            for &model_handle in models_to_prepare {
                if let Err(err) = self.get_or_create_render_model(context, model_handle) {
                    tracing::warn!("Could not prepare model! ({err})");
                }
            }
        }

        // TODO: Don't copy all the models to render data here.
        let mut models = snapshot.models.models.clone();

        models.sort_unstable_by(|a, b| a.model.cmp(&b.model));

        self.poses_cache.clear();
        self.render_models_cache.clear();

        // Build an intermediate list of render models to render with some of the details resolved.
        for model_to_render in models.iter() {
            let Some(render_model_handle) = self.render_model_for_model(model_to_render.model)
            else {
                continue;
            };

            let Some(render_model) = self.models.get(render_model_handle) else {
                continue;
            };

            let mut flags = ModelRenderFlags::empty();
            flags.set(ModelRenderFlags::HIGHLIGHTED, model_to_render.highlighted);

            let first_node_index = if let Some(ref pose) = model_to_render.pose {
                let first_node = self.poses_cache.len() as u32;

                pose.bones.iter().for_each(|bone| {
                    self.poses_cache.push(gpu::Bone {
                        transform: bone.to_cols_array_2d(),
                    });
                });

                flags.set(ModelRenderFlags::CUSTOM_POSE, true);

                first_node
            } else {
                render_model.nodes_range.start
            };

            self.render_models_cache.push(RenderModelToRender {
                render_model: render_model_handle,
                transform: model_to_render.transform,
                first_node_index,
                flags,
            });
        }

        {
            // Write all the custom poses to the GPU.
            let (buffer, bind_group) = self.poses.advance();

            if buffer.write(context, self.poses_cache.as_slice()) {
                *bind_group = context
                    .device
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("poses_bind_group"),
                        layout: &self.poses_bind_group_layout,
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: buffer.buffer().as_entire_binding(),
                        }],
                    });
            }
        }

        self.model_instances_cache.clear();

        for model_to_render in self.render_models_cache.iter() {
            self.model_instances_cache.push(gpu::ModelInstanceData {
                transform: model_to_render.transform.to_cols_array_2d(),
                first_node_index: model_to_render.first_node_index,
                flags: model_to_render.flags.bits(),
                _pad: Default::default(),
            })
        }

        self.batches.clear();
        let mut start_index: usize = 0;

        let instances_count = self.render_models_cache.len();
        while start_index < instances_count {
            let render_model = self.render_models_cache[start_index].render_model;

            let mut end_index = start_index + 1;
            while end_index < instances_count
                && self.render_models_cache[end_index].render_model == render_model
            {
                end_index += 1;
            }

            self.batches.push(Batch {
                render_model,
                range: start_index as u32..end_index as u32,
            });
            start_index = end_index;
        }

        // Upload the instances to the GPU.
        let model_instances = self.model_instances.advance();
        model_instances.write(context, self.model_instances_cache.as_slice());
    }

    fn queue(
        &self,
        bindings: &RenderBindings,
        frame: &mut Frame,
        geometry_buffer: &GeometryBuffer,
        _snapshot: &RenderSnapshot,
    ) {
        self.opaque_render_pass(&mut frame.encoder, geometry_buffer, bindings);
        self.alpha_render_pass(&mut frame.encoder, geometry_buffer, bindings);
    }
}

impl ModelRenderPipeline {
    fn bind_static_resources(&self, render_pass: &mut wgpu::RenderPass, bindings: &RenderBindings) {
        render_pass.set_bind_group(0, &bindings.camera_env_buffer.current().bind_group, &[]);
        render_pass.set_bind_group(2, &self.models.nodes_bind_group, &[]);
        render_pass.set_bind_group(3, &self.poses.current().1, &[]);

        render_pass.set_vertex_buffer(0, self.models.vertices_buffer_slice());
        render_pass.set_vertex_buffer(1, self.model_instances.current().slice(..));
        render_pass.set_index_buffer(
            self.models.indices_buffer_slice(),
            wgpu::IndexFormat::Uint32,
        );
    }

    fn draw_meshes(
        &self,
        render_pass: &mut wgpu::RenderPass,
        meshes: &[RenderMesh],
        instance_range: std::ops::Range<u32>,
    ) {
        for mesh in meshes {
            let Some(bind_group) = self.texture_bind_groups.get(&mesh.texture) else {
                continue;
            };
            render_pass.set_bind_group(1, bind_group, &[]);
            render_pass.draw_indexed(mesh.index_range.clone(), 0, instance_range.clone());
        }
    }

    fn opaque_render_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        geometry_buffer: &GeometryBuffer,
        bindings: &RenderBindings,
    ) {
        let mut render_pass = geometry_buffer.begin_opaque_render_pass(encoder, "models_opaque");

        self.bind_static_resources(&mut render_pass, bindings);

        // Opaque (non-keyed) meshes.
        render_pass.set_pipeline(&self.opaque_pipeline);
        for batch in self.batches.iter() {
            let Some(render_model) = self.models.get(batch.render_model) else {
                continue;
            };
            self.draw_meshes(
                &mut render_pass,
                &render_model.opaque_meshes,
                batch.range.clone(),
            );
        }

        // Color-keyed meshes (still in the opaque pass; shader discards near-black pixels).
        render_pass.set_pipeline(&self.keyed_pipeline);
        for batch in self.batches.iter() {
            let Some(render_model) = self.models.get(batch.render_model) else {
                continue;
            };
            self.draw_meshes(
                &mut render_pass,
                &render_model.keyed_meshes,
                batch.range.clone(),
            );
        }
    }

    fn alpha_render_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        geometry_buffer: &GeometryBuffer,
        bindings: &RenderBindings,
    ) {
        let mut render_pass = geometry_buffer.begin_alpha_render_pass(encoder, "models_alpha");

        self.bind_static_resources(&mut render_pass, bindings);

        render_pass.set_pipeline(&self.alpha_pipeline);
        for batch in self.batches.iter() {
            let Some(render_model) = self.models.get(batch.render_model) else {
                continue;
            };
            self.draw_meshes(
                &mut render_pass,
                &render_model.alpha_meshes,
                batch.range.clone(),
            );
        }
    }
}

pub mod gpu {
    use bytemuck::NoUninit;

    #[derive(Clone, Copy, NoUninit)]
    #[repr(C)]
    pub struct ModelInstanceData {
        pub transform: [[f32; 4]; 4],
        pub first_node_index: u32,
        pub flags: u32,
        pub _pad: [u32; 2],
    }

    #[derive(Clone, Copy, NoUninit)]
    #[repr(C)]
    pub struct Bone {
        pub transform: [[f32; 4]; 4],
    }
}
