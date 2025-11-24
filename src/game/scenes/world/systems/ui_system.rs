use glam::{IVec2, Mat4, UVec2, Vec2, Vec4};
use winit::event::MouseButton;

use crate::{
    engine::{
        input::InputState,
        prelude::{Frame, Renderer},
    },
    game::scenes::world::{
        render::{RenderStore, RenderWorld, UiRect},
        sim_world::{SelectionRect, SimWorld},
    },
    wgsl_shader,
};

pub struct UiSystem {
    rect_render_pipeline: wgpu::RenderPipeline,
}

impl UiSystem {
    pub fn new(renderer: &Renderer, render_store: &RenderStore) -> Self {
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
                            array_stride: std::mem::size_of::<UiRect>() as wgpu::BufferAddress,
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
                            format: renderer.surface.format(),
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

    pub fn input(&self, sim_world: &mut SimWorld, input_state: &InputState) {
        if input_state.mouse_just_pressed(MouseButton::Left) {
            sim_world.ui.selection_rect = input_state.mouse_position().map(|pos| SelectionRect {
                pos,
                size: IVec2::ZERO,
            });
            return;
        }

        if input_state.mouse_just_released(MouseButton::Left) {
            sim_world.ui.selection_rect = None;
            return;
        }

        let Some(rect) = sim_world.ui.selection_rect.as_mut() else {
            return;
        };

        if let Some(mouse_position) = input_state.mouse_position() {
            rect.size = mouse_position - rect.pos;
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

        render_world.ui_rects.clear();

        if let Some(rect) = &sim_world.ui.selection_rect {
            let pos = Vec2::new(rect.pos.x as f32, rect.pos.y as f32);
            let size = Vec2::new(rect.size.x as f32, rect.size.y as f32);

            let min = Vec2::new(pos.x.min(pos.x + size.x), pos.y.min(pos.y + size.y));
            let max = Vec2::new(pos.x.max(pos.x + size.x), pos.y.max(pos.y + size.y));

            const THICKNESS: f32 = 1.0;

            render_world.ui_rects.push(UiRect {
                min: min.to_array(),
                max: max.to_array(),
                color: Vec4::new(0.0, 0.0, 0.0, 0.5).to_array(),
            });

            // Left
            render_world.ui_rects.push(UiRect {
                min: min.to_array(),
                max: Vec2::new(min.x + THICKNESS, max.y).to_array(),
                color: Vec4::new(1.0, 1.0, 1.0, 0.5).to_array(),
            });

            // Right
            render_world.ui_rects.push(UiRect {
                min: Vec2::new(max.x - THICKNESS, min.y).to_array(),
                max: Vec2::new(max.x, max.y).to_array(),
                color: Vec4::new(1.0, 1.0, 1.0, 0.5).to_array(),
            });

            // Top
            render_world.ui_rects.push(UiRect {
                min: Vec2::new(min.x + THICKNESS, min.y).to_array(),
                max: Vec2::new(max.x - THICKNESS, min.y + THICKNESS).to_array(),
                color: Vec4::new(1.0, 1.0, 1.0, 0.5).to_array(),
            });

            // Bottom
            render_world.ui_rects.push(UiRect {
                min: Vec2::new(min.x + THICKNESS, max.y - THICKNESS).to_array(),
                max: Vec2::new(max.x - THICKNESS, max.y).to_array(),
                color: Vec4::new(1.0, 1.0, 1.0, 0.5).to_array(),
            });
        }
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
