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

/// A single UI vertex in logical UI coordinates.
#[derive(Clone, Copy, Debug, bytemuck::NoUninit)]
#[repr(C)]
pub struct UiVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl UiVertex {
    fn new(pos: Vec2, uv: Vec2, color: Vec4) -> Self {
        Self {
            pos: pos.to_array(),
            uv: uv.to_array(),
            color: color.to_array(),
        }
    }
}

/// A textured indexed UI mesh draw with an optional clip rect.
#[derive(Clone, Debug)]
pub struct UiMesh {
    pub vertices: Vec<UiVertex>,
    pub indices: Vec<u32>,
    pub texture: Handle<Texture>,
    pub clip_rect: Option<Rect>,
}

impl UiMesh {
    /// Creates a textured rectangle mesh.
    pub fn textured_rect(
        rect: Rect,
        texture: Handle<Texture>,
        uv_min: Vec2,
        uv_max: Vec2,
        color: Vec4,
    ) -> Self {
        let min = rect.position.as_vec2();
        let max = (rect.position + rect.size).as_vec2();

        Self {
            vertices: vec![
                UiVertex::new(min, uv_min, color),
                UiVertex::new(
                    Vec2::new(max.x, min.y),
                    Vec2::new(uv_max.x, uv_min.y),
                    color,
                ),
                UiVertex::new(max, uv_max, color),
                UiVertex::new(
                    Vec2::new(min.x, max.y),
                    Vec2::new(uv_min.x, uv_max.y),
                    color,
                ),
            ],
            indices: vec![0, 1, 2, 2, 3, 0],
            texture,
            clip_rect: None,
        }
    }

    #[must_use]
    pub fn with_clip_rect(mut self, clip_rect: Rect) -> Self {
        self.clip_rect = Some(clip_rect);
        self
    }
}

pub struct UiMeshRenderer {
    solid_white_texture: Handle<Texture>,
    render_pipeline: wgpu::RenderPipeline,
    vertices_buffer: GrowingBuffer<UiVertex>,
    indices_buffer: GrowingBuffer<u32>,
    new_size: Option<UVec2>,
    viewport_buffer: wgpu::Buffer,
    viewport_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    bind_groups: HashMap<Handle<Texture>, wgpu::BindGroup>,
}

impl UiMeshRenderer {
    /// Creates the mesh renderer and its GPU state for UI geometry.
    pub fn new(surface: &SurfaceDesc) -> Self {
        let Gpu { device, .. } = &globals::gpu();

        let viewport = gpu::Viewport::from(surface.size);

        let viewport_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ui_mesh_renderer_viewport_buffer"),
            contents: bytemuck::bytes_of(&viewport),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let viewport_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ui_mesh_renderer_viewport_bind_group_layout"),
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
            label: Some("ui_mesh_renderer_viewport_bind_group"),
            layout: &viewport_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: viewport_buffer.as_entire_binding(),
            }],
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ui_mesh_renderer_texture_bind_group_layout"),
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
            label: Some("ui_mesh_renderer_sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui_mesh_renderer_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "window.wgsl"
            ))),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ui_mesh_renderer_pipeline_layout"),
            bind_group_layouts: &[&viewport_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui_mesh_renderer_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: None,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<UiVertex>() as wgpu::BufferAddress,
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

        let vertices_buffer =
            GrowingBuffer::new(256, wgpu::BufferUsages::VERTEX, "ui_mesh_renderer_vertices");
        let indices_buffer =
            GrowingBuffer::new(384, wgpu::BufferUsages::INDEX, "ui_mesh_renderer_indices");

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
            solid_white_texture,
            render_pipeline,
            vertices_buffer,
            indices_buffer,
            new_size: None,
            viewport_buffer,
            viewport_bind_group,
            texture_bind_group_layout,
            sampler,
            bind_groups: HashMap::default(),
        }
    }

    /// Returns the generated white texture used for solid-color meshes.
    pub fn solid_white_texture(&self) -> Handle<Texture> {
        self.solid_white_texture
    }

    /// Queues a viewport resize to be applied the next time meshes are submitted.
    pub fn resize(&mut self, size: UVec2) {
        self.new_size = Some(size);
    }

    /// Uploads the provided meshes and renders them in order.
    pub fn submit(
        &mut self,
        render_context: &mut RenderContext,
        render_target: &RenderTarget,
        meshes: &[UiMesh],
    ) {
        if let Some(new_size) = self.new_size.take() {
            let viewport = gpu::Viewport::from(new_size);

            globals::gpu().queue.write_buffer(
                &self.viewport_buffer,
                0,
                bytemuck::bytes_of(&viewport),
            );
        }

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut draws = Vec::new();

        for mesh in meshes {
            if mesh.vertices.is_empty() || mesh.indices.is_empty() {
                continue;
            }

            if !self.ensure_bind_group(mesh.texture) {
                continue;
            }

            let first_vertex = vertices.len() as u32;
            let first_index = indices.len() as u32;

            vertices.extend_from_slice(&mesh.vertices);
            indices.extend(mesh.indices.iter().map(|index| index + first_vertex));

            draws.push(DrawCall {
                texture: mesh.texture,
                clip_rect: mesh.clip_rect,
                index_range: first_index..indices.len() as u32,
            });
        }

        self.vertices_buffer.write(&vertices);
        self.indices_buffer.write(&indices);

        let mut render_pass =
            render_context
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("ui_mesh_renderer_pass"),
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
        render_pass.set_index_buffer(self.indices_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_bind_group(0, &self.viewport_bind_group, &[]);

        for draw in draws.iter() {
            let Some(bind_group) = self.bind_groups.get(&draw.texture) else {
                continue;
            };

            if let Some(clip_rect) = draw.clip_rect {
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
            render_pass.draw_indexed(draw.index_range.clone(), 0, 0..1);
        }
    }

    /// Lazily creates the texture bind group for a mesh texture.
    fn ensure_bind_group(&mut self, texture_handle: Handle<Texture>) -> bool {
        if self.bind_groups.contains_key(&texture_handle) {
            return true;
        }

        let Some(texture) = globals::textures().get(texture_handle) else {
            return false;
        };

        let bind_group = globals::gpu()
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ui_mesh_renderer_texture_bind_group"),
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

struct DrawCall {
    texture: Handle<Texture>,
    clip_rect: Option<Rect>,
    index_range: std::ops::Range<u32>,
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
}
