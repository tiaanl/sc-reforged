use crate::{
    engine::{
        gizmos::GizmoVertex,
        growing_buffer::GrowingBuffer,
        renderer::{Frame, Renderer},
    },
    game::{
        AssetReader,
        scenes::world::{
            extract::RenderSnapshot,
            render::{
                GeometryBuffer, RenderLayouts, RenderBindings,
                camera_render_pipeline::CameraEnvironmentLayout, per_frame::PerFrame,
                render_pipeline::RenderPipeline,
            },
        },
    },
    wgsl_shader,
};

pub struct GizmoRenderPipeline {
    pipeline: wgpu::RenderPipeline,

    instances_buffer: PerFrame<GrowingBuffer<GizmoVertex>, 3>,
}

impl GizmoRenderPipeline {
    pub fn new(
        renderer: &Renderer,
        surface_format: wgpu::TextureFormat,
        layouts: &mut RenderLayouts,
    ) -> Self {
        let device = &renderer.device;

        let module = device.create_shader_module(wgsl_shader!("gizmos"));

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gizmos_pipeline_layout"),
            bind_group_layouts: &[layouts.get::<CameraEnvironmentLayout>(renderer)],
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

        let instances_buffer = PerFrame::new(|index| {
            GrowingBuffer::new(
                renderer,
                1024,
                wgpu::BufferUsages::VERTEX,
                format!("gizmo_vertices:{index}"),
            )
        });

        Self {
            pipeline,
            instances_buffer,
        }
    }
}

impl RenderPipeline for GizmoRenderPipeline {
    fn prepare(
        &mut self,
        _assets: &AssetReader,
        renderer: &Renderer,
        _render_world: &mut RenderBindings,
        snapshot: &RenderSnapshot,
    ) {
        let instances = self.instances_buffer.advance();
        instances.write(renderer, &snapshot.gizmos.vertices);
    }

    fn queue(
        &self,
        render_world: &RenderBindings,
        frame: &mut Frame,
        _geometry_buffer: &GeometryBuffer,
        snapshot: &RenderSnapshot,
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
        render_pass.set_vertex_buffer(0, self.instances_buffer.current().slice(..));
        render_pass.set_bind_group(0, &render_world.camera_env_buffer.bind_group, &[]);
        render_pass.draw(0..(snapshot.gizmos.vertices.len() as u32), 0..1);
    }
}
