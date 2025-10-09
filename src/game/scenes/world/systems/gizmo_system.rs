use crate::{
    engine::{
        gizmos::GizmoVertex,
        prelude::{BufferLayout, Frame, Renderer},
    },
    game::scenes::world::{render_world::RenderWorld, sim_world::SimWorld, systems::RenderStore},
    wgsl_shader,
};

pub struct GizmoSystem {
    pipeline: wgpu::RenderPipeline,
}

impl GizmoSystem {
    pub fn new(renderer: &Renderer, render_store: &RenderStore) -> Self {
        let device = &renderer.device;

        let module = device.create_shader_module(wgsl_shader!("gizmos"));

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gizmos_pipeline_layout"),
            bind_group_layouts: &[&render_store.camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gizmos_render_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vertex_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[GizmoVertex::layout()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fragment_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: renderer.surface.format(),
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        Self { pipeline }
    }
}

impl GizmoSystem {
    pub fn extract(&mut self, sim_world: &mut SimWorld, render_world: &mut RenderWorld) {
        // Move all the sim world vertices to the render world vertices.
        render_world.gizmo_vertices.clear();
        std::mem::swap(
            &mut sim_world.gizmo_vertices,
            &mut render_world.gizmo_vertices,
        );
    }

    pub fn prepare(&mut self, render_world: &mut RenderWorld, renderer: &Renderer) {
        render_world.ensure_gizmo_vertices_capacity(
            &renderer.device,
            render_world.gizmo_vertices.len() as u32,
        );

        let data = bytemuck::cast_slice(&render_world.gizmo_vertices);

        renderer
            .queue
            .write_buffer(&render_world.gizmo_vertices_buffer, 0, data);
    }

    pub fn queue(&mut self, render_world: &RenderWorld, frame: &mut Frame) {
        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gizmos_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.surface,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, render_world.gizmo_vertices_buffer.slice(..));
        render_pass.set_bind_group(0, &render_world.camera_env_bind_group, &[]);
        render_pass.draw(0..(render_world.gizmo_vertices.len() as u32), 0..1);
    }
}
