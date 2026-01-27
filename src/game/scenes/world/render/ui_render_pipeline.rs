use crate::{
    engine::{
        growing_buffer::GrowingBuffer,
        renderer::{Frame, Renderer},
    },
    game::{
        AssetReader,
        scenes::world::{
            extract::RenderSnapshot,
            render::{
                RenderLayouts, RenderWorld, per_frame::PerFrame, render_layouts::RenderLayout,
                render_pipeline::RenderPipeline, ui_render_pipeline, uniform_buffer::UniformBuffer,
            },
        },
    },
    wgsl_shader,
};

pub struct UiStateLayout;

impl RenderLayout for UiStateLayout {
    fn label() -> &'static str {
        "ui_state_bind_group_layout"
    }

    fn entries() -> &'static [wgpu::BindGroupLayoutEntry] {
        const ENTRIES: &[wgpu::BindGroupLayoutEntry] = &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }];

        ENTRIES
    }
}

pub struct UiRenderPipeline {
    rect_render_pipeline: wgpu::RenderPipeline,

    state_uniform: PerFrame<UniformBuffer, 3>,
    rects_buffer: PerFrame<GrowingBuffer<gpu::Rect>, 3>,
}

impl UiRenderPipeline {
    pub fn new(
        renderer: &Renderer,
        surface_format: wgpu::TextureFormat,
        layouts: &mut RenderLayouts,
    ) -> Self {
        let module = renderer.device.create_shader_module(wgsl_shader!("ui"));

        let layout = renderer
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ui_rect_pipeline_layout"),
                bind_group_layouts: &[layouts.get::<UiStateLayout>(renderer)],
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
                            array_stride: std::mem::size_of::<ui_render_pipeline::gpu::Rect>()
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
                            format: surface_format,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview: None,
                    cache: None,
                });

        let state_uniform = {
            let layout = layouts.get::<UiStateLayout>(renderer);

            PerFrame::new(|index| {
                let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("ui_state_buffer_{index}")),
                    size: std::mem::size_of::<gpu::State>() as wgpu::BufferAddress,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
                    mapped_at_creation: false,
                });

                let bind_group = renderer
                    .device
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some(&format!("ui_state_bind_group_{index}")),
                        layout,
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: buffer.as_entire_binding(),
                        }],
                    });

                UniformBuffer::new(buffer, bind_group)
            })
        };

        let rects_buffer = PerFrame::new(|index| {
            GrowingBuffer::new(
                renderer,
                1024,
                wgpu::BufferUsages::VERTEX,
                format!("ui_rects_buffer:{index}"),
            )
        });

        Self {
            rect_render_pipeline,
            state_uniform,
            rects_buffer,
        }
    }
}

impl RenderPipeline for UiRenderPipeline {
    fn prepare(
        &mut self,
        _assets: &AssetReader,
        renderer: &Renderer,
        _render_world: &mut RenderWorld,
        snapshot: &RenderSnapshot,
    ) {
        let state = gpu::State {
            view_proj: snapshot.ui.proj_view.to_cols_array_2d(),
        };

        let state_uniform = self.state_uniform.advance();
        state_uniform.write(renderer, bytemuck::bytes_of(&state));

        let rects: Vec<_> = snapshot
            .ui
            .ui_rects
            .iter()
            .map(|rect| gpu::Rect {
                min: rect.min.to_array(),
                max: rect.max.to_array(),
                color: rect.color.to_array(),
            })
            .collect();

        let rects_buffer = self.rects_buffer.advance();
        rects_buffer.write(renderer, rects.as_slice());
    }

    fn queue(
        &self,
        _render_world: &RenderWorld,
        frame: &mut Frame,
        _geometry_buffer: &super::GeometryBuffer,
        snapshot: &RenderSnapshot,
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

        render_pass.set_vertex_buffer(0, self.rects_buffer.current().slice(..));
        render_pass.set_bind_group(0, &self.state_uniform.current().bind_group, &[]);

        render_pass.draw(0..4, 0..(snapshot.ui.ui_rects.len() as u32));
    }
}

pub mod gpu {
    use bytemuck::NoUninit;

    #[derive(Clone, Copy, NoUninit)]
    #[repr(C)]
    pub struct State {
        pub view_proj: [[f32; 4]; 4],
    }

    #[derive(Clone, Copy, NoUninit)]
    #[repr(C)]
    pub struct Rect {
        pub min: [f32; 2],
        pub max: [f32; 2],
        pub color: [f32; 4],
    }
}
