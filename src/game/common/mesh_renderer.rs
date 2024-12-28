use glam::Mat4;
use wgpu::{util::DeviceExt, vertex_attr_array, ShaderStages};

use crate::engine::prelude::*;

use super::model::Model;

/// A texture bind group with meta data.
#[derive(Debug)]
pub struct Texture {
    pub bind_group: wgpu::BindGroup,
    pub translucent: bool,
}

/// A mesh containing gpu resources that we can render.
#[derive(Debug)]
pub struct TexturedMesh {
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

#[derive(Default)]
pub struct MeshList {
    pub meshes: Vec<MeshItem>,
}

pub struct MeshRenderer {
    asset_store: AssetStore,
    render_pipeline: wgpu::RenderPipeline,
    transforms_bind_group_layout: wgpu::BindGroupLayout,
}

impl MeshRenderer {
    pub fn new(
        asset_store: AssetStore,
        renderer: &Renderer,
        shaders: &mut Shaders,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        fog_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader_module = shaders.create_shader(
            renderer,
            "model_renderer",
            include_str!("mesh.wgsl"),
            "model.wgsl",
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
                        // u_fog
                        fog_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });

        let render_pipeline =
            renderer
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("mesh_renderer_render_pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader_module,
                        entry_point: "vertex_main",
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[
                            Vertex::vertex_buffer_layout(),
                            wgpu::VertexBufferLayout {
                                array_stride: std::mem::size_of::<glam::Mat4>()
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
                    depth_stencil: renderer.depth_stencil_state(wgpu::CompareFunction::Less),
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &shader_module,
                        entry_point: "fragment_main",
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: renderer.surface_config.format,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview: None,
                    cache: None,
                });

        Self {
            asset_store,
            render_pipeline,
            transforms_bind_group_layout,
        }
    }

    pub fn mesh_list_from_model(model: &Model, transform: Transform) -> MeshList {
        let mut list = MeshList::default();

        for mesh in model.meshes.iter() {
            list.meshes.push(MeshItem {
                transform: transform.to_mat4() * mesh.model_transform,
                mesh: mesh.mesh,
                distance_from_camera: f32::MAX,
            });
        }

        list
    }

    pub fn render_multiple(
        &self,
        frame: &mut Frame,
        camera_bind_group: &wgpu::BindGroup,
        fog_bind_group: &wgpu::BindGroup,
        meshes: &MeshList,
    ) {
        if meshes.meshes.is_empty() {
            return;
        }

        let device = frame.device.clone();
        let mut render_pass = frame.begin_basic_render_pass("mesh_renderer_render_pass", true);

        for mesh_item in meshes.meshes.iter() {
            let Some(mesh) = self.asset_store.get(mesh_item.mesh) else {
                // Ignore meshes we can't find.
                tracing::warn!("Mesh not found: {:?}", mesh_item.mesh);
                continue;
            };

            // Create a buffer to render to.
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("mesh_indices_buffer"),
                contents: bytemuck::cast_slice(&[mesh_item.transform]),
                usage: wgpu::BufferUsages::VERTEX,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, mesh.gpu_mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, buffer.slice(..));
            render_pass.set_index_buffer(
                mesh.gpu_mesh.index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.set_bind_group(0, &mesh.texture.bind_group, &[]);
            render_pass.set_bind_group(1, camera_bind_group, &[]);
            render_pass.set_bind_group(2, fog_bind_group, &[]);
            render_pass.draw_indexed(0..mesh.gpu_mesh.index_count, 0, 0..1);
        }
    }
}
