use ahash::HashMap;
use glam::Mat4;
use wgpu::{util::DeviceExt, vertex_attr_array, ShaderStages};

use crate::engine::{
    assets::{Asset, AssetManager, Handle},
    mesh::{GpuIndexedMesh, Vertex},
    renderer::Renderer,
    shaders::Shaders,
};

use super::asset_loader::AssetLoader;

/// A mesh containing gpu resources that we can render.
#[derive(Debug)]
pub struct TexturedMesh {
    pub gpu_mesh: GpuIndexedMesh,
    pub texture: wgpu::BindGroup,
}

impl Asset for TexturedMesh {}

pub struct MeshItem {
    pub transform: Mat4,
    pub mesh: Handle<TexturedMesh>,
}

#[derive(Default)]
pub struct MeshList {
    pub meshes: Vec<MeshItem>,
}

pub struct MeshRenderer {
    asset_manager: AssetManager,
    render_pipeline: wgpu::RenderPipeline,
    transforms_bind_group_layout: wgpu::BindGroupLayout,
}

impl MeshRenderer {
    pub fn new(
        asset_manager: AssetManager,
        renderer: &Renderer,
        shaders: &mut Shaders,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
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
                        // u_camera
                        &camera_bind_group_layout,
                        // u_texture
                        renderer.texture_bind_group_layout(),
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
                            wgpu::VertexBufferLayout {
                                array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                                step_mode: wgpu::VertexStepMode::Vertex,
                                attributes: &vertex_attr_array![
                                    0 => Float32x3,
                                    1 => Float32x3,
                                    2 => Float32x2,
                                ],
                            },
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
            asset_manager,
            render_pipeline,
            transforms_bind_group_layout,
        }
    }

    pub fn add(&mut self, _renderer: &Renderer, _assets: &AssetLoader) -> Handle<TexturedMesh> {
        // let model = self
        //     .smf_to_model(renderer, assets, smf)
        //     .expect("Could not load model! FIX THIS");

        // self.models.add(model)
        todo!()
    }

    pub fn render_multiple(
        &self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        camera_bind_group: &wgpu::BindGroup,
        meshes: MeshList,
    ) {
        if meshes.meshes.is_empty() {
            return;
        }

        // Create a list of matrices for each textures mesh.
        let mut instances = HashMap::default();

        for mesh in meshes.meshes.iter() {
            let matrices = instances.entry(mesh.mesh).or_insert(Vec::default());
            matrices.push(mesh.transform);
        }

        let mut render_pass = Self::create_render_pass(renderer, encoder, output);

        for (mesh, matrices) in instances.into_iter() {
            let mesh = self.asset_manager.get(mesh).unwrap();

            // Create a buffer to render to.
            let buffer = renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("mesh_indices_buffer"),
                    contents: bytemuck::cast_slice(&matrices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, mesh.gpu_mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, buffer.slice(..));
            render_pass.set_index_buffer(
                mesh.gpu_mesh.index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.set_bind_group(0, camera_bind_group, &[]);
            render_pass.set_bind_group(1, &mesh.texture, &[]);
            render_pass.draw_indexed(0..mesh.gpu_mesh.index_count, 0, 0..matrices.len() as u32);
        }

        // render_pass.set_pipeline(&self.render_pipeline);
        // render_pass.set_bind_group(0, camera_bind_group, &[]);
        // batch.iter().for_each(|job| {
        //     let Some(model) = self.models.get(&job.handle) else {
        //         return;
        //     };

        //     for mesh in model.meshes.iter() {
        //         let node_index = mesh.node_index as u32;
        //         render_pass.set_bind_group(1, &model.transforms_bind_group, &[]);
        //         render_pass.set_bind_group(2, &mesh.texture, &[]);
        //         render_pass.draw_mesh(&mesh.gpu_mesh, node_index..node_index + 1);
        //     }
        // });
    }

    fn create_render_pass<'encoder>(
        renderer: &Renderer,
        encoder: &'encoder mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) -> wgpu::RenderPass<'encoder> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("model_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(
                renderer.render_pass_depth_stencil_attachment(wgpu::LoadOp::Clear(1.0)),
            ),
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }
}
