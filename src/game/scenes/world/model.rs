use glam::Vec3;
use wgpu::vertex_attr_array;

use crate::engine::{
    mesh::{GpuMesh, RenderPassMeshExt},
    renderer::{GpuTexture, Renderer},
};

pub struct ModelRenderer {
    texture_bind_group_layout: wgpu::BindGroupLayout,
    render_pipeline: wgpu::RenderPipeline,
}

impl ModelRenderer {
    pub fn new(renderer: &Renderer, camera_bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        let texture_bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let module = renderer
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("model_shader_module"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                    "model.wgsl"
                ))),
            });

        let render_pipeline_layout =
            renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("model_render_pipeline_layout"),
                    bind_group_layouts: &[camera_bind_group_layout, &texture_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let render_pipeline =
            renderer
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("model_render_pipeline"),
                    layout: Some(&render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: "vertex_main",
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<crate::engine::mesh::Vertex>()
                                as wgpu::BufferAddress,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &vertex_attr_array![
                                0 => Float32x3,
                                1 => Float32x3,
                                2 => Float32x2,
                            ],
                        }],
                    },
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: renderer.depth_stencil_state(wgpu::CompareFunction::LessEqual),
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: "fragment_main",
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: renderer.surface_config.format,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview: None,
                    cache: None,
                });

        Self {
            texture_bind_group_layout,
            render_pipeline,
        }
    }

    pub fn render(
        &self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        camera_bind_group: &wgpu::BindGroup,
        mesh: &GpuMesh,
        texture: &GpuTexture,
        _position: Vec3,
    ) {
        let texture_bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("texture_bind_group"),
                layout: &self.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture.sampler),
                    },
                ],
            });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("model_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: renderer
                .render_pass_depth_stencil_attachment(wgpu::LoadOp::Load),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, &texture_bind_group, &[]);
        render_pass.draw_mesh(mesh);
    }
}
