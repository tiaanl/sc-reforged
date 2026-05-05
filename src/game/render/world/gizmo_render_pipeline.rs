use crate::{
    engine::{
        gizmos::GizmoVertex,
        growing_buffer::GrowingBuffer,
        renderer::{Gpu, RenderContext},
        shader_cache::{ShaderCache, ShaderSource},
    },
    game::render::{
        geometry_buffer::GeometryBuffer,
        per_frame::PerFrame,
        world::{
            camera_render_pipeline::CameraEnvironmentLayout, render_bindings::RenderBindings,
            render_layouts::RenderLayouts, render_pipeline::RenderPipeline,
            world_render_snapshot::WorldRenderSnapshot,
        },
    },
};

pub struct GizmoRenderPipeline {
    pipeline: wgpu::RenderPipeline,

    instances_buffer: PerFrame<GrowingBuffer<GizmoVertex>>,
}

impl GizmoRenderPipeline {
    pub fn new(gpu: &Gpu, layouts: &mut RenderLayouts, shader_cache: &mut ShaderCache) -> Self {
        let device = &gpu.device;

        let module = shader_cache.get_or_create(&gpu.device, ShaderSource::Gizmos);

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gizmos_pipeline_layout"),
            bind_group_layouts: &[layouts.get::<CameraEnvironmentLayout>()],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gizmos_render_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module,
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
            depth_stencil: Some(wgpu::DepthStencilState {
                format: GeometryBuffer::DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module,
                entry_point: Some("fragment_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: GeometryBuffer::opaque_targets(),
            }),
            multiview: None,
            cache: None,
        });

        let instances_buffer = PerFrame::new(|index| {
            GrowingBuffer::new(
                gpu,
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
        gpu: &Gpu,
        _bindings: &mut RenderBindings,
        snapshot: &WorldRenderSnapshot,
    ) {
        let instances = self.instances_buffer.advance();
        instances.write(gpu, &snapshot.gizmos.vertices);
    }

    fn queue(
        &self,
        bindings: &RenderBindings,
        render_context: &mut RenderContext,
        geometry_buffer: &GeometryBuffer,
        snapshot: &WorldRenderSnapshot,
    ) {
        let mut render_pass = geometry_buffer
            .begin_opaque_render_pass(&mut render_context.encoder, "gizmos_render_pass");

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.instances_buffer.current().slice(..));
        render_pass.set_bind_group(0, &bindings.camera_env_buffer.current().bind_group, &[]);
        render_pass.draw(0..(snapshot.gizmos.vertices.len() as u32), 0..1);
    }
}
