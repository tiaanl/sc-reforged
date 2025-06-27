use glam::UVec4;
use wgpu::util::DeviceExt;

use crate::engine::prelude::*;
use crate::engine::ui::Rect;

enum RenderCommand {
    Color { rect: Rect },
}

impl RenderCommand {
    pub fn color(rect: Rect) -> Self {
        Self::Color { rect }
    }
}

pub struct RenderContext {
    pipeline: wgpu::RenderPipeline,

    commands: Vec<RenderCommand>,
}

impl RenderContext {
    pub fn new(renderer: &Renderer) -> Self {
        const SHADER: &str = r"
            @vertex fn vertex() -> @builtin(position) vec4<f32> {
                return vec4<f32>(0.0, 0.0, 0.0, 1.0);
            }

            @fragment fn fragment() -> @location(0) vec4<f32> {
                return vec4<f32>(0.2, 0.3, 0.4, 1.0);
            }
        ";
        let module = renderer.create_shader_module("ui", SHADER);

        let pipeline_layout =
            renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("ui_pipeline_layout"),
                    bind_group_layouts: &[],
                    push_constant_ranges: &[],
                });

        let pipeline = renderer
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("ui_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![
                            0 => Uint32x4,
                            1 => Float32x4,
                        ],
                    }],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: renderer.surface_config.format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            });

        Self {
            pipeline,
            commands: Vec::default(),
        }
    }

    pub fn render_color(&mut self, rect: Rect) {
        self.commands.push(RenderCommand::color(rect));
    }

    pub fn render(&mut self, frame: &mut Frame) {
        #[derive(Clone, Copy, bytemuck::NoUninit)]
        #[repr(C)]
        struct Data {
            udata: UVec4,
            fdata: Vec4,
        }

        let buffer_data: Vec<Data> = self
            .commands
            .iter()
            .map(|command| match command {
                RenderCommand::Color { rect } => Data {
                    udata: UVec4::new(
                        rect.pos.left,
                        rect.pos.top,
                        rect.size.width,
                        rect.size.height,
                    ),
                    fdata: Vec4::ZERO,
                },
            })
            .collect();

        let buffer = frame
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("render_object_data"),
                contents: bytemuck::cast_slice(&buffer_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

        let mut render_pass = frame.begin_basic_render_pass("ui", false);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, buffer.slice(..));
        render_pass.draw(0..4, 0..1);
    }
}
