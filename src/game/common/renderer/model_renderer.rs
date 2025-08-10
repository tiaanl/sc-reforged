use ahash::HashMap;
use wgpu::util::DeviceExt;

use crate::{
    engine::{
        prelude::*,
        storage::{Handle, Storage},
    },
    game::{
        animations::Animation,
        geometry_buffers::GeometryBuffers,
        image::BlendMode,
        model,
        models::models,
        renderer::{
            instance::{RenderInstance, RenderInstanceAnimation},
            render_animations::{RenderAnimation, RenderAnimations},
            render_models::{RenderModel, RenderModels, RenderVertex},
            render_textures,
        },
    },
};

#[derive(Clone, Copy, Debug, bytemuck::NoUninit)]
#[repr(C)]
struct GpuInstance {
    model_matrix: Mat4,
    id: u32,
    animation_time: f32,
    _padding: [u32; 2],
}

#[derive(Default)]
pub struct InstanceUpdater {
    transform: Option<Mat4>,
    animation: Option<Handle<RenderAnimation>>,
    animation_time: Option<f32>,
    /// If `true`, the even if an animation was specified, the animation handle will be cleared.
    clear_animation: bool,
}

impl InstanceUpdater {
    pub fn set_transform(&mut self, transform: Mat4) {
        self.transform = Some(transform);
    }

    pub fn set_animation(&mut self, animation: Handle<RenderAnimation>) {
        self.animation = Some(animation);
    }

    pub fn set_animation_time(&mut self, time: f32) {
        self.animation_time = Some(time);
    }

    pub fn clear_animation(&mut self) {
        self.clear_animation = true;
    }
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct InstanceKey {
    pub render_model: Handle<RenderModel>,
    pub render_animation: Option<Handle<RenderAnimation>>,
}

impl InstanceKey {
    pub fn new(
        render_model: Handle<RenderModel>,
        render_animation: Option<Handle<RenderAnimation>>,
    ) -> Self {
        Self {
            render_model,
            render_animation,
        }
    }
}

pub struct ModelRenderer {
    textures: render_textures::RenderTextures,
    models: RenderModels,
    animations: RenderAnimations,

    /// Keep a list of each model we have to render.
    render_instances: Storage<RenderInstance>,

    /// The pipeline to render all opaque models.
    opaque_pipeline: wgpu::RenderPipeline,
    /// The pipeline to render all models with an alpha channel.
    alpha_pipeline: wgpu::RenderPipeline,
}

impl ModelRenderer {
    pub fn new(
        shaders: &mut Shaders,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        environment_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let textures = render_textures::RenderTextures::new();
        let models = RenderModels::default();
        let animations = RenderAnimations::default();

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
                    animations.bind_group_layout(),
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
                array_stride: std::mem::size_of::<GpuInstance>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![
                    4 => Float32x4,
                    5 => Float32x4,
                    6 => Float32x4,
                    7 => Float32x4,
                    8 => Uint32,
                    9 => Float32,
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
            animations,

            render_instances: Storage::default(),

            opaque_pipeline,
            alpha_pipeline,
        }
    }

    pub fn add_render_instance(
        &mut self,
        model_handle: Handle<model::Model>,
        transform: Mat4,
        entity_id: u32,
    ) -> Result<Handle<RenderInstance>, AssetError> {
        let instance_key =
            self.models
                .add_model(&mut self.textures, &mut self.animations, model_handle)?;

        let render_instance = RenderInstance::new(instance_key.render_model, entity_id, transform);
        Ok(self.render_instances.insert(render_instance))
    }

    pub fn add_animation(
        &mut self,
        model_handle: Handle<model::Model>,
        animation_handle: Handle<Animation>,
    ) -> Handle<RenderAnimation> {
        let model = models().get(model_handle).expect("Could not get model");
        let nodes = &model.nodes;
        self.animations.add(animation_handle, nodes)
    }

    pub fn update_instance(
        &mut self,
        render_instance: Handle<RenderInstance>,
        mut update: impl FnMut(&mut InstanceUpdater),
    ) {
        let mut updater = InstanceUpdater::default();

        update(&mut updater);

        let Some(instance) = self.render_instances.get_mut(render_instance) else {
            tracing::warn!("Invalid model instance handle to update.");
            return;
        };

        if let Some(transform) = updater.transform {
            instance.transform = transform;
        }

        if updater.clear_animation {
            instance.animation = None;
        } else {
            if let Some(animation) = updater.animation {
                instance.animation = Some(RenderInstanceAnimation::from_animation(animation));
            }

            if let Some(animation_time) = updater.animation_time {
                if let Some(ref mut animation) = instance.animation {
                    animation.time = animation_time;
                }
            }
        }
    }

    pub fn render(
        &mut self,
        frame: &mut Frame,
        geometry_buffers: &GeometryBuffers,
        camera_bind_group: &wgpu::BindGroup,
        environment_bind_group: &wgpu::BindGroup,
    ) {
        // Build instance maps.
        #[derive(Debug, Eq, Hash, PartialEq)]
        struct Key {
            model: Handle<RenderModel>,
            animation: Handle<RenderAnimation>,
        }

        let mut opaque_instances: HashMap<Key, Vec<GpuInstance>> = HashMap::default();

        for (_, instance) in self.render_instances.iter() {
            // Figure out which animation to render.
            let animation = if let Some(ref animation) = instance.animation {
                *animation
            } else {
                let Some(model) = self.models.get(instance.render_model) else {
                    tracing::warn!("Missing render model.");
                    continue;
                };
                RenderInstanceAnimation::from_animation(model.rest_pose)
            };

            opaque_instances
                .entry(Key {
                    model: instance.render_model,
                    animation: animation.handle,
                })
                .or_default()
                .push(GpuInstance {
                    model_matrix: instance.transform,
                    id: instance.entity_id,
                    animation_time: animation.time,
                    _padding: Default::default(),
                });
        }

        // Opaque
        {
            let mut render_pass = frame
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("model_renderer_render_pass"),
                    color_attachments: &geometry_buffers.color_attachments(),
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &geometry_buffers.depth.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            render_pass.set_pipeline(&self.opaque_pipeline);
            render_pass.set_bind_group(0, camera_bind_group, &[]);
            render_pass.set_bind_group(1, environment_bind_group, &[]);

            for (key, gpu_instances) in opaque_instances.iter() {
                let Some(model) = self.models.get(key.model) else {
                    tracing::warn!("model lookup failure!");
                    continue;
                };

                let Some(animation) = self.animations.get(key.animation) else {
                    tracing::warn!("animation lookup failure!");
                    continue;
                };

                // Create the buffer with instances.
                let instances_buffer =
                    renderer()
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("model_renderer_instances"),
                            contents: bytemuck::cast_slice(gpu_instances),
                            usage: wgpu::BufferUsages::VERTEX,
                        });

                render_pass.set_vertex_buffer(1, instances_buffer.slice(..));

                render_pass.set_bind_group(3, &animation.bind_group, &[]);

                model.render(&mut render_pass, &self.textures, BlendMode::Opaque);

                model.render(&mut render_pass, &self.textures, BlendMode::ColorKeyed);
            }
        }

        // Alpha
        {
            let mut render_pass = frame
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("model_renderer_render_pass"),
                    color_attachments: &geometry_buffers.color_attachments(),
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &geometry_buffers.depth.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Discard,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            render_pass.set_pipeline(&self.alpha_pipeline);
            render_pass.set_bind_group(0, camera_bind_group, &[]);
            render_pass.set_bind_group(1, environment_bind_group, &[]);

            for (key, gpu_instances) in opaque_instances.iter() {
                let Some(model) = self.models.get(key.model) else {
                    tracing::warn!("model lookup failure!");
                    continue;
                };

                let Some(animation) = self.animations.get(key.animation) else {
                    tracing::warn!("animation lookup failure!");
                    continue;
                };

                // Create the buffer with instances.
                let instances_buffer =
                    renderer()
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("model_renderer_instances"),
                            contents: bytemuck::cast_slice(gpu_instances),
                            usage: wgpu::BufferUsages::VERTEX,
                        });

                render_pass.set_vertex_buffer(1, instances_buffer.slice(..));

                render_pass.set_bind_group(3, &animation.bind_group, &[]);

                model.render(&mut render_pass, &self.textures, BlendMode::Alpha);
            }
        }
    }
}
