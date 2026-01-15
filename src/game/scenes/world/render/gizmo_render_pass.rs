use crate::{
    engine::{
        gizmos::GizmoVertex,
        renderer::{Frame, Renderer},
    },
    game::scenes::world::render::{RenderStore, RenderWorld},
    wgsl_shader,
};

#[derive(Default)]
pub struct GizmoRenderSnapshot {
    pub vertices: Vec<GizmoVertex>,
}

pub struct GizmoRenderPass {
    pipeline: wgpu::RenderPipeline,
}

impl GizmoRenderPass {
    pub fn new(
        renderer: &Renderer,
        surface_format: wgpu::TextureFormat,
        render_store: &RenderStore,
    ) -> Self {
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
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GizmoVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x4, // position
                        1 => Float32x4, // color
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
                module: &module,
                entry_point: Some("fragment_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
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

impl GizmoRenderPass {
    pub fn prepare(
        &mut self,
        render_world: &mut RenderWorld,
        renderer: &Renderer,
        snapshot: &GizmoRenderSnapshot,
    ) {
        render_world
            .gizmo_vertices_buffer
            .write(renderer, &snapshot.vertices);
    }

    pub fn queue(
        &mut self,
        render_world: &RenderWorld,
        snapshot: &GizmoRenderSnapshot,
        frame: &mut Frame,
    ) {
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
        render_pass.draw(0..(snapshot.vertices.len() as u32), 0..1);
    }
}
