use glam::UVec2;

use super::mipmaps::MipMaps;

pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    mipmaps: MipMaps,
}

impl Renderer {
    const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        let mipmaps = MipMaps::new(&device, Self::TEXTURE_FORMAT);

        Self {
            device,
            queue,
            mipmaps,
        }
    }

    pub fn create_texture(&self, label: &str, image: &image::RgbaImage) -> wgpu::TextureView {
        let (width, height) = (image.width(), image.height());

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let mip_level_count = (width.max(height) as f32).log2().floor() as u32 + 1;

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::default(),
                aspect: wgpu::TextureAspect::All,
            },
            image,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            size,
        );

        self.mipmaps
            .generate_mipmaps(&self.device, &self.queue, &texture, mip_level_count);

        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub fn create_sampler(
        &self,
        label: &str,
        address_mode: wgpu::AddressMode,
        mag_filter: wgpu::FilterMode,
        min_filter: wgpu::FilterMode,
    ) -> wgpu::Sampler {
        self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(label),
            address_mode_u: address_mode,
            address_mode_v: address_mode,
            address_mode_w: address_mode,
            mag_filter,
            min_filter,
            ..Default::default()
        })
    }
}

/// A single object passed around during the rendering of a single frame.
pub struct Frame {
    /// The encoder to use for creating render passes.
    pub encoder: wgpu::CommandEncoder,

    /// The window surface.
    pub surface: wgpu::TextureView,

    /// The index of the frame being rendered.
    pub frame_index: u64,

    /// The size of the surface.
    pub size: UVec2,
}
