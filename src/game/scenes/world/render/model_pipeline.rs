use glam::Mat4;

use crate::{
    engine::{
        renderer::{Frame, Renderer},
        storage::Handle,
    },
    game::{
        model::Model,
        scenes::world::{
            render::{
                GeometryBuffer, ModelInstanceData, RenderModel, RenderStore, RenderVertex,
                RenderWorld,
            },
            sim_world::{Objects, SimWorld},
        },
    },
    wgsl_shader,
};

#[derive(Clone, Copy, PartialEq)]
struct RenderKey {
    render_model: Handle<RenderModel>,
}

impl Eq for RenderKey {}

impl PartialOrd for RenderKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RenderKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.render_model.cmp(&other.render_model)
    }
}

enum RenderNodeIndex {
    Base(u32),
}

bitflags::bitflags! {
    pub struct ModelRenderFlags : u32 {
        const HIGHLIGHTED = 1 << 0;
    }
}

struct ModelToRender {
    key: RenderKey,
    transform: Mat4,
    first_node_index: RenderNodeIndex,
    flags: ModelRenderFlags,
}

/// Wrapper passed to objects so they can specify what they want rendered in the scene.
pub struct RenderWrapper<'a> {
    render_store: &'a mut RenderStore,
    models_to_render: &'a mut Vec<ModelToRender>,
}

impl<'a> RenderWrapper<'a> {
    pub fn render_model(&mut self, transform: Mat4, model: Handle<Model>, flags: ModelRenderFlags) {
        let Some(render_model_handle) = self.render_store.render_model_for_model(model) else {
            tracing::warn!("Missing render model for model!");
            return;
        };

        let Some(render_model) = self.render_store.models.get(render_model_handle) else {
            tracing::warn!("Missing render model for render model handle!");
            return;
        };

        self.models_to_render.push(ModelToRender {
            key: RenderKey {
                render_model: render_model_handle,
            },
            transform,
            first_node_index: RenderNodeIndex::Base(render_model.nodes_range.start),
            flags,
        });
    }
}

struct Batch {
    key: RenderKey,
    range: std::ops::Range<u32>,
}

pub struct ModelPipeline {
    opaque_pipeline: wgpu::RenderPipeline,
    alpha_pipeline: wgpu::RenderPipeline,
    models_to_render: Vec<ModelToRender>,
    batches: Vec<Batch>,
}

impl ModelPipeline {
    pub fn new(renderer: &Renderer, render_store: &RenderStore) -> Self {
        let device = &renderer.device;

        let module = device.create_shader_module(wgsl_shader!("models"));

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("objects_pipeline_layout"),
            bind_group_layouts: &[
                &render_store.camera_bind_group_layout,
                &render_store.textures.texture_data_bind_group_layout,
                &render_store.models.nodes_bind_group_layout,
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

        let models_to_render = Vec::default();
        let batches = Vec::default();

        Self {
            opaque_pipeline,
            alpha_pipeline,

            models_to_render,
            batches,
        }
    }

    pub fn extract(&mut self, sim_world: &mut SimWorld, render_store: &mut RenderStore) {
        self.models_to_render.clear();

        let mut wrapper = RenderWrapper {
            render_store,
            models_to_render: &mut self.models_to_render,
        };

        let state = sim_world.state();
        let visible_objects = &state.visible_objects;
        let selected_objects = &state.selected_objects;
        let objects = sim_world.ecs.resource::<Objects>();

        visible_objects
            .iter()
            .filter_map(|object_handle| objects.get(*object_handle).map(|o| (o, *object_handle)))
            .for_each(|(object, handle)| {
                let mut flags = ModelRenderFlags::empty();
                flags.set(
                    ModelRenderFlags::HIGHLIGHTED,
                    selected_objects.contains(&handle),
                );
                object.gather_models_to_render(&mut wrapper, flags);
            });
    }

    pub fn prepare(&mut self, renderer: &Renderer, render_world: &mut RenderWorld) {
        self.models_to_render
            .sort_unstable_by(|a, b| a.key.cmp(&b.key));

        let model_instances: Vec<ModelInstanceData> = self
            .models_to_render
            .iter()
            .map(|instance| ModelInstanceData {
                transform: instance.transform.to_cols_array_2d(),
                first_node_index: match instance.first_node_index {
                    RenderNodeIndex::Base(i) => i,
                },
                flags: instance.flags.bits(),
                _pad: Default::default(),
            })
            .collect();

        let mut batches: Vec<Batch> = Vec::new();
        let mut start_index: usize = 0;

        while start_index < model_instances.len() {
            let key = self.models_to_render[start_index].key;
            let mut end_index = start_index + 1;
            while end_index < model_instances.len() && self.models_to_render[end_index].key == key {
                end_index += 1;
            }
            batches.push(Batch {
                key,
                range: start_index as u32..end_index as u32,
            });
            start_index = end_index;
        }

        // Upload the instances to the GPU.
        render_world
            .model_instances
            .write(renderer, &model_instances);

        self.batches = batches;
    }

    pub fn queue(
        &self,
        render_store: &RenderStore,
        render_world: &RenderWorld,
        frame: &mut Frame,
        geometry_buffer: &GeometryBuffer,
    ) {
        self.opaque_render_pass(
            &mut frame.encoder,
            geometry_buffer,
            render_store,
            render_world,
        );

        self.alpha_render_pass(
            &mut frame.encoder,
            geometry_buffer,
            render_store,
            render_world,
        );
    }

    fn setup_render_pass(
        render_pass: &mut wgpu::RenderPass,
        pipeline: &wgpu::RenderPipeline,
        render_store: &RenderStore,
        render_world: &RenderWorld,
    ) {
        render_pass.set_pipeline(pipeline);

        render_pass.set_bind_group(0, &render_world.camera_env_bind_group, &[]);
        render_pass.set_bind_group(1, &render_store.textures.texture_data_bind_group, &[]);
        render_pass.set_bind_group(2, &render_store.models.nodes_bind_group, &[]);

        render_pass.set_vertex_buffer(0, render_store.models.vertices_buffer_slice());
        render_pass.set_vertex_buffer(1, render_world.model_instances.slice(..));
        render_pass.set_index_buffer(
            render_store.models.indices_buffer_slice(),
            wgpu::IndexFormat::Uint32,
        );
    }

    fn opaque_render_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        geometry_buffer: &GeometryBuffer,
        render_store: &RenderStore,
        render_world: &RenderWorld,
    ) {
        let mut render_pass = geometry_buffer.begin_opaque_render_pass(encoder, "objects_opaque");

        Self::setup_render_pass(
            &mut render_pass,
            &self.opaque_pipeline,
            render_store,
            render_world,
        );

        for (render_model, range) in self.batches.iter().filter_map(|batch| {
            render_store
                .models
                .get(batch.key.render_model)
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
        render_store: &RenderStore,
        render_world: &RenderWorld,
    ) {
        let mut render_pass = geometry_buffer.begin_alpha_render_pass(encoder, "objects_opaque");

        Self::setup_render_pass(
            &mut render_pass,
            &self.alpha_pipeline,
            render_store,
            render_world,
        );

        // TODO: Should the blend mode pipelines each have their own set of instances? Right now
        //       the instances without alpha ranges are just filtered out. This might be good
        //       enough.
        for (render_model, range) in self.batches.iter().filter_map(|batch| {
            render_store
                .models
                .get(batch.key.render_model)
                .and_then(|render_model| {
                    (!render_model.alpha_range.is_empty())
                        .then(|| (render_model, batch.range.clone()))
                })
        }) {
            render_pass.draw_indexed(render_model.alpha_range.clone(), 0, range);
        }
    }
}
