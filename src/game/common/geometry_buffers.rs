use glam::{UVec2, Vec3};

pub struct RenderTarget {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl RenderTarget {
    fn new(device: &wgpu::Device, label: &str, size: UVec2, format: wgpu::TextureFormat) -> Self {
        let size = wgpu::Extent3d {
            width: size.x.max(1),
            height: size.y.max(1),
            depth_or_array_layers: 1,
        };

        let full_label = format!("render_target_texture_{label}");

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&full_label),
            size,
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

        Self { texture, view }
    }
}

pub struct ReadBackBuffer {
    buffer: wgpu::Buffer,
}

impl ReadBackBuffer {
    pub fn new(device: &wgpu::Device, label: &str) -> Self {
        let full_label = format!("read_back_buffer_{label}");

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&full_label),
            size: wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self { buffer }
    }

    /// Fetch the data from the given texture at the given position into this [ReadBackBuffer].
    pub fn fetch(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
        position: UVec2,
    ) {
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: position.x,
                    y: position.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.buffer,
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

    /// Read the data from this [ReadBackBuffer] for use on the CPU.
    pub fn read<T>(&self, device: &wgpu::Device, f: impl FnOnce(&wgpu::BufferView) -> T) -> T {
        let buffer_slice = self.buffer.slice(..);

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
        self.buffer.unmap();

        res
    }
}

pub struct GeometryBuffers {
    pub depth: RenderTarget,
    pub color: RenderTarget,
    pub position_id: RenderTarget,

    position_id_read_back_buffer: ReadBackBuffer,

    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

#[derive(Debug, Default)]
pub struct GeometryData {
    pub position: Vec3,
    pub id: u32,
}

impl GeometryBuffers {
    const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
    const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
    const POSITION_ID_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;

    pub fn new(device: &wgpu::Device, size: UVec2) -> Self {
        tracing::info!("Creating geometry buffers ({}x{})", size.x, size.y);

        let depth = RenderTarget::new(device, "depth", size, Self::DEPTH_FORMAT);
        let color = RenderTarget::new(device, "color", size, Self::COLOR_FORMAT);
        let position_id = RenderTarget::new(device, "position", size, Self::POSITION_ID_FORMAT);

        let position_id_read_back_buffer = ReadBackBuffer::new(device, "position_id");

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("g_buffer_bind_group_layout"),
            entries: &[
                // color
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
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("g_buffer_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&color.view),
            }],
        });

        Self {
            depth,
            color,
            position_id,

            position_id_read_back_buffer,

            bind_group_layout,
            bind_group,
        }
    }

    pub fn fetch_data(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        position: UVec2,
    ) -> GeometryData {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("geometry_buffer_fetch_data"),
        });

        self.position_id_read_back_buffer
            .fetch(&mut encoder, &self.position_id.texture, position);

        queue.submit(Some(encoder.finish()));

        let (position, id) = self.position_id_read_back_buffer.read(device, |data| {
            let f: [f32; 4] = bytemuck::cast_slice(&data[0..16])[0..4].try_into().unwrap();
            (
                Vec3::new(f[0], f[1], f[2]),
                u32::from_le_bytes(f[3].to_le_bytes()),
            )
        });

        GeometryData { position, id }
    }

    pub fn attachments<'a>(&'a self) -> [Option<wgpu::RenderPassColorAttachment<'a>>; 2] {
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
                view: &self.position_id.view,
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
                format: Self::COLOR_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: Self::POSITION_ID_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
        ]
    }

    pub fn alpha_targets() -> &'static [Option<wgpu::ColorTargetState>] {
        &[
            Some(wgpu::ColorTargetState {
                format: Self::COLOR_FORMAT,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: Self::POSITION_ID_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
        ]
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
