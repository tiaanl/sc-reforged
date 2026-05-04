use std::sync::Arc;

use ahash::HashMap;

use crate::{
    engine::{
        growing_buffer::GrowingBuffer,
        renderer::{Gpu, RenderContext, RenderTarget},
        shader_cache::ShaderCache,
        storage::Handle,
    },
    game::{
        assets::{model::Model, models::Models},
        render::{
            geometry_buffer::GeometryBuffer,
            per_frame::PerFrame,
            textures::{Texture, Textures},
            world::{
                WorldRenderSnapshot,
                camera_render_pipeline::CameraEnvironmentLayout,
                render_bindings::RenderBindings,
                render_layouts::RenderLayouts,
                render_models::{RenderMesh, RenderModel, RenderModels, RenderVertex},
                render_pipeline::RenderPipeline,
            },
        },
    },
};

bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct ModelRenderFlags : u32 {
        const HIGHLIGHTED = 1 << 0;
        const CUSTOM_POSE = 1 << 1;
    }
}

struct Batch {
    model: Handle<Model>,
    range: std::ops::Range<u32>,
}

pub struct ModelRenderPipeline {
    textures: Arc<Textures>,
    asset_models: Arc<Models>,

    models: RenderModels,

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

    /// Sorted indices into `snapshot.models.models`, grouping instances by
    /// `Handle<Model>` so they can be drawn as contiguous batches.
    sorted_indices_cache: Vec<usize>,

    /// Per-instance data uploaded to the GPU each frame.
    model_instances_cache: Vec<gpu::ModelInstanceData>,
    /// Parallel array to `model_instances_cache` holding the model handle for
    /// each instance — used to compute batch ranges after the fill pass.
    instance_models_cache: Vec<Handle<Model>>,
    model_instances: PerFrame<GrowingBuffer<gpu::ModelInstanceData>>,

    /// Local cache where custom Pose data is stored per frame.
    poses_bind_group_layout: wgpu::BindGroupLayout,
    poses_cache: Vec<gpu::Bone>,
    poses: PerFrame<(GrowingBuffer<gpu::Bone>, wgpu::BindGroup)>,

    batches: Vec<Batch>,
}

impl ModelRenderPipeline {
    pub fn new(
        gpu: &Gpu,
        layouts: &mut RenderLayouts,
        shader_cache: &mut ShaderCache,
        textures: Arc<Textures>,
        asset_models: Arc<Models>,
    ) -> Self {
        let device = &gpu.device;

        let models = RenderModels::new(gpu);

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
            &gpu.device,
            crate::engine::shader_cache::ShaderSource::Models,
        );

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("models_pipeline_layout"),
            bind_group_layouts: &[
                layouts.get::<CameraEnvironmentLayout>(),
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
                array_stride: std::mem::size_of::<
                    super::model_render_pipeline::gpu::ModelInstanceData,
                >() as wgpu::BufferAddress,
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
                gpu,
                1 << 7,
                wgpu::BufferUsages::VERTEX,
                format!("model_instances:{index}"),
            )
        });

        let poses = PerFrame::new(|index| {
            let buffer = GrowingBuffer::new(
                gpu,
                1 << 7,
                wgpu::BufferUsages::STORAGE,
                format!("poses:{index}"),
            );

            let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
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

            texture_bind_group_layout,
            sampler,
            texture_bind_groups: HashMap::default(),

            opaque_pipeline,
            keyed_pipeline,
            alpha_pipeline,

            sorted_indices_cache: Vec::default(),

            model_instances_cache: Vec::default(),
            instance_models_cache: Vec::default(),
            model_instances,

            poses_bind_group_layout,
            poses_cache: Vec::default(),
            poses,

            batches: Vec::default(),
        }
    }

    pub fn ensure_render_model(&mut self, gpu: &Gpu, model_handle: Handle<Model>) {
        if self.models.contains(model_handle) {
            return;
        }

        self.models
            .add(&self.textures, &self.asset_models, gpu, model_handle);

        // Create per-texture bind groups for any new textures introduced by this model.
        let new_textures: Vec<Handle<Texture>> = self
            .models
            .get(model_handle)
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
            self.ensure_texture_bind_group(&gpu.device, texture);
        }
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
}

impl RenderPipeline for ModelRenderPipeline {
    fn prepare(
        &mut self,
        gpu: &Gpu,
        _bindings: &mut RenderBindings,
        snapshot: &WorldRenderSnapshot,
    ) {
        let snapshot_models = &snapshot.models.models;

        // Sort by model handle so instances of the same model end up contiguous.
        // We sort indices into the snapshot rather than cloning the snapshot itself.
        self.sorted_indices_cache.clear();
        self.sorted_indices_cache.extend(0..snapshot_models.len());
        self.sorted_indices_cache
            .sort_unstable_by_key(|&i| snapshot_models[i].model);

        self.poses_cache.clear();
        self.model_instances_cache.clear();
        self.instance_models_cache.clear();

        // Walk the sorted snapshot, lazily preparing each new model's GPU data
        // on first sight, and emitting per-instance data inline.
        for i in 0..self.sorted_indices_cache.len() {
            let idx = self.sorted_indices_cache[i];
            let m = &snapshot_models[idx];

            self.ensure_render_model(gpu, m.model);

            let mut flags = ModelRenderFlags::empty();
            flags.set(ModelRenderFlags::HIGHLIGHTED, m.highlighted);

            let first_node_index = if let Some(ref pose) = m.pose {
                let first = self.poses_cache.len() as u32;
                for bone in pose.bones.iter() {
                    self.poses_cache.push(gpu::Bone {
                        transform: bone.to_cols_array_2d(),
                    });
                }
                flags.set(ModelRenderFlags::CUSTOM_POSE, true);
                first
            } else {
                // Default-pose draws read from the per-model nodes buffer, which
                // always starts at index 0.
                0
            };

            self.model_instances_cache.push(gpu::ModelInstanceData {
                transform: m.transform.to_cols_array_2d(),
                first_node_index,
                flags: flags.bits(),
                _pad: Default::default(),
            });
            self.instance_models_cache.push(m.model);
        }

        // Compute batches by walking the (sorted) parallel `instance_models_cache`.
        self.batches.clear();
        let instances_count = self.instance_models_cache.len();
        let mut start = 0;
        while start < instances_count {
            let model = self.instance_models_cache[start];
            let mut end = start + 1;
            while end < instances_count && self.instance_models_cache[end] == model {
                end += 1;
            }
            self.batches.push(Batch {
                model,
                range: start as u32..end as u32,
            });
            start = end;
        }

        // Upload custom poses; rebuild the bind group if the buffer was reallocated.
        {
            let (buffer, bind_group) = self.poses.advance();

            if buffer.write(gpu, self.poses_cache.as_slice()) {
                *bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("poses_bind_group"),
                    layout: &self.poses_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.buffer().as_entire_binding(),
                    }],
                });
            }
        }

        // Upload instances.
        let model_instances = self.model_instances.advance();
        model_instances.write(gpu, self.model_instances_cache.as_slice());
    }

    fn queue(
        &self,
        bindings: &RenderBindings,
        render_context: &mut RenderContext,
        _render_target: &RenderTarget,
        geometry_buffer: &GeometryBuffer,
        _snapshot: &WorldRenderSnapshot,
    ) {
        self.opaque_render_pass(&mut render_context.encoder, geometry_buffer, bindings);
        self.alpha_render_pass(&mut render_context.encoder, geometry_buffer, bindings);
    }
}

impl ModelRenderPipeline {
    fn bind_pass_resources(&self, render_pass: &mut wgpu::RenderPass, bindings: &RenderBindings) {
        render_pass.set_bind_group(0, &bindings.camera_env_buffer.current().bind_group, &[]);
        render_pass.set_bind_group(3, &self.poses.current().1, &[]);
        render_pass.set_vertex_buffer(1, self.model_instances.current().slice(..));
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

    fn run_pass<F>(
        &self,
        render_pass: &mut wgpu::RenderPass,
        pipeline: &wgpu::RenderPipeline,
        select_meshes: F,
    ) where
        F: Fn(&RenderModel) -> &[RenderMesh],
    {
        render_pass.set_pipeline(pipeline);
        for batch in self.batches.iter() {
            let Some(render_model) = self.models.get(batch.model) else {
                continue;
            };
            let meshes = select_meshes(render_model);
            if meshes.is_empty() {
                continue;
            }
            render_pass.set_bind_group(2, &render_model.nodes_bind_group, &[]);
            render_pass.set_vertex_buffer(0, render_model.vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                render_model.index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            self.draw_meshes(render_pass, meshes, batch.range.clone());
        }
    }

    fn opaque_render_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        geometry_buffer: &GeometryBuffer,
        bindings: &RenderBindings,
    ) {
        let mut render_pass = geometry_buffer.begin_opaque_render_pass(encoder, "models_opaque");
        self.bind_pass_resources(&mut render_pass, bindings);

        self.run_pass(&mut render_pass, &self.opaque_pipeline, |m| {
            &m.opaque_meshes
        });
        self.run_pass(&mut render_pass, &self.keyed_pipeline, |m| &m.keyed_meshes);
    }

    fn alpha_render_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        geometry_buffer: &GeometryBuffer,
        bindings: &RenderBindings,
    ) {
        let mut render_pass = geometry_buffer.begin_alpha_render_pass(encoder, "models_alpha");
        self.bind_pass_resources(&mut render_pass, bindings);

        self.run_pass(&mut render_pass, &self.alpha_pipeline, |m| &m.alpha_meshes);
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
