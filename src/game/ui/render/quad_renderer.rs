use ahash::HashMap;
use glam::{IVec2, UVec2, Vec2, Vec4};
use wgpu::util::DeviceExt;

use crate::{
    engine::{
        growing_buffer::GrowingBuffer,
        renderer::{Gpu, RenderContext, RenderTarget, SurfaceDesc},
        storage::Handle,
    },
    game::{
        assets::{
            asset_source::AssetSource,
            image::{BlendMode, Image},
        },
        globals,
        render::textures::Texture,
        ui::Rect,
    },
};

#[derive(Clone, Copy, Debug)]
pub enum QuadKind {
    Solid,
    Texture {
        texture: Handle<Texture>,
    },
    SubTexture {
        texture: Handle<Texture>,
        uv_min: Vec2,
        uv_max: Vec2,
    },
}

/// A fully resolved textured quad ready for rendering.
#[derive(Clone, Copy, Debug)]
pub struct Quad {
    pub rect: Rect,
    pub color: Vec4,
    pub clip_rect: Option<Rect>,
    pub kind: QuadKind,
}

impl Quad {
    pub fn solid(rect: Rect) -> Self {
        Self {
            rect,
            ..Default::default()
        }
    }

    pub fn texture(rect: Rect, texture: Handle<Texture>) -> Self {
        Self {
            rect,
            kind: QuadKind::Texture { texture },
            ..Default::default()
        }
    }

    pub fn sub_texture(rect: Rect, texture: Handle<Texture>, uv_min: Vec2, uv_max: Vec2) -> Self {
        Self {
            rect,
            kind: QuadKind::SubTexture {
                texture,
                uv_min,
                uv_max,
            },
            ..Default::default()
        }
    }

    #[must_use]
    pub fn with_color(mut self, color: Vec4) -> Self {
        self.color = color;
        self
    }

    #[must_use]
    pub fn with_clip_rect(mut self, clip_rect: Rect) -> Self {
        self.clip_rect = Some(clip_rect);
        self
    }
}

impl Default for Quad {
    fn default() -> Self {
        Self {
            rect: Rect::default(),
            color: Vec4::ONE,
            clip_rect: None,
            kind: QuadKind::Solid,
        }
    }
}

pub struct QuadRenderer {
    gpu: Gpu,

    solid_white_texture: Handle<Texture>,

    render_pipeline: wgpu::RenderPipeline,

    vertices_buffer: GrowingBuffer<gpu::Vertex>,

    indices_buffer: GrowingBuffer<u32>,

    instances_buffer: GrowingBuffer<gpu::RectInstance>,

    new_size: Option<UVec2>,
    viewport_buffer: wgpu::Buffer,
    viewport_bind_group: wgpu::BindGroup,

    texture_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    bind_groups: HashMap<Handle<Texture>, wgpu::BindGroup>,
}

impl QuadRenderer {
    /// Creates the quad renderer and its GPU state for menu quads.
    pub fn new(gpu: Gpu, surface: &SurfaceDesc) -> Self {
        let Gpu { device, .. } = &gpu;

        let viewport = gpu::Viewport::from(surface.size);

        let viewport_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad_renderer_viewport_buffer"),
            contents: bytemuck::bytes_of(&viewport),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let viewport_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("quad_renderer_viewport_bind_group_layout"),
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

        let viewport_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("quad_renderer_viewport_bind_group"),
            layout: &viewport_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: viewport_buffer.as_entire_binding(),
            }],
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("quad_renderer_texture_bind_group_layout"),
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

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("quad_renderer_sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("quad_renderer_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "window.wgsl"
            ))),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("quad_renderer_pipeline_layout"),
            bind_group_layouts: &[&viewport_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("quad_renderer_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: None,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<gpu::Vertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![
                            0 => Float32x2,
                            1 => Float32x2,
                            2 => Float32x4,
                        ],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<gpu::RectInstance>()
                            as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![
                            3 => Float32x2,
                            4 => Float32x2,
                            5 => Float32x4,
                            6 => Float32x2,
                            7 => Float32x2,
                        ],
                    },
                ],
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

        let vertices = [
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
            &gpu,
            vertices.len() as u32,
            wgpu::BufferUsages::VERTEX,
            "quad_renderer_vertices",
        );
        vertices_buffer.write(&gpu, &vertices);

        let indices = [0, 1, 2, 2, 3, 0];

        let mut indices_buffer = GrowingBuffer::new(
            &gpu,
            indices.len() as u32,
            wgpu::BufferUsages::INDEX,
            "quad_renderer_indices",
        );
        indices_buffer.write(&gpu, &indices);

        let instances_buffer = GrowingBuffer::new(
            &gpu,
            64,
            wgpu::BufferUsages::VERTEX,
            "quad_renderer_instances",
        );

        let white_image = globals::images().insert(
            "solid_white",
            Image::from_rgba(
                AssetSource::Generated,
                image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 255, 255, 255])),
                BlendMode::Opaque,
            ),
        );
        let solid_white_texture = globals::textures()
            .create_from_image(white_image)
            .expect("generated solid white texture should be valid");

        Self {
            gpu,
            solid_white_texture,
            render_pipeline,
            vertices_buffer,
            indices_buffer,
            instances_buffer,
            new_size: None,
            viewport_buffer,
            viewport_bind_group,
            texture_bind_group_layout,
            sampler,
            bind_groups: HashMap::default(),
        }
    }

    /// Queues a viewport resize to be applied the next time quads are submitted.
    pub fn resize(&mut self, size: UVec2) {
        self.new_size = Some(size);
    }

    /// Uploads the provided quads and renders them in order.
    pub fn submit(
        &mut self,
        render_context: &mut RenderContext,
        render_target: &RenderTarget,
        quads: &[Quad],
    ) {
        if let Some(new_size) = self.new_size.take() {
            let viewport = gpu::Viewport::from(new_size);

            self.gpu
                .queue
                .write_buffer(&self.viewport_buffer, 0, bytemuck::bytes_of(&viewport));
        }

        let drawable_quads: Vec<_> = quads
            .iter()
            .filter_map(|quad| {
                let texture = match quad.kind {
                    QuadKind::Solid => self.solid_white_texture,
                    QuadKind::Texture { texture } | QuadKind::SubTexture { texture, .. } => texture,
                };

                let (uv_min, uv_max) =
                    if let QuadKind::SubTexture { uv_min, uv_max, .. } = quad.kind {
                        (uv_min, uv_max)
                    } else {
                        (Vec2::ZERO, Vec2::ONE)
                    };

                self.ensure_bind_group(texture).then_some((
                    texture,
                    quad.clip_rect,
                    gpu::RectInstance {
                        pos: quad.rect.position.as_vec2().to_array(),
                        size: quad.rect.size.as_vec2().to_array(),
                        color: quad.color.to_array(),
                        uv_min: uv_min.to_array(),
                        uv_max: uv_max.to_array(),
                    },
                ))
            })
            .collect();

        let instances: Vec<_> = drawable_quads
            .iter()
            .map(|(_, _, instance)| *instance)
            .collect();
        self.instances_buffer.write(&self.gpu, &instances);

        let mut render_pass =
            render_context
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("quad_renderer_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &render_target.view,
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
        render_pass.set_vertex_buffer(1, self.instances_buffer.slice(..));
        render_pass.set_index_buffer(self.indices_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_bind_group(0, &self.viewport_bind_group, &[]);

        for (index, (texture, clip_rect, _)) in drawable_quads.iter().enumerate() {
            let Some(bind_group) = self.bind_groups.get(texture) else {
                continue;
            };

            if let Some(clip_rect) = clip_rect {
                let clip_min = clip_rect.position.max(IVec2::ZERO);
                let clip_max =
                    (clip_rect.position + clip_rect.size).min(render_target.size.as_ivec2());
                let clip_size = clip_max - clip_min;

                if clip_size.x <= 0 || clip_size.y <= 0 {
                    continue;
                }

                render_pass.set_scissor_rect(
                    clip_min.x as u32,
                    clip_min.y as u32,
                    clip_size.x as u32,
                    clip_size.y as u32,
                );
            } else {
                render_pass.set_scissor_rect(0, 0, render_target.size.x, render_target.size.y);
            }

            render_pass.set_bind_group(1, bind_group, &[]);
            render_pass.draw_indexed(
                0..self.indices_buffer.count,
                0,
                index as u32..index as u32 + 1,
            );
        }
    }

    /// Lazily creates the texture bind group for a quad texture.
    fn ensure_bind_group(&mut self, texture_handle: Handle<Texture>) -> bool {
        if self.bind_groups.contains_key(&texture_handle) {
            return true;
        }

        let Some(texture) = globals::textures().get(texture_handle) else {
            return false;
        };

        let bind_group = self
            .gpu
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("quad_renderer_texture_bind_group"),
                layout: &self.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });

        self.bind_groups.insert(texture_handle, bind_group);
        true
    }
}

pub mod gpu {
    use glam::UVec2;

    use bytemuck::NoUninit;

    #[derive(Clone, Copy, NoUninit)]
    #[repr(C)]
    pub struct Viewport {
        pub size: [f32; 2],
    }

    impl From<UVec2> for Viewport {
        fn from(value: UVec2) -> Self {
            Self {
                size: value.as_vec2().to_array(),
            }
        }
    }

    #[derive(Clone, Copy, NoUninit)]
    #[repr(C)]
    pub struct Vertex {
        pub pos: [f32; 2],
        pub uv: [f32; 2],
        pub color: [f32; 4],
    }

    #[derive(Clone, Copy, NoUninit)]
    #[repr(C)]
    pub struct RectInstance {
        pub pos: [f32; 2],
        pub size: [f32; 2],
        pub color: [f32; 4],
        pub uv_min: [f32; 2],
        pub uv_max: [f32; 2],
    }
}
