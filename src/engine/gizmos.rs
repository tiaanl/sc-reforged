use wgpu::{util::DeviceExt, vertex_attr_array};

use super::renderer::Renderer;

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
pub struct GizmoVertex {
    pub position: [f32; 4],
    pub color: [f32; 4],
}

pub struct GizmosRenderer {
    pipeline: wgpu::RenderPipeline,
}

impl GizmosRenderer {
    pub fn new(renderer: &Renderer, camera_bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        let shader = renderer
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("gizmos_shader_module"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                    "gizmos.wgsl"
                ))),
            });

        let pipeline_layout =
            renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("gizmos_pipeline_layout"),
                    bind_group_layouts: &[&camera_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = renderer
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("gizmos_render_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vertex_main",
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<GizmoVertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &vertex_attr_array![
                            0 => Float32x4,
                            1 => Float32x4,
                        ],
                    }],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fragment_main",
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(renderer.surface_config.format.into())],
                }),
                multiview: None,
                cache: None,
            });

        Self { pipeline }
    }

    pub fn render(
        &self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        camera_bind_group: &wgpu::BindGroup,
        vertices: &Vec<GizmoVertex>,
    ) {
        let buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("gizmos_vertex_buffer"),
                contents: bytemuck::cast_slice(vertices.as_ref()),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("gizmos_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, buffer.slice(..));
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.draw(0..(vertices.len() as u32), 0..1);
    }
}
