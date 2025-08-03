use glam::{IVec2, UVec4};
use wgpu::util::DeviceExt;

use crate::engine::prelude::*;
use crate::engine::ui::geometry::Color;
use crate::engine::ui::{Rect, Size};

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
struct Vertex {
    position: [i32; 4],
    tex_coord: [f32; 4],
    color: [f32; 4],
}

/// Data about the context being rendered that is stored on the GPU.
#[derive(Clone, Copy, Default, bytemuck::NoUninit)]
#[repr(C)]
struct ContextData {
    /// The current size of the window in screen pixels.
    screen_size: IVec2,
    /// The size of a UI pixel in screen pixels.
    pixel_size: i32,

    _dummy: f32,
}

enum RenderCommand {
    Color { rect: Rect, color: Vec4 },
}

impl RenderCommand {
    pub fn color(rect: Rect, color: Vec4) -> Self {
        Self::Color { rect, color }
    }
}

pub struct RenderContext {
    pipeline: wgpu::RenderPipeline,

    context_data_buffer: wgpu::Buffer,
    context_data_bind_group: wgpu::BindGroup,

    context_data: Tracked<ContextData>,

    commands: Vec<RenderCommand>,
}

impl RenderContext {
    pub fn new(renderer: &Renderer, pixel_size: i32) -> Self {
        let context_data = ContextData {
            screen_size: IVec2::ONE, // Avoid division by 0.
            pixel_size,
            _dummy: 0.0,
        };

        let context_data_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("ui_context_data_buffer"),
                    contents: bytemuck::cast_slice(&[context_data]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let context_data_bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("context_data_bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let context_data_bind_group =
            renderer
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("ui_context_data_bind_group"),
                    layout: &context_data_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: context_data_buffer.as_entire_binding(),
                    }],
                });

        const SHADER: &str = r"
            struct ContextData {
                screen_size: vec2<i32>,
                pixel_size: i32,
            }

            @group(0)
            @binding(0)
            var<uniform> context_data: ContextData;

            struct VertexInput {
                @location(0) position: vec4<i32>,
                @location(1) tex_coord: vec4<f32>,
                @location(2) color: vec4<f32>,
            }

            struct VertexOutput {
                @builtin(position) position: vec4<f32>,
                @location(0) tex_coord: vec4<f32>,
                @location(1) color: vec4<f32>,
            }

            @vertex
            fn vertex(vertex: VertexInput) -> VertexOutput {
                let x =
                    (
                        f32(vertex.position.x) /
                        f32(context_data.pixel_size) /
                        f32(context_data.screen_size.x)
                    ) * 2.0 - 1.0;

                let y =
                    1.0 - (
                        f32(vertex.position.y) /
                        f32(context_data.pixel_size) /
                        f32(context_data.screen_size.y)
                    ) * 2.0;

                return VertexOutput(
                    vec4<f32>(x, y, 0.0, 1.0),
                    vertex.tex_coord,
                    vertex.color,
                );
            }

            @fragment
            fn fragment(vertex: VertexOutput) -> @location(0) vec4<f32> {
                return vertex.color;
            }
        ";
        let module = renderer.create_shader_module("ui", SHADER);

        let pipeline_layout =
            renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("ui_pipeline_layout"),
                    bind_group_layouts: &[&context_data_bind_group_layout],
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
                        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![
                            0 => Sint32x4,
                            1 => Float32x4,
                            2 => Float32x4,
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
                        format: renderer.surface.format(),
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            });

        Self {
            pipeline,

            context_data_buffer,
            context_data_bind_group,

            context_data: Tracked::new(context_data),

            commands: Vec::default(),
        }
    }

    pub fn render_color(&mut self, rect: Rect, color: Vec4) {
        self.commands.push(RenderCommand::color(rect, color));
    }

    pub fn resize(&mut self, screen_size: Size) {
        self.context_data.screen_size = IVec2::new(screen_size.width, screen_size.height);
    }

    pub fn render(&mut self, frame: &mut Frame) {
        // Update the context data if needed.
        self.context_data.if_changed(|context_data| {
            frame.queue.write_buffer(
                &self.context_data_buffer,
                0,
                bytemuck::cast_slice(&[*context_data]),
            );
        });

        let mut mesh = IndexedMesh::<Vertex>::default();

        self.commands.iter().for_each(|command| match command {
            RenderCommand::Color { rect, color } => {
                let first_vertex = mesh.vertices.len() as u32;

                mesh.vertices.push(Vertex {
                    position: [rect.pos.left, rect.pos.top, 0, 0],
                    tex_coord: [0.0, 0.0, 0.0, 0.0],
                    color: color.to_array(),
                });
                mesh.vertices.push(Vertex {
                    position: [rect.pos.left + rect.size.width, rect.pos.top, 0, 0],
                    tex_coord: [1.0, 0.0, 0.0, 0.0],
                    color: color.to_array(),
                });
                mesh.vertices.push(Vertex {
                    position: [
                        rect.pos.left + rect.size.width,
                        rect.pos.top + rect.size.height,
                        0,
                        0,
                    ],
                    tex_coord: [1.0, 1.0, 0.0, 0.0],
                    color: color.to_array(),
                });
                mesh.vertices.push(Vertex {
                    position: [rect.pos.left, rect.pos.top + rect.size.height, 0, 0],
                    tex_coord: [0.0, 1.0, 0.0, 0.0],
                    color: color.to_array(),
                });

                mesh.indices.extend_from_slice(&[
                    first_vertex,
                    first_vertex + 1,
                    first_vertex + 2,
                    first_vertex + 2,
                    first_vertex + 3,
                    first_vertex,
                ]);
            }
        });

        let gpu_mesh = mesh.to_gpu(frame.renderer);

        // let mut render_pass = frame.begin_basic_render_pass("ui", false);
        // render_pass.set_pipeline(&self.pipeline);
        // render_pass.set_bind_group(0, &self.context_data_bind_group, &[]);
        // render_pass.set_index_buffer(gpu_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        // render_pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
        // render_pass.draw_indexed(0..gpu_mesh.index_count, 0, 0..1);

        self.commands.clear();
    }
}

impl From<Color> for Vec4 {
    fn from(value: Color) -> Self {
        Self::new(
            value.red as f32 / 255.0,
            value.green as f32 / 255.0,
            value.blue as f32 / 255.0,
            value.alpha as f32 / 255.0,
        )
    }
}
