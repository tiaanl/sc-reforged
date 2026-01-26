use crate::{
    engine::renderer::{Frame, Renderer},
    game::{
        AssetReader,
        scenes::world::{
            extract::RenderSnapshot,
            render::{
                RenderLayouts, RenderUiRect, RenderWorld, render_pipeline::RenderPipeline,
                render_world::UiStateLayout,
            },
        },
    },
    wgsl_shader,
};

pub struct UiRenderPipeline {
    rect_render_pipeline: wgpu::RenderPipeline,
}

impl RenderPipeline for UiRenderPipeline {
    fn prepare(
        &mut self,
        _assets: &AssetReader,
        renderer: &Renderer,
        render_world: &mut RenderWorld,
        snapshot: &RenderSnapshot,
    ) {
        let state = gpu::State {
            view_proj: snapshot.ui.proj_view.to_cols_array_2d(),
        };

        renderer
            .queue
            .write_buffer(&render_world.ui_state_buffer, 0, bytemuck::bytes_of(&state));

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

        render_world.ui_rects_buffer.write(renderer, &rects);
    }

    fn queue(
        &self,
        render_world: &RenderWorld,
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

        render_pass.set_vertex_buffer(0, render_world.ui_rects_buffer.slice(..));
        render_pass.set_bind_group(0, &render_world.ui_state_bind_group, &[]);

        render_pass.draw(0..4, 0..(snapshot.ui.ui_rects.len() as u32));
    }
}

impl UiRenderPipeline {
    pub fn new(
        renderer: &Renderer,
        surface_format: wgpu::TextureFormat,
        layouts: &mut RenderLayouts,
    ) -> Self {
        let device = &renderer.device;

        let module = device.create_shader_module(wgsl_shader!("ui"));

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ui_rect_pipeline_layout"),
            bind_group_layouts: &[layouts.get::<UiStateLayout>(renderer)],
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
