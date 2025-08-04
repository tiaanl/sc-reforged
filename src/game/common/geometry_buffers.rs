use glam::{UVec2, Vec3, Vec4};

use crate::engine::prelude::renderer;

pub struct Buffer {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub read_back_buffer: wgpu::Buffer,
}

impl Buffer {
    fn new(device: &wgpu::Device, label: &str, size: UVec2, format: wgpu::TextureFormat) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.x.max(1),
                height: size.y.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let read_back_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            texture,
            view,
            read_back_buffer,
        }
    }

    pub fn fetch(&self, encoder: &mut wgpu::CommandEncoder, pos: UVec2) {
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: pos.x,
                    y: pos.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.read_back_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn read<T>(&self, device: &wgpu::Device, f: impl FnOnce(&wgpu::BufferView) -> T) -> T {
        let buffer_slice = self.read_back_buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, |result| {
            if result.is_err() {
                eprintln!("Failed to map buffer for reading");
            }
        });

        // Wait or poll for the GPU to complete the copy
        device.poll(wgpu::Maintain::Wait);

        let data = buffer_slice.get_mapped_range();

        let res = f(&data);

        drop(data);
        self.read_back_buffer.unmap();

        res
    }
}

pub struct GeometryBuffers {
    pub depth: Buffer,
    pub color: Buffer,
    pub position: Buffer,
    pub id: Buffer,

    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

#[derive(Default)]
pub struct GeometryData {
    pub color: Vec4,
    pub position: Vec3,
    pub id: u32,
}

impl GeometryBuffers {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
    const COLORS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
    const POSITIONS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;
    const IDS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Uint;

    pub fn new() -> Self {
        let size = renderer().surface.size();

        tracing::info!("Creating geometry buffers ({}x{})", size.x, size.y);

        let depth = Buffer::new(&renderer().device, "g_depth", size, Self::DEPTH_FORMAT);

        let color = Buffer::new(
            &renderer().device,
            "g_buffer_colors",
            size,
            Self::COLORS_FORMAT,
        );

        let position = Buffer::new(
            &renderer().device,
            "g_buffer_positions",
            size,
            Self::POSITIONS_FORMAT,
        );

        let id = Buffer::new(&renderer().device, "g_buffer_ids", size, Self::IDS_FORMAT);

        let bind_group_layout =
            renderer()
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("g_buffer_bind_group_layout"),
                    entries: &[
                        // t_colors
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
                        // t_positions
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        // t_entity_ids
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Uint,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                    ],
                });

        let bind_group = renderer()
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("g_buffer_bind_group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&color.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&position.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&id.view),
                    },
                ],
            });

        Self {
            depth,
            color,
            position,
            id,

            bind_group_layout,
            bind_group,
        }
    }

    pub fn fetch_data(&self, pos: UVec2) -> GeometryData {
        let mut encoder =
            renderer()
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("geometry_buffers_pick"),
                });

        self.color.fetch(&mut encoder, pos);
        self.position.fetch(&mut encoder, pos);
        self.id.fetch(&mut encoder, pos);

        renderer().queue.submit(Some(encoder.finish()));

        // -----------------------------------------------------------------------------------------

        let color = self.color.read(&renderer().device, |data| {
            Vec4::new(
                data[0] as f32 / 255.0,
                data[1] as f32 / 255.0,
                data[2] as f32 / 255.0,
                data[3] as f32 / 255.0,
            )
        });

        let position = self.position.read(&renderer().device, |data| {
            let f: [f32; 4] = bytemuck::cast_slice(&data[0..16])[0..4].try_into().unwrap();
            Vec3::new(f[0], f[1], f[2])
        });

        let id = self.id.read(&renderer().device, |data| {
            u32::from_ne_bytes(data[0..4].try_into().unwrap())
        });

        GeometryData {
            color,
            position,
            id,
        }
    }

    pub fn color_attachments<'a>(&'a self) -> [Option<wgpu::RenderPassColorAttachment<'a>>; 3] {
        [
            Some(wgpu::RenderPassColorAttachment {
                view: &self.color.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &self.position.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &self.id.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            }),
        ]
    }

    pub fn opaque_targets() -> &'static [Option<wgpu::ColorTargetState>] {
        &[
            Some(wgpu::ColorTargetState {
                format: Self::COLORS_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: Self::POSITIONS_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: Self::IDS_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
        ]
    }

    pub fn alpha_targets() -> &'static [Option<wgpu::ColorTargetState>] {
        &[
            Some(wgpu::ColorTargetState {
                format: Self::COLORS_FORMAT,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: Self::POSITIONS_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: Self::IDS_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
        ]
    }

    fn create_texture(label: &str, size: UVec2, format: wgpu::TextureFormat) -> wgpu::Texture {
        renderer().device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }

    pub fn depth_stencil_state(
        depth_compare: wgpu::CompareFunction,
        depth_write_enabled: bool,
    ) -> wgpu::DepthStencilState {
        wgpu::DepthStencilState {
            format: Self::DEPTH_FORMAT,
            depth_write_enabled,
            depth_compare,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }
    }
}
