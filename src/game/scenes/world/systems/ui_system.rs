use glam::{Mat4, UVec2};

use crate::{
    engine::renderer::{Frame, Renderer, Surface},
    game::scenes::world::{
        render::{RenderStore, RenderUiRect, RenderWorld},
        sim_world::SimWorld,
    },
    wgsl_shader,
};

pub struct UiSystem {
    rect_render_pipeline: wgpu::RenderPipeline,
}

impl UiSystem {
    pub fn new(renderer: &Renderer, surface: &Surface, render_store: &RenderStore) -> Self {
        let module = renderer
            .device
            .create_shader_module(wgsl_shader!("ui_rect"));

        let layout = renderer
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ui_rect_pipeline_layout"),
                bind_group_layouts: &[&render_store.ui_state_bind_group_layout],
                push_constant_ranges: &[],
            });

        let rect_render_pipeline =
            renderer
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("ui_rect_render_pipeline"),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: None,
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<RenderUiRect>()
                                as wgpu::BufferAddress,
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
                            format: surface.format(),
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

    pub fn extract(
        &self,
        sim_world: &mut SimWorld,
        _render_store: &mut RenderStore,
        render_world: &mut RenderWorld,
        viewport_size: UVec2,
    ) {
        let proj = Mat4::orthographic_rh(
            0.0,
            viewport_size.x as f32,
            viewport_size.y as f32,
            0.0,
            -1.0,
            1.0,
        );

        render_world.ui_state.view_proj = proj.to_cols_array_2d();

        // Copy the requested [UiRect]s to the temp buffer. Trying to avoid allocations.
        render_world.ui_rects.clear();
        render_world
            .ui_rects
            .extend(sim_world.ui.ui_rects.iter().map(|rect| {
                let min = rect.pos.as_vec2();
                let max = min + rect.size.as_vec2();
                RenderUiRect {
                    min: [min.x.min(max.x), min.y.min(max.y)],
                    max: [min.x.max(max.x), min.y.max(max.y)],
                    color: rect.color.to_array(),
                }
            }));
    }

    pub fn prepare(&mut self, render_world: &mut RenderWorld, renderer: &Renderer) {
        renderer.queue.write_buffer(
            &render_world.ui_state_buffer,
            0,
            bytemuck::bytes_of(&render_world.ui_state),
        );

        render_world
            .ui_rects_buffer
            .write(renderer, &render_world.ui_rects);
    }

    pub fn queue(&mut self, render_world: &RenderWorld, frame: &mut Frame) {
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

        render_pass.draw(0..4, 0..(render_world.ui_rects.len() as u32));
    }
}
