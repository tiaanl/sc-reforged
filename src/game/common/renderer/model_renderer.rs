use crate::{
    engine::{prelude::*, storage::Handle},
    game::{
        animations::{Animation, animations},
        geometry_buffers::GeometryBuffers,
        image::BlendMode,
        model,
        models::models,
        renderer::{
            render_models::{RenderModel, RenderModels, RenderNode, RenderVertex},
            render_textures,
        },
    },
};

use ahash::{HashMap, HashSet};
use slab::Slab;
use wgpu::util::DeviceExt;

struct AnimationDescription {
    handle: Handle<Animation>,
    time: f32,
}

struct ModelInstance {
    model_handle: Handle<model::Model>,
    render_model_handle: Handle<RenderModel>,
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

#[derive(Default)]
pub struct InstanceUpdater {
    transform: Option<Mat4>,
    animation: Option<Handle<Animation>>,
    /// If `true`, the even if an animation was specified, the animation handle will be cleared.
    clear_animation: bool,
}

impl InstanceUpdater {
    pub fn set_transform(&mut self, transform: Mat4) {
        self.transform = Some(transform);
    }

    pub fn set_animation(&mut self, animation: Handle<Animation>) {
        self.animation = Some(animation);
    }

    pub fn clear_animation(&mut self) {
        self.clear_animation = true;
    }
}

pub struct ModelRenderer {
    textures: render_textures::RenderTextures,
    models: RenderModels,

    /// Keep a list of each model we have to render.
    model_instances: Slab<ModelInstance>,
    /// Keeps a list of transforms for each model instance to render.
    instance_buffers: HashMap<Handle<RenderModel>, InstanceBuffer>,
    /// A list of changed instance buffers that has to be uploaded before the next render.
    dirty_instance_buffers: HashSet<Handle<RenderModel>>,

    opaque_pipeline: wgpu::RenderPipeline,
    alpha_pipeline: wgpu::RenderPipeline,
}

impl ModelRenderer {
    pub fn new(
        shaders: &mut Shaders,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        environment_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let textures = render_textures::RenderTextures::new();
        let models = RenderModels::new();

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
                array_stride: std::mem::size_of::<RenderVertex>() as wgpu::BufferAddress,
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
        let render_model_handle = self.models.add_model(&mut self.textures, model_handle)?;

        let model_instance_handle =
            ModelInstanceHandle(self.model_instances.insert(ModelInstance {
                model_handle,
                transform,
                render_model_handle,
                entity_id,
                animation_description: None,
            }));

        self.dirty_instance_buffers.insert(render_model_handle);

        Ok(model_instance_handle)
    }

    pub fn update_instance(
        &mut self,
        model_instance_handle: ModelInstanceHandle,
        mut update: impl FnMut(&mut InstanceUpdater),
    ) {
        let mut updater = InstanceUpdater::default();

        update(&mut updater);

        let Some(instance) = self.model_instances.get_mut(model_instance_handle.0) else {
            tracing::warn!("Invalid model instance handle to update.");
            return;
        };

        let mut dirty = false;

        if let Some(transform) = updater.transform {
            instance.transform = transform;
            dirty = true;
        }

        if updater.clear_animation {
            instance.animation_description = None;
            dirty = true;
        } else if let Some(animation) = updater.animation {
            instance.animation_description = Some(AnimationDescription {
                handle: animation,
                time: 0.0,
            });
            dirty = true;
        }

        if dirty {
            // Tag the instance for updating.
            self.dirty_instance_buffers
                .insert(instance.render_model_handle);
        }
    }

    pub fn get_model(&self, model_instance_handle: ModelInstanceHandle) -> Option<&RenderModel> {
        self.model_instances
            .get(model_instance_handle.0)
            .and_then(|instance| self.models.get(instance.render_model_handle))
    }

    pub fn update(&mut self, delta_time: f32) {
        self.model_instances.iter_mut().for_each(|(_, instance)| {
            if instance.animation_description.is_none() {
                if let Some(render_model) = self.models.get_mut(instance.render_model_handle) {
                    render_model.animated_nodes = None;
                }
                return;
            }

            let animation_description = instance.animation_description.as_mut().unwrap();

            animation_description.time += delta_time;

            // SAFETY: We can unwrap the animation, because we're filtering the list already.
            if let Some(animation) = animations().get(animation_description.handle) {
                if let Some(model) = models().get(instance.model_handle) {
                    let pose =
                        animation.sample_pose(animation_description.time, &model.nodes, true);

                    debug_assert!(pose.len() == model.nodes.len());

                    fn local_transform(nodes: &[RenderNode], node_index: u32) -> Mat4 {
                        let node = &nodes[node_index as usize];
                        if node.parent_node_index == u32::MAX {
                            Mat4::from_cols_array(&node.transform)
                        } else {
                            local_transform(nodes, node.parent_node_index)
                                * Mat4::from_cols_array(&node.transform)
                        }
                    }

                    let temp_nodes: Vec<RenderNode> = pose
                        .iter()
                        .enumerate()
                        .map(|(node_index, sample)| RenderNode {
                            transform: sample.to_mat4().to_cols_array(),
                            parent_node_index: model.nodes[node_index].parent,
                            _padding: [0; 3],
                        })
                        .collect();

                    let nodes: Vec<RenderNode> = temp_nodes
                        .iter()
                        .enumerate()
                        .map(|(node_index, node)| RenderNode {
                            transform: local_transform(&temp_nodes, node_index as u32)
                                .to_cols_array(),
                            parent_node_index: node.parent_node_index,
                            _padding: [0; 3],
                        })
                        .collect();

                    self.models
                        .update_animation_nodes(instance.render_model_handle, &nodes);
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
                    .filter(|(_, instance)| instance.render_model_handle == model_handle)
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
