use std::sync::Arc;

use ahash::HashMap;
use glam::{UVec2, Vec2};
use wgpu::util::DeviceExt;

use crate::{
    engine::{
        growing_buffer::GrowingBuffer,
        renderer::{Frame, RenderContext, SurfaceDesc},
        storage::Handle,
    },
    game::{
        assets::{
            image::Image,
            sprites::{Sprite3d, Sprites},
        },
        render::textures::{Texture, Textures},
    },
};

// TODO: Move this to a more general place for reuse.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Rect {
    pub pos: UVec2,
    pub size: UVec2,
}

#[derive(Debug)]
pub enum Primitive {
    Texture {
        rect: Rect,
        texture: Handle<Texture>,
        alpha: f32,
    },
    Sprite {
        rect: Rect,
        sprite: Handle<Sprite3d>,
        frame: usize,
    },
}

#[derive(Default)]
pub struct Primitives {
    primitives: Vec<Primitive>,
}

impl Primitives {
    /// Clears all queued sprite primitives.
    pub fn clear(&mut self) {
        self.primitives.clear();
    }

    /// Queues a draw using a full texture.
    pub fn add_texture(&mut self, rect: Rect, texture: Handle<Texture>, alpha: f32) {
        self.primitives.push(Primitive::Texture {
            rect,
            texture,
            alpha,
        });
    }

    /// Queues a sprite draw using a sprite handle and frame index.
    pub fn add_sprite(&mut self, rect: Rect, sprite: Handle<Sprite3d>, frame: usize) {
        self.primitives.push(Primitive::Sprite {
            rect,
            sprite,
            frame,
        });
    }
}

pub struct SpriteRenderer {
    render_context: RenderContext,
    sprites: Arc<Sprites>,
    textures: Arc<Textures>,

    render_pipeline: wgpu::RenderPipeline,

    _vertices: Vec<gpu::Vertex>,
    vertices_buffer: GrowingBuffer<gpu::Vertex>,

    _indices: Vec<u32>,
    indices_buffer: GrowingBuffer<u32>,

    instances_buffer: GrowingBuffer<gpu::RectInstance>,

    viewport_dirty: bool,
    viewport: gpu::Viewport,
    viewport_buffer: wgpu::Buffer,
    viewport_bind_group: wgpu::BindGroup,

    texture_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    bind_groups: HashMap<Handle<Texture>, wgpu::BindGroup>,
}

impl SpriteRenderer {
    /// Creates the sprite renderer and its GPU state for the main menu scene.
    pub fn new(
        render_context: RenderContext,
        surface: &SurfaceDesc,
        sprites: Arc<Sprites>,
        textures: Arc<Textures>,
    ) -> Self {
        let RenderContext { device, .. } = &render_context;

        let viewport = gpu::Viewport {
            size: surface.size.as_vec2().to_array(),
        };

        let viewport_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport_buffer"),
            contents: bytemuck::bytes_of(&viewport),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let viewport_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let viewport_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport_bind_group"),
            layout: &viewport_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: viewport_buffer.as_entire_binding(),
            }],
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("window_context"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "window.wgsl"
            ))),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("window_context"),
            bind_group_layouts: &[&viewport_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("window_context"),
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
                            5 => Float32,
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
            &render_context,
            vertices.len() as u32,
            wgpu::BufferUsages::VERTEX,
            "window_context_vertices",
        );
        vertices_buffer.write(&render_context, &vertices);

        let indices = vec![0, 1, 2, 2, 3, 0];

        let mut indices_buffer = GrowingBuffer::new(
            &render_context,
            indices.len() as u32,
            wgpu::BufferUsages::INDEX,
            "window_context_indices",
        );
        indices_buffer.write(&render_context, &indices);

        let instances_buffer = GrowingBuffer::new(
            &render_context,
            64,
            wgpu::BufferUsages::VERTEX,
            "window_context_instances",
        );

        Self {
            render_context,
            sprites,
            textures,

            render_pipeline,

            _vertices: vertices,
            vertices_buffer,
            _indices: indices,
            indices_buffer,
            instances_buffer,

            viewport_dirty: false,
            viewport,
            viewport_buffer,
            viewport_bind_group,

            texture_bind_group_layout,
            sampler,
            bind_groups: HashMap::default(),
        }
    }

    /// Returns a texture handle for the full source image.
    pub fn create_texture(&mut self, image: Handle<Image>) -> Option<Handle<Texture>> {
        let texture_handle = self.textures.create_from_image(image)?;
        self.ensure_bind_group(texture_handle)
            .then_some(texture_handle)
    }

    fn ensure_bind_group(&mut self, texture_handle: Handle<Texture>) -> bool {
        if self.bind_groups.contains_key(&texture_handle) {
            return true;
        }

        let Some(texture) = self.textures.get(texture_handle) else {
            return false;
        };

        let bind_group = self
            .render_context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("texture_bind_group"),
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

    /// Updates the viewport used to convert pixel coordinates into clip space.
    pub fn resize(&mut self, size: UVec2) {
        self.viewport = gpu::Viewport {
            size: size.as_vec2().to_array(),
        };
        self.viewport_dirty = true;
    }

    /// Uploads queued sprite instances and issues the draw calls for the frame.
    pub fn submit(&mut self, context: &RenderContext, frame: &mut Frame, primitives: &Primitives) {
        if self.viewport_dirty {
            context.queue.write_buffer(
                &self.viewport_buffer,
                0,
                bytemuck::bytes_of(&self.viewport),
            );
        }

        struct ResolvedPrimitive {
            texture: Handle<Texture>,
            instance: gpu::RectInstance,
        }

        let resolved_primitives: Vec<_> = primitives
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::Texture {
                    rect,
                    texture,
                    alpha,
                } => {
                    if !self.ensure_bind_group(*texture) {
                        return None;
                    }

                    Some(ResolvedPrimitive {
                        texture: *texture,
                        instance: gpu::RectInstance {
                            pos: rect.pos.as_vec2().to_array(),
                            size: rect.size.as_vec2().to_array(),
                            alpha: *alpha,
                            uv_min: Vec2::ZERO.to_array(),
                            uv_max: Vec2::ONE.to_array(),
                        },
                    })
                }
                Primitive::Sprite {
                    rect,
                    sprite,
                    frame,
                } => {
                    let (image, alpha, top_left, bottom_right) = {
                        let sprite_data = self.sprites.get(*sprite)?;
                        let sprite_frame = sprite_data.frame(*frame)?;
                        (
                            sprite_data.image,
                            sprite_data.alpha.unwrap_or(1.0),
                            sprite_frame.top_left,
                            sprite_frame.bottom_right,
                        )
                    };

                    let texture = self.textures.create_from_image(image)?;
                    if !self.ensure_bind_group(texture) {
                        return None;
                    }

                    let texture_data = self.textures.get(texture)?;
                    let texture_size = texture_data.size.as_vec2();
                    let uv_min = top_left.as_vec2() / texture_size;
                    let uv_max = bottom_right.as_vec2() / texture_size;

                    Some(ResolvedPrimitive {
                        texture,
                        instance: gpu::RectInstance {
                            pos: rect.pos.as_vec2().to_array(),
                            size: rect.size.as_vec2().to_array(),
                            alpha,
                            uv_min: uv_min.to_array(),
                            uv_max: uv_max.to_array(),
                        },
                    })
                }
            })
            .collect();

        let instances: Vec<_> = resolved_primitives
            .iter()
            .map(|primitive| gpu::RectInstance {
                pos: primitive.instance.pos,
                size: primitive.instance.size,
                alpha: primitive.instance.alpha,
                uv_min: primitive.instance.uv_min,
                uv_max: primitive.instance.uv_max,
            })
            .collect();

        self.instances_buffer.write(context, &instances);

        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("window_context"),
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
        render_pass.set_vertex_buffer(1, self.instances_buffer.slice(..));
        render_pass.set_index_buffer(self.indices_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_bind_group(0, &self.viewport_bind_group, &[]);

        for (index, primitive) in resolved_primitives.iter().enumerate() {
            let Some(bind_group) = self.bind_groups.get(&primitive.texture) else {
                continue;
            };

            render_pass.set_bind_group(1, bind_group, &[]);
            render_pass.draw_indexed(
                0..self.indices_buffer.count,
                0,
                index as u32..index as u32 + 1,
            );
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

    #[derive(Clone, Copy, NoUninit)]
    #[repr(C)]
    pub struct RectInstance {
        pub pos: [f32; 2],
        pub size: [f32; 2],
        pub alpha: f32,
        pub uv_min: [f32; 2],
        pub uv_max: [f32; 2],
    }
}
