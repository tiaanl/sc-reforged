use glam::Mat4;

use crate::{
    engine::renderer::{Frame, Renderer},
    game::{
        assets::Assets,
        scenes::world::render::{RenderStore, RenderUiRect, RenderWorld, render_pass::RenderPass},
    },
    wgsl_shader,
};

#[derive(Default)]
pub struct UiRenderSnapshot {
    pub view_proj: Mat4,
    pub ui_rects: Vec<RenderUiRect>,
}

pub struct UiRenderPass {
    rect_render_pipeline: wgpu::RenderPipeline,
}

impl RenderPass for UiRenderPass {
    type Snapshot = UiRenderSnapshot;

    fn prepare(
        &mut self,
        _assets: &Assets,
        renderer: &Renderer,
        _render_store: &mut RenderStore,
        render_world: &mut RenderWorld,
        snapshot: &Self::Snapshot,
    ) {
        renderer.queue.write_buffer(
            &render_world.ui_state_buffer,
            0,
            bytemuck::bytes_of(&render_world.ui_state),
        );

        render_world
            .ui_rects_buffer
            .write(renderer, &snapshot.ui_rects);
    }

    fn queue(
        &self,
        _render_store: &RenderStore,
        render_world: &RenderWorld,
        frame: &mut Frame,
        _geometry_buffer: &super::GeometryBuffer,
        snapshot: &Self::Snapshot,
    ) {
        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.surface,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        render_pass.set_pipeline(&self.rect_render_pipeline);

        render_pass.set_vertex_buffer(0, render_world.ui_rects_buffer.slice(..));
        render_pass.set_bind_group(0, &render_world.ui_state_bind_group, &[]);

        render_pass.draw(0..4, 0..(snapshot.ui_rects.len() as u32));
    }
}

impl UiRenderPass {
    pub fn new(
        renderer: &Renderer,
        surface_format: wgpu::TextureFormat,
        render_store: &RenderStore,
    ) -> Self {
        let device = &renderer.device;

        let module = device.create_shader_module(wgsl_shader!("ui"));

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ui_rect_pipeline_layout"),
            bind_group_layouts: &[&render_store.ui_state_bind_group_layout],
            push_constant_ranges: &[],
        });

        let rect_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui_rect_render_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: None,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<RenderUiRect>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2,
                        1 => Float32x2,
                        2 => Float32x4,
                    ],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: None,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        Self {
            rect_render_pipeline,
        }
    }
}
