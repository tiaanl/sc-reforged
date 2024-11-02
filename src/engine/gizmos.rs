use wgpu::{util::DeviceExt, vertex_attr_array};

use super::renderer::{BufferLayout, RenderPipelineConfig, Renderer};

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
pub struct GizmoVertex {
    pub position: [f32; 4],
    pub color: [f32; 4],
}

impl BufferLayout for GizmoVertex {
    fn vertex_buffers() -> &'static [wgpu::VertexBufferLayout<'static>] {
        const VERTEX_ATTR_ARRAY: &[wgpu::VertexAttribute] = &vertex_attr_array![
            0 => Float32x4, // position
            1 => Float32x4, // color
        ];

        &[wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: VERTEX_ATTR_ARRAY,
        }]
    }
}

pub struct GizmosRenderer {
    pipeline: wgpu::RenderPipeline,
}

impl GizmosRenderer {
    pub fn new(renderer: &Renderer, camera_bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        let shader = renderer.create_shader_module("gizmos", include_str!("gizmos.wgsl"));

        let pipeline = renderer.create_render_pipeline(
            RenderPipelineConfig::<GizmoVertex>::new("gizmos", &shader)
                .bind_group_layout(&camera_bind_group_layout)
                .primitive(wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
                    ..Default::default()
                })
                .disable_depth_buffer(),
        );

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
