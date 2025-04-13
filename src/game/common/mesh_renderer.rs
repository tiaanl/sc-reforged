use std::collections::HashMap;

use glam::Mat4;
use wgpu::{ShaderStages, util::DeviceExt, vertex_attr_array};

use crate::engine::prelude::*;

use super::geometry_buffers::GeometryBuffers;

#[derive(Clone, Copy, Debug)]
pub enum BlendMode {
    /// No blending.
    Opaque,
    /// Color keyed (use black as the key).
    ColorKeyed,
    /// Use the alpha channel of the texture.
    Alpha,
    /// Multiply the values from the texture with the background. Mostly used for light effects.
    Multiply,
}

/// A texture bind group with meta data.
#[derive(Debug)]
pub struct Texture {
    pub bind_group: wgpu::BindGroup,
    pub blend_mode: BlendMode,
}

/// A mesh containing gpu resources that we can render.
#[derive(Debug)]
pub struct TexturedMesh {
    pub indexed_mesh: IndexedMesh<Vertex>,
    pub gpu_mesh: GpuIndexedMesh,
    pub texture: Texture,
}

impl Asset for TexturedMesh {}

#[derive(Debug)]
pub struct MeshItem {
    pub transform: Mat4,
    pub mesh: Handle<TexturedMesh>,
    /// Used to sort the objects to render from far to near to handle translucent textures.
    pub distance_from_camera: f32,
}

pub struct MeshRenderer {
    asset_store: AssetStore,
    opaque_render_pipeline: wgpu::RenderPipeline,
    ck_render_pipeline: wgpu::RenderPipeline,
    alpha_render_pipeline: wgpu::RenderPipeline,
    transforms_bind_group_layout: wgpu::BindGroupLayout,
}

impl MeshRenderer {
    pub fn new(
        asset_store: AssetStore,
        renderer: &Renderer,
        shaders: &mut Shaders,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let module = shaders.create_shader(
            renderer,
            "model_renderer",
            include_str!("mesh.wgsl"),
            "model.wgsl",
            HashMap::default(),
        );

        let transforms_bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("model_transforms_bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let pipeline_layout =
            renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("mesh_renderer_pipeline_layout"),
                    bind_group_layouts: &[
                        // u_texture
                        renderer.texture_bind_group_layout(),
                        // u_camera
                        camera_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });

        let opaque_render_pipeline =
            Self::create_pipeline(renderer, &pipeline_layout, &module, BlendMode::Opaque);
        let ck_render_pipeline =
            Self::create_pipeline(renderer, &pipeline_layout, &module, BlendMode::ColorKeyed);
        let alpha_render_pipeline =
            Self::create_pipeline(renderer, &pipeline_layout, &module, BlendMode::Alpha);

        Self {
            asset_store,
            opaque_render_pipeline,
            ck_render_pipeline,
            alpha_render_pipeline,
            transforms_bind_group_layout,
        }
    }

    fn create_pipeline(
        renderer: &Renderer,
        layout: &wgpu::PipelineLayout,
        module: &wgpu::ShaderModule,
        blend_mode: BlendMode,
    ) -> wgpu::RenderPipeline {
        renderer
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("mesh_renderer_render_pipeline"),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module,
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[
                        Vertex::layout(),
                        wgpu::VertexBufferLayout {
                            array_stride: (std::mem::size_of::<glam::Mat4>())
                                as wgpu::BufferAddress,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &vertex_attr_array![
                                3 => Float32x4,
                                4 => Float32x4,
                                5 => Float32x4,
                                6 => Float32x4,
                            ],
                        },
                    ],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    ..Default::default()
                },
                depth_stencil: Some(renderer.depth_buffer.depth_stencil_state(
                    wgpu::CompareFunction::Less,
                    match blend_mode {
                        BlendMode::Opaque | BlendMode::ColorKeyed | BlendMode::Multiply => true,
                        // Don't write to the depth buffer if we use alpha blending.
                        BlendMode::Alpha => false,
                    },
                )),
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module,
                    entry_point: match blend_mode {
                        BlendMode::ColorKeyed => Some("ck_fragment_main"),
                        BlendMode::Opaque | BlendMode::Alpha | BlendMode::Multiply => {
                            Some("fragment_main")
                        }
                    },
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    // targets: &[Some(wgpu::ColorTargetState {
                    //     format: renderer.surface_config.format,
                    //     blend: match blend_mode {
                    //         BlendMode::Opaque => None,
                    //         BlendMode::ColorKeyed => None,
                    //         BlendMode::Alpha => Some(wgpu::BlendState::ALPHA_BLENDING),
                    //         BlendMode::Multiply => None,
                    //     },
                    //     write_mask: wgpu::ColorWrites::ALL,
                    // })],
                    targets: GeometryBuffers::targets(),
                }),
                multiview: None,
                cache: None,
            })
    }

    pub fn render_multiple(
        &self,
        frame: &mut Frame,
        geometry_buffers: &GeometryBuffers,
        camera_bind_group: &wgpu::BindGroup,
        blend_mode: BlendMode,
        meshes: &[MeshItem],
    ) {
        if meshes.is_empty() {
            return;
        }

        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("opaque_meshes"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &geometry_buffers.colors.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &geometry_buffers.positions.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &geometry_buffers.normals.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &geometry_buffers.ids.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &frame.depth_buffer.texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        let render_pipeline = match blend_mode {
            BlendMode::Opaque => &self.opaque_render_pipeline,
            BlendMode::ColorKeyed => &self.ck_render_pipeline,
            BlendMode::Alpha => &self.alpha_render_pipeline,
            BlendMode::Multiply => todo!(),
        };

        render_pass.set_pipeline(render_pipeline);
        render_pass.set_bind_group(1, camera_bind_group, &[]);

        for mesh_item in meshes.iter() {
            let Some(mesh) = self.asset_store.get(mesh_item.mesh) else {
                // Ignore meshes we can't find.
                tracing::warn!("Mesh not found: {:?}", mesh_item.mesh);
                continue;
            };

            // Create a buffer to render to.
            let buffer = frame
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("mesh_indices_buffer"),
                    contents: bytemuck::cast_slice(&[mesh_item.transform]),
                    usage: wgpu::BufferUsages::VERTEX,
                });

            render_pass.set_vertex_buffer(0, mesh.gpu_mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, buffer.slice(..));
            render_pass.set_index_buffer(
                mesh.gpu_mesh.index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.set_bind_group(0, &mesh.texture.bind_group, &[]);
            render_pass.draw_indexed(0..mesh.gpu_mesh.index_count, 0, 0..1);
        }
    }
}
