use ahash::HashMap;
use wgpu::util::DeviceExt;

use crate::{
    engine::{
        prelude::*,
        storage::{Handle, Storage},
    },
    game::{
        animations::Animation,
        geometry_buffers::{GeometryBuffers, RenderTarget},
        math::{BoundingSphere, Frustum},
        model,
        renderer::{
            render_animations::{RenderAnimation, RenderAnimations},
            render_instance::{RenderInstance, RenderInstanceAnimation},
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

struct InstancesBuffer {
    buffer: wgpu::Buffer,
    cursor: u64,
    capacity: u64,
}

impl InstancesBuffer {
    const STRIDE: usize = std::mem::size_of::<GpuInstance>();

    fn new(capacity: u64) -> Self {
        let buffer_size_in_bytes = capacity as usize * Self::STRIDE;

        let buffer = renderer().device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("model_renderer_instances_buffer"),
            size: buffer_size_in_bytes as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            buffer,
            cursor: 0,
            capacity,
        }
    }

    fn write(&mut self, instances: &[GpuInstance]) -> std::ops::Range<u64> {
        renderer().queue.write_buffer(
            &self.buffer,
            self.cursor * Self::STRIDE as u64,
            bytemuck::cast_slice(instances),
        );

        let start = self.cursor;
        self.cursor += instances.len() as u64;
        (start * Self::STRIDE as u64)..(self.cursor * Self::STRIDE as u64)
    }

    fn reset(&mut self) {
        self.cursor = 0;
    }
}

#[derive(Default)]
pub struct InstanceUpdater {
    transform: Option<Mat4>,
    animation: Option<RenderInstanceAnimation>,
    /// If `true`, the even if an animation was specified, the animation handle will be cleared.
    clear_animation: bool,
}

impl InstanceUpdater {
    pub fn set_transform(&mut self, transform: Mat4) {
        self.transform = Some(transform);
    }

    pub fn set_animation(&mut self, animation: Handle<RenderAnimation>, time: f32) {
        self.animation = Some(RenderInstanceAnimation { animation, time });
    }

    pub fn _clear_animation(&mut self) {
        self.clear_animation = true;
    }
}

pub struct ModelRenderer {
    textures: render_textures::RenderTextures,
    models: RenderModels,
    animations: RenderAnimations,

    /// Keep a list of each model we have to render.
    render_instances: Storage<RenderInstance>,

    /// A single buffer to hold instances for a render pass.
    instances_buffer: InstancesBuffer,

    /// The pipeline to render all opaque models.
    opaque_pipeline: wgpu::RenderPipeline,
    /// The pipeline to render all models with an alpha channel.
    alpha_pipeline: wgpu::RenderPipeline,
    /// The pipeline to render all shadow casting models to the shadow buffer.
    shadow_pipeline: wgpu::RenderPipeline,
}

impl ModelRenderer {
    pub fn new(
        shaders: &mut Shaders,
        shadow_render_target: &RenderTarget,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        environment_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let textures = render_textures::RenderTextures::new();
        let models = RenderModels::default();
        let animations = RenderAnimations::default();

        // Default for 1024 instances for now.
        let instances_buffer = InstancesBuffer::new(2);

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
                    &textures.texture_set_bind_group_layout,
                    animations.bind_group_layout(),
                ],
                push_constant_ranges: &[],
            });

        let buffers = &[
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<RenderVertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![
                    0 => Float32x3, // position
                    1 => Float32x3, // normal
                    2 => Float32x2, // tex_coord
                    3 => Uint32, // node_index
                    4 => Uint32, // texture_index
                ],
            },
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<GpuInstance>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![
                    5 => Float32x4, // model_mat_col_0
                    6 => Float32x4, // model_mat_col_1
                    7 => Float32x4, // model_mat_col_2
                    8 => Float32x4, // model_mat_col_3
                    9 => Uint32, // entity_id
                    10 => Float32, // time
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
                        entry_point: Some("vertex_main"),
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
                        entry_point: Some("vertex_main"),
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
                        false, // No depth writes for alpha pass.
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

        let shadow_pipeline =
            renderer()
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("model_renderer_shadow_render_pipeline"),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some("shadow_vertex"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers,
                    },
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: shadow_render_target.format,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::LessEqual,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState {
                            constant: 2,
                            slope_scale: 2.0,
                            clamp: 0.0,
                        },
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    fragment: None,
                    multiview: None,
                    cache: None,
                });

        Self {
            textures,
            models,
            animations,

            instances_buffer,

            render_instances: Storage::default(),

            opaque_pipeline,
            alpha_pipeline,
            shadow_pipeline,
        }
    }

    pub fn add_render_instance(
        &mut self,
        model_handle: Handle<model::Model>,
        transform: Mat4,
        entity_id: u32,
    ) -> Result<Handle<RenderInstance>, AssetError> {
        let render_model =
            self.models
                .add_model(&mut self.textures, &mut self.animations, model_handle)?;

        let render_instance = RenderInstance::new(render_model, entity_id, transform);
        Ok(self.render_instances.insert(render_instance))
    }

    pub fn add_animation(
        &mut self,
        model_handle: Handle<model::Model>,
        animation_handle: Handle<Animation>,
    ) -> Handle<RenderAnimation> {
        self.animations.add(model_handle, animation_handle)
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
        } else if let Some(animation) = updater.animation {
            instance.animation = Some(animation);
        }
    }

    fn build_render_set(&self, frustum: &Frustum) -> RenderSet {
        let mut result = RenderSet::default();

        let mut total_instances = 0;

        for (_, instance) in self.render_instances.iter() {
            let Some(model) = self.models.get(instance.render_model) else {
                tracing::warn!("Missing render model.");
                continue;
            };

            let center = instance
                .transform
                .transform_point3(model.bounding_sphere.center);

            // Move the bounding sphere to the location of the model.
            let bounding_sphere = BoundingSphere {
                center,
                radius: model.bounding_sphere.radius,
            };

            if !frustum.intersects_bounding_sphere(&bounding_sphere) {
                continue;
            }

            // Figure out which animation to render.
            let animation = if let Some(ref animation) = instance.animation {
                *animation
            } else {
                RenderInstanceAnimation::from_animation(model.rest_pose)
            };

            let key = RenderSetKey {
                model: instance.render_model,
                animation: animation.animation,
            };

            let gpu_instance = GpuInstance {
                model_matrix: instance.transform,
                id: instance.entity_id,
                animation_time: animation.time,
                _padding: Default::default(),
            };
            result
                .opaque_instances
                .entry(key)
                .or_default()
                .push(gpu_instance);
            total_instances += 1;

            if let Some(ref mesh) = model.alpha_mesh {
                if mesh.index_count != 0 {
                    result
                        .alpha_instances
                        .entry(key)
                        .or_default()
                        .push(gpu_instance);
                    total_instances += 1;
                }
            }
        }

        result.total_instances = total_instances;

        result
    }

    pub fn render_shadow_casters(
        &mut self,
        frame: &mut Frame,
        shadow_render_target: &RenderTarget,
        frustum: &Frustum,
        environment_bind_group: &wgpu::BindGroup,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        let render_set = self.build_render_set(frustum);

        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("model_renderer_shadow_pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &shadow_render_target.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        render_pass.set_pipeline(&self.shadow_pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, environment_bind_group, &[]);

        for (key, gpu_instances) in render_set.opaque_instances.iter() {
            let Some(model) = self.models.get(key.model) else {
                tracing::warn!("model lookup failure!");
                continue;
            };

            let Some(animation) = self.animations.get(key.animation) else {
                tracing::warn!("animation lookup failure!");
                continue;
            };

            let Some(ref mesh) = model.opaque_mesh else {
                tracing::warn!("No opaque mesh, but in opaque instances!");
                continue;
            };

            // TODO: Don't put empty meshes into the instances buffer.
            if mesh.index_count == 0 {
                continue;
            }

            // Create the buffer with instances.
            let instances_buffer =
                renderer()
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("model_renderer_instances"),
                        contents: bytemuck::cast_slice(gpu_instances),
                        usage: wgpu::BufferUsages::VERTEX,
                    });

            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, instances_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

            render_pass.set_bind_group(2, &model.texture_set.bind_group, &[]);
            render_pass.set_bind_group(3, &animation.bind_group, &[]);

            render_pass.draw_indexed(0..mesh.index_count, 0, 0..gpu_instances.len() as u32);
        }
    }

    pub fn render(
        &mut self,
        frame: &mut Frame,
        frustum: &Frustum,
        geometry_buffers: &GeometryBuffers,
        camera_bind_group: &wgpu::BindGroup,
        environment_bind_group: &wgpu::BindGroup,
    ) {
        let render_set = self.build_render_set(frustum);

        {
            // Make sure we can fit all the instances.
            if render_set.total_instances > self.instances_buffer.capacity {
                let new_size = render_set.total_instances.max(1).next_power_of_two();
                tracing::info!("Resizing instances buffer to {}", new_size);
                self.instances_buffer = InstancesBuffer::new(new_size);
            }
        }
        self.instances_buffer.reset();

        // Opaque
        {
            let mut render_pass = frame
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("model_renderer_render_pass"),
                    color_attachments: &geometry_buffers.opaque_attachments(),
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

            for (key, gpu_instances) in render_set.opaque_instances.iter() {
                let Some(model) = self.models.get(key.model) else {
                    tracing::warn!("model lookup failure!");
                    continue;
                };

                let Some(animation) = self.animations.get(key.animation) else {
                    tracing::warn!("animation lookup failure!");
                    continue;
                };

                let Some(ref mesh) = model.opaque_mesh else {
                    tracing::warn!("No opaque mesh, but in opaque instances!");
                    continue;
                };

                // TODO: Don't put empty meshes into the instances buffer.
                if mesh.index_count == 0 {
                    continue;
                }

                let instance_range = self.instances_buffer.write(gpu_instances);

                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass
                    .set_vertex_buffer(1, self.instances_buffer.buffer.slice(instance_range));
                render_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

                render_pass.set_bind_group(2, &model.texture_set.bind_group, &[]);
                render_pass.set_bind_group(3, &animation.bind_group, &[]);

                render_pass.draw_indexed(0..mesh.index_count, 0, 0..gpu_instances.len() as u32);
            }
        }

        // Alpha
        {
            let mut render_pass = frame
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("model_renderer_render_pass"),
                    color_attachments: &geometry_buffers.alpha_attachments(),
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

            render_pass.set_pipeline(&self.alpha_pipeline);
            render_pass.set_bind_group(0, camera_bind_group, &[]);
            render_pass.set_bind_group(1, environment_bind_group, &[]);

            for (key, gpu_instances) in render_set.alpha_instances.iter() {
                let Some(model) = self.models.get(key.model) else {
                    tracing::warn!("model lookup failure!");
                    continue;
                };

                let Some(animation) = self.animations.get(key.animation) else {
                    tracing::warn!("animation lookup failure!");
                    continue;
                };

                let Some(ref mesh) = model.alpha_mesh else {
                    tracing::warn!("No alpha mesh, but in alpha instances!");
                    continue;
                };

                // TODO: Don't put empty meshes into the instances buffer.
                if mesh.index_count == 0 {
                    continue;
                }

                let instances_range = self.instances_buffer.write(gpu_instances);

                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass
                    .set_vertex_buffer(1, self.instances_buffer.buffer.slice(instances_range));
                render_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

                render_pass.set_bind_group(2, &model.texture_set.bind_group, &[]);
                render_pass.set_bind_group(3, &animation.bind_group, &[]);

                render_pass.draw_indexed(0..mesh.index_count, 0, 0..gpu_instances.len() as u32);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct RenderSetKey {
    model: Handle<RenderModel>,
    animation: Handle<RenderAnimation>,
}

#[derive(Default)]
struct RenderSet {
    total_instances: u64,
    opaque_instances: HashMap<RenderSetKey, Vec<GpuInstance>>,
    alpha_instances: HashMap<RenderSetKey, Vec<GpuInstance>>,
}
