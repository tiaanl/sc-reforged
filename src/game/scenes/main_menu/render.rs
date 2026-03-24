use glam::{UVec2, Vec2};
use wgpu::util::DeviceExt;

use crate::engine::{
    growing_buffer::GrowingBuffer,
    renderer::{Frame, Renderer, SurfaceDesc},
};

pub type TextureId = usize;

pub enum Primitive {
    Rect {
        pos: Vec2,
        size: Vec2,
        texture: TextureId,
        alpha: f32,
    },
}

#[derive(Default)]
pub struct Primitives {
    primitives: Vec<Primitive>,
}

impl Primitives {
    pub fn add_rect(&mut self, pos: Vec2, size: Vec2, texture: TextureId, alpha: f32) {
        self.primitives.push(Primitive::Rect {
            pos,
            size,
            texture,
            alpha,
        });
    }
}

struct Texture {
    size: UVec2,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
}

pub struct WindowRenderer {
    render_pipeline: wgpu::RenderPipeline,

    vertices: Vec<gpu::Vertex>,
    vertices_buffer: GrowingBuffer<gpu::Vertex>,

    indices: Vec<u32>,
    indices_buffer: GrowingBuffer<u32>,

    viewport_dirty: bool,
    viewport: gpu::Viewport,
    viewport_buffer: wgpu::Buffer,
    viewport_bind_group: wgpu::BindGroup,

    texture_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    textures: Vec<Texture>,
}

impl WindowRenderer {
    pub fn new(renderer: &Renderer, surface: &SurfaceDesc) -> Self {
        let viewport = gpu::Viewport {
            size: surface.size.as_vec2().to_array(),
        };

        let viewport_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("viewport_buffer"),
                    contents: bytemuck::bytes_of(&viewport),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let viewport_bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("viewport_bind_group_layout"),
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

        let viewport_bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("viewport_bind_group"),
                layout: &viewport_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: viewport_buffer.as_entire_binding(),
                }],
            });

        let texture_bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("texture"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let sampler = renderer.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let shader = renderer
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("window_renderer"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                    "window.wgsl"
                ))),
            });

        let pipeline_layout =
            renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("window_renderer"),
                    bind_group_layouts: &[&viewport_bind_group_layout, &texture_bind_group_layout],
                    push_constant_ranges: &[wgpu::PushConstantRange {
                        stages: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        range: 0..20,
                    }],
                });

        let render_pipeline =
            renderer
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("window_renderer"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: None,
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<gpu::Vertex>() as wgpu::BufferAddress,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &wgpu::vertex_attr_array![
                                0 => Float32x2,
                                1 => Float32x2,
                                2 => Float32x4,
                            ],
                        }],
                    },
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: None,
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: surface.format,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview: None,
                    cache: None,
                });

        let vertices = vec![
            gpu::Vertex {
                pos: [0.0, 0.0],
                uv: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            gpu::Vertex {
                pos: [1.0, 0.0],
                uv: [1.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            gpu::Vertex {
                pos: [1.0, 1.0],
                uv: [1.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            gpu::Vertex {
                pos: [0.0, 1.0],
                uv: [0.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
        ];

        let mut vertices_buffer = GrowingBuffer::new(
            renderer,
            vertices.len() as u32,
            wgpu::BufferUsages::VERTEX,
            "window_renderer_vertices",
        );
        vertices_buffer.write(renderer, &vertices);

        let indices = vec![0, 1, 2, 2, 3, 0];

        let mut indices_buffer = GrowingBuffer::new(
            renderer,
            indices.len() as u32,
            wgpu::BufferUsages::INDEX,
            "window_renderer_indices",
        );
        indices_buffer.write(renderer, &indices);

        Self {
            render_pipeline,

            vertices,
            vertices_buffer,
            indices,
            indices_buffer,

            viewport_dirty: false,
            viewport,
            viewport_buffer,
            viewport_bind_group,

            texture_bind_group_layout,
            sampler,
            textures: Vec::default(),
        }
    }

    pub fn create_texture(&mut self, renderer: &Renderer, rgba: image::RgbaImage) -> TextureId {
        let size = wgpu::Extent3d {
            width: rgba.width(),
            height: rgba.height(),
            depth_or_array_layers: 1,
        };

        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        renderer.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(rgba.width() * 4),
                rows_per_image: Some(rgba.height()),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let id = self.textures.len();

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("texture_bind_group_{id}")),
                layout: &self.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });

        self.textures.push(Texture {
            size: UVec2::new(rgba.width(), rgba.height()),
            view,
            bind_group,
        });

        id
    }

    pub fn resize(&mut self, size: UVec2) {
        self.viewport = gpu::Viewport {
            size: size.as_vec2().to_array(),
        };
        self.viewport_dirty = true;
    }

    pub fn submit(&mut self, renderer: &Renderer, frame: &mut Frame, primitives: Primitives) {
        if self.viewport_dirty {
            renderer.queue.write_buffer(
                &self.viewport_buffer,
                0,
                bytemuck::bytes_of(&self.viewport),
            );
        }

        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("window_renderer"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.surface,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertices_buffer.slice(..));
        render_pass.set_index_buffer(self.indices_buffer.slice(..), wgpu::IndexFormat::Uint32);

        for p in primitives.primitives.iter() {
            match p {
                Primitive::Rect {
                    pos,
                    size,
                    texture,
                    alpha,
                } => {
                    let Some(texture) = self.textures.get(*texture) else {
                        continue;
                    };

                    let bytes = [pos.x, pos.y, size.x, size.y, *alpha];

                    render_pass.set_push_constants(
                        wgpu::ShaderStages::VERTEX_FRAGMENT,
                        0,
                        bytemuck::bytes_of(&bytes),
                    );
                    render_pass.set_bind_group(0, &self.viewport_bind_group, &[]);
                    render_pass.set_bind_group(1, &texture.bind_group, &[]);
                    render_pass.draw_indexed(0..self.indices_buffer.count, 0, 0..1);
                }
            }
        }
    }
}

pub mod gpu {
    use bytemuck::NoUninit;

    #[derive(Clone, Copy, NoUninit)]
    #[repr(C)]
    pub struct Viewport {
        pub size: [f32; 2],
    }

    #[derive(Clone, Copy, NoUninit)]
    #[repr(C)]
    pub struct Vertex {
        pub pos: [f32; 2],
        pub uv: [f32; 2],
        pub color: [f32; 4],
    }
}
