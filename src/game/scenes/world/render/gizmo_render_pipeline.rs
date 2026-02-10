use crate::{
    engine::{
        gizmos::GizmoVertex,
        growing_buffer::GrowingBuffer,
        renderer::{Frame, Renderer},
        shader_cache::{ShaderCache, ShaderSource},
    },
    game::{
        AssetReader,
        scenes::world::{
            extract::RenderSnapshot,
            render::{
                GeometryBuffer, RenderBindings, RenderLayouts,
                camera_render_pipeline::CameraEnvironmentLayout, per_frame::PerFrame,
                render_pipeline::RenderPipeline,
            },
        },
    },
};

pub struct GizmoRenderPipeline {
    pipeline: wgpu::RenderPipeline,

    instances_buffer: PerFrame<GrowingBuffer<GizmoVertex>>,
}

impl GizmoRenderPipeline {
    pub fn new(
        renderer: &Renderer,
        surface_format: wgpu::TextureFormat,
        layouts: &mut RenderLayouts,
        shader_cache: &mut ShaderCache,
    ) -> Self {
        let device = &renderer.device;

        let module = shader_cache.get_or_create(&renderer.device, ShaderSource::Gizmos);

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gizmos_pipeline_layout"),
            bind_group_layouts: &[layouts.get::<CameraEnvironmentLayout>(renderer)],
            push_constant_ranges: &[],
        });
        let gizmo_vertex_layout = <GizmoVertex as renderer::AsVertexLayout>::vertex_buffer_layout();
        let gizmo_vertex_attributes =
            renderer::to_wgpu_vertex_attributes(gizmo_vertex_layout.attributes);
        let gizmo_vertex_buffers = [gizmo_vertex_layout.to_wgpu(&gizmo_vertex_attributes)];

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gizmos_render_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module,
                entry_point: Some("vertex_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &gizmo_vertex_buffers,
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module,
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
        _bindings: &mut RenderBindings,
        snapshot: &RenderSnapshot,
    ) {
        let instances = self.instances_buffer.advance();
        instances.write(renderer, &snapshot.gizmos.vertices);
    }

    fn queue(
        &self,
        bindings: &RenderBindings,
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
        render_pass.set_bind_group(0, &bindings.camera_env_buffer.current().bind_group, &[]);
        render_pass.draw(0..(snapshot.gizmos.vertices.len() as u32), 0..1);
    }
}
