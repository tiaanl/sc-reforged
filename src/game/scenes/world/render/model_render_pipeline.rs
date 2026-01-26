use ahash::HashMap;
use glam::Mat4;

use crate::{
    engine::{
        assets::AssetError,
        renderer::{Frame, Renderer},
        storage::Handle,
    },
    game::{
        AssetReader,
        model::Model,
        scenes::world::{
            extract::RenderSnapshot,
            render::{
                GeometryBuffer, ModelInstanceData, RenderLayouts, RenderModel, RenderVertex,
                RenderWorld, camera_render_pipeline::CameraEnvironmentLayout,
                render_pipeline::RenderPipeline,
            },
        },
    },
    wgsl_shader,
};

use super::{render_models::RenderModels, render_textures::RenderTextures};

bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct ModelRenderFlags : u32 {
        const HIGHLIGHTED = 1 << 0;
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
    textures: RenderTextures,
    models: RenderModels,

    /// Cache of model handles to render model handles.
    model_to_render_model: HashMap<Handle<Model>, Handle<RenderModel>>,

    opaque_pipeline: wgpu::RenderPipeline,
    alpha_pipeline: wgpu::RenderPipeline,

    /// Local cache for render models to render.
    render_models_cache: Vec<RenderModelToRender>,

    /// Local cache where model instance data is built from the snapshot.
    model_instances_cache: Vec<ModelInstanceData>,

    batches: Vec<Batch>,
}

impl ModelRenderPipeline {
    pub fn new(renderer: &Renderer, layouts: &mut RenderLayouts) -> Self {
        let device = &renderer.device;

        let textures = RenderTextures::new(renderer);
        let models = RenderModels::new(renderer);

        let model_to_render_model = HashMap::default();

        let module = device.create_shader_module(wgsl_shader!("models"));

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("objects_pipeline_layout"),
            bind_group_layouts: &[
                layouts.get::<CameraEnvironmentLayout>(renderer),
                &textures.texture_data_bind_group_layout,
                &models.nodes_bind_group_layout,
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
                    4 => Uint32,     // texture_data_index
                ],
            },
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<ModelInstanceData>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![
                    5 => Float32x4,  // model_mat_0
                    6 => Float32x4,  // model_mat_1
                    7 => Float32x4,  // model_mat_2
                    8 => Float32x4,  // model_mat_3
                    9 => Uint32,     // first_node_index
                    10 => Uint32,    // flags
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

        let opaque_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("opaque_objects_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vertex_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers,
            },
            primitive,
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fragment_opaque"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: GeometryBuffer::opaque_targets(),
            }),
            multiview: None,
            cache: None,
        });

        let alpha_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("alpha_objects_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vertex_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers,
            },
            primitive,
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fragment_alpha"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: GeometryBuffer::alpha_targets(),
            }),
            multiview: None,
            cache: None,
        });

        Self {
            textures,
            models,

            opaque_pipeline,
            alpha_pipeline,
            model_to_render_model,

            render_models_cache: Vec::default(),
            model_instances_cache: Vec::default(),

            batches: Vec::default(),
        }
    }

    pub fn get_or_create_render_model(
        &mut self,
        assets: &AssetReader,
        renderer: &Renderer,
        model_handle: Handle<Model>,
    ) -> Result<Handle<RenderModel>, AssetError> {
        if let Some(render_model_handle) = self.model_to_render_model.get(&model_handle) {
            return Ok(*render_model_handle);
        }

        let render_model_handle =
            self.models
                .add(assets, renderer, &mut self.textures, model_handle)?;

        // Cache the new handle.
        self.model_to_render_model
            .insert(model_handle, render_model_handle);

        Ok(render_model_handle)
    }

    #[inline]
    pub fn render_model_for_model(&self, model: Handle<Model>) -> Option<Handle<RenderModel>> {
        self.model_to_render_model.get(&model).cloned()
    }
}

impl RenderPipeline for ModelRenderPipeline {
    fn prepare(
        &mut self,
        assets: &AssetReader,
        renderer: &Renderer,
        render_world: &mut RenderWorld,
        snapshot: &RenderSnapshot,
    ) {
        if !snapshot.models.models_to_prepare.is_empty() {
            let models_to_prepare = &snapshot.models.models_to_prepare;
            tracing::info!("Preparing {} models for the GPU.", models_to_prepare.len());

            for &model_handle in models_to_prepare {
                if let Err(err) = self.get_or_create_render_model(assets, renderer, model_handle) {
                    tracing::warn!("Could not prepare model! ({err})");
                }
            }
        }

        // TODO: Don't copy all the models to render data here.
        let mut models = snapshot.models.models.clone();

        models.sort_unstable_by(|a, b| a.model.cmp(&b.model));

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

            let first_node_index = render_model.nodes_range.start;

            self.render_models_cache.push(RenderModelToRender {
                render_model: render_model_handle,
                transform: model_to_render.transform,
                first_node_index,
                flags: {
                    let mut flags = ModelRenderFlags::empty();
                    flags.set(ModelRenderFlags::HIGHLIGHTED, model_to_render.highlighted);
                    flags
                },
            });
        }

        self.model_instances_cache.clear();

        for model_to_render in self.render_models_cache.iter() {
            self.model_instances_cache.push(ModelInstanceData {
                transform: model_to_render.transform.to_cols_array_2d(),
                first_node_index: model_to_render.first_node_index,
                flags: model_to_render.flags.bits(),
                _pad: Default::default(),
            })
        }

        let mut batches: Vec<Batch> = Vec::new();
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

            batches.push(Batch {
                render_model,
                range: start_index as u32..end_index as u32,
            });
            start_index = end_index;
        }

        // Upload the instances to the GPU.
        render_world
            .model_instances
            .write(renderer, &self.model_instances_cache);

        self.batches = batches;
    }

    fn queue(
        &self,
        render_world: &RenderWorld,
        frame: &mut Frame,
        geometry_buffer: &GeometryBuffer,
        _snapshot: &RenderSnapshot,
    ) {
        self.opaque_render_pass(&mut frame.encoder, geometry_buffer, render_world);
        self.alpha_render_pass(&mut frame.encoder, geometry_buffer, render_world);
    }
}

impl ModelRenderPipeline {
    fn setup_render_pass(
        render_pass: &mut wgpu::RenderPass,
        pipeline: &wgpu::RenderPipeline,
        textures: &RenderTextures,
        models: &RenderModels,
        render_world: &RenderWorld,
    ) {
        render_pass.set_pipeline(pipeline);

        render_pass.set_bind_group(0, &render_world.camera_env_bind_group, &[]);
        render_pass.set_bind_group(1, &textures.texture_data_bind_group, &[]);
        render_pass.set_bind_group(2, &models.nodes_bind_group, &[]);

        render_pass.set_vertex_buffer(0, models.vertices_buffer_slice());
        render_pass.set_vertex_buffer(1, render_world.model_instances.slice(..));
        render_pass.set_index_buffer(models.indices_buffer_slice(), wgpu::IndexFormat::Uint32);
    }

    fn opaque_render_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        geometry_buffer: &GeometryBuffer,
        render_world: &RenderWorld,
    ) {
        let mut render_pass = geometry_buffer.begin_opaque_render_pass(encoder, "objects_opaque");

        Self::setup_render_pass(
            &mut render_pass,
            &self.opaque_pipeline,
            &self.textures,
            &self.models,
            render_world,
        );

        for (render_model, range) in self.batches.iter().filter_map(|batch| {
            self.models
                .get(batch.render_model)
                .and_then(|render_model| {
                    (!render_model.opaque_range.is_empty())
                        .then(|| (render_model, batch.range.clone()))
                })
        }) {
            render_pass.draw_indexed(render_model.opaque_range.clone(), 0, range);
        }
    }

    fn alpha_render_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        geometry_buffer: &GeometryBuffer,
        render_world: &RenderWorld,
    ) {
        let mut render_pass = geometry_buffer.begin_alpha_render_pass(encoder, "objects_opaque");

        Self::setup_render_pass(
            &mut render_pass,
            &self.alpha_pipeline,
            &self.textures,
            &self.models,
            render_world,
        );

        // TODO: Should the blend mode pipelines each have their own set of instances? Right now
        //       the instances without alpha ranges are just filtered out. This might be good
        //       enough.
        for (render_model, range) in self.batches.iter().filter_map(|batch| {
            self.models
                .get(batch.render_model)
                .and_then(|render_model| {
                    (!render_model.alpha_range.is_empty())
                        .then(|| (render_model, batch.range.clone()))
                })
        }) {
            render_pass.draw_indexed(render_model.alpha_range.clone(), 0, range);
        }
    }
}
