use glam::{UVec2, Vec3, Vec4};
use half::f16;

use crate::engine::renderer::Renderer;

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
    pub colors: Buffer,
    pub positions: Buffer,
    pub normals: Buffer,
    pub alpha_accumulation: Buffer,
    pub alpha_revealage: Buffer,
    pub ids: Buffer,

    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

#[derive(Default)]
pub struct GeometryData {
    pub color: Vec4,
    pub position: Vec3,
    pub normal: Vec3,
    pub id: u32,
}

impl GeometryBuffers {
    const COLORS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
    const POSITIONS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
    const NORMALS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
    const ALPHA_ACCUMULATION_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
    const ALPHA_REVEALAGE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;
    const IDS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Uint;

    pub fn new(renderer: &Renderer) -> Self {
        let size = UVec2::new(
            renderer.surface_config.width,
            renderer.surface_config.height,
        );

        tracing::info!("Creating geometry buffers ({}x{})", size.x, size.y);

        let colors = Buffer::new(
            &renderer.device,
            "g_buffer_colors",
            size,
            Self::COLORS_FORMAT,
        );

        let positions = Buffer::new(
            &renderer.device,
            "g_buffer_positions",
            size,
            Self::POSITIONS_FORMAT,
        );

        let normals = Buffer::new(
            &renderer.device,
            "g_buffer_normals",
            size,
            Self::NORMALS_FORMAT,
        );

        let alpha_accumulation = Buffer::new(
            &renderer.device,
            "g_buffer_alpha_accumulation",
            size,
            Self::ALPHA_ACCUMULATION_FORMAT,
        );

        let alpha_revealage = Buffer::new(
            &renderer.device,
            "g_buffer_alpha_revealabe",
            size,
            Self::ALPHA_REVEALAGE_FORMAT,
        );

        let ids = Buffer::new(&renderer.device, "g_buffer_ids", size, Self::IDS_FORMAT);

        let bind_group_layout =
            renderer
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
                        // t_normals
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        // t_alpha_accumulation
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        // t_alpha_revealage
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
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
                            binding: 5,
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

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("g_buffer_bind_group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&colors.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&positions.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&normals.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&alpha_accumulation.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&alpha_revealage.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(&ids.view),
                    },
                ],
            });

        Self {
            colors,
            positions,
            normals,
            alpha_accumulation,
            alpha_revealage,
            ids,

            bind_group_layout,
            bind_group,
        }
    }

    pub fn fetch_data(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pos: UVec2,
    ) -> GeometryData {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("geometry_buffers_pick"),
        });

        self.colors.fetch(&mut encoder, pos);
        self.positions.fetch(&mut encoder, pos);
        self.normals.fetch(&mut encoder, pos);
        self.ids.fetch(&mut encoder, pos);

        queue.submit(Some(encoder.finish()));

        // -----------------------------------------------------------------------------------------

        let color = self.colors.read(device, |data| {
            Vec4::new(
                data[0] as f32 / 255.0,
                data[1] as f32 / 255.0,
                data[2] as f32 / 255.0,
                data[3] as f32 / 255.0,
            )
        });

        let position = self.positions.read(device, |data| {
            let f: [f16; 4] = bytemuck::cast_slice(&data[0..8])[0..4].try_into().unwrap();
            Vec3::new(f[0].to_f32(), f[1].to_f32(), f[2].to_f32())
        });

        let normal = self.normals.read(device, |data| {
            let f: [f16; 4] = bytemuck::cast_slice(&data[0..8])[0..4].try_into().unwrap();
            Vec3::new(f[0].to_f32(), f[1].to_f32(), f[2].to_f32())
        });

        let id = self.ids.read(device, |data| {
            u32::from_ne_bytes(data[0..4].try_into().unwrap())
        });

        GeometryData {
            color,
            position,
            normal,
            id,
        }
    }

    pub fn color_attachments<'a>(&'a self) -> [Option<wgpu::RenderPassColorAttachment<'a>>; 6] {
        [
            Some(wgpu::RenderPassColorAttachment {
                view: &self.colors.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &self.positions.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &self.normals.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &self.alpha_accumulation.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &self.alpha_revealage.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &self.ids.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            }),
        ]
    }

    pub fn targets() -> &'static [Option<wgpu::ColorTargetState>] {
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
                format: Self::NORMALS_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: Self::ALPHA_ACCUMULATION_FORMAT,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: Self::ALPHA_REVEALAGE_FORMAT,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::Zero,
                        dst_factor: wgpu::BlendFactor::OneMinusSrc,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: Self::IDS_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
        ]
    }

    fn create_texture(
        renderer: &Renderer,
        label: &str,
        size: UVec2,
        format: wgpu::TextureFormat,
    ) -> wgpu::Texture {
        renderer.device.create_texture(&wgpu::TextureDescriptor {
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
}
