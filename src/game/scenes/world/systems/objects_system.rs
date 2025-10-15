use crate::{
    engine::{gizmos::GizmosRenderer, prelude::*, storage::Handle},
    game::scenes::world::{
        render::{ModelInstanceData, RenderModel, RenderStore, RenderVertex, RenderWorld},
        sim_world::SimWorld,
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
    Override(u32),
}

struct ModelToRender {
    key: RenderKey,
    transform: Mat4,
    first_node_index: RenderNodeIndex,
}

struct Batch {
    key: RenderKey,
    range: std::ops::Range<u32>,
}

pub struct ObjectsSystem {
    opaque_pipeline: wgpu::RenderPipeline,
    alpha_pipeline: wgpu::RenderPipeline,
    models_to_render: Vec<ModelToRender>,
    batches: Vec<Batch>,
}

impl ObjectsSystem {
    pub fn new(renderer: &Renderer, render_store: &RenderStore) -> Self {
        let module = renderer
            .device
            .create_shader_module(wgsl_shader!("objects"));

        let layout = renderer
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
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

        let opaque_pipeline =
            renderer
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                        targets: &[Some(wgpu::ColorTargetState {
                            format: renderer.surface.format(),
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview: None,
                    cache: None,
                });

        let alpha_pipeline =
            renderer
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                        targets: &[Some(wgpu::ColorTargetState {
                            format: renderer.surface.format(),
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
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

    pub fn render_gizmos(&self, sim_world: &mut SimWorld) {
        for (_, object) in sim_world.objects.objects.iter() {
            sim_world
                .gizmo_vertices
                .extend(GizmosRenderer::create_iso_sphere(
                    object.transform.to_mat4(),
                    object.bounding_sphere.radius,
                    6,
                ));
        }
    }

    pub fn extract(&mut self, sim_world: &mut SimWorld, render_store: &mut RenderStore) {
        self.models_to_render = sim_world
            .visible_objects
            .iter()
            .filter_map(|object| {
                sim_world
                    .objects
                    .get(*object)
                    .and_then(|object| {
                        object
                            .model_to_render()
                            .map(|model_handle| (object.transform.to_mat4(), model_handle))
                    })
                    .and_then(|(transform, model_handle)| {
                        render_store.render_model_for_model(model_handle).map(
                            |render_model_handle| {
                                let render_model =
                                    render_store.models.get(render_model_handle).unwrap();

                                ModelToRender {
                                    key: RenderKey {
                                        render_model: render_model_handle,
                                    },
                                    transform,
                                    first_node_index: RenderNodeIndex::Base(
                                        render_model.nodes_range.start,
                                    ),
                                }
                            },
                        )
                    })
            })
            .collect();
    }

    pub fn prepare(&mut self, render_world: &mut RenderWorld, renderer: &Renderer) {
        self.models_to_render
            .sort_unstable_by(|a, b| a.key.cmp(&b.key));

        let model_instances: Vec<ModelInstanceData> = self
            .models_to_render
            .iter()
            .map(|instance| ModelInstanceData {
                transform: instance.transform.to_cols_array_2d(),
                first_node_index: match instance.first_node_index {
                    RenderNodeIndex::Base(i) | RenderNodeIndex::Override(i) => i,
                },
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
        depth_buffer: &wgpu::TextureView,
    ) {
        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("models_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.surface,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_buffer,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        let setup_render_pass = |render_pass: &mut wgpu::RenderPass,
                                 pipeline: &wgpu::RenderPipeline| {
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
        };

        // Opaque
        {
            setup_render_pass(&mut render_pass, &self.opaque_pipeline);

            for (render_model, range) in self.batches.iter().filter_map(|batch| {
                render_store
                    .models
                    .get(batch.key.render_model)
                    .map(|render_model| (render_model, batch.range.clone()))
            }) {
                render_pass.draw_indexed(render_model.opaque_range.clone(), 0, range);
            }
        }

        // Alpha
        {
            setup_render_pass(&mut render_pass, &self.alpha_pipeline);

            for (render_model, range) in self.batches.iter().filter_map(|batch| {
                render_store
                    .models
                    .get(batch.key.render_model)
                    .and_then(|render_model| {
                        if render_model.alpha_range.is_empty() {
                            None
                        } else {
                            Some((render_model, batch.range.clone()))
                        }
                    })
            }) {
                render_pass.draw_indexed(render_model.alpha_range.clone(), 0, range);
            }
        }
    }
}
