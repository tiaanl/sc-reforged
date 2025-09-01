use std::sync::Arc;

use glam::UVec2;

use crate::global;

use super::mip_maps::MipMaps;

pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub surface: super::surface::Surface,

    mip_maps: MipMaps,
}

pub trait BufferLayout: Clone {
    fn layout() -> wgpu::VertexBufferLayout<'static>;
}

impl BufferLayout for () {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: 0,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[],
        }
    }
}

impl Renderer {
    const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

    pub fn new(window: Arc<winit::window::Window>) -> Self {
        let winit::dpi::PhysicalSize { width, height } = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window).expect("create surface");

        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            }))
            .expect("Could not request an adapter.");

        let supported = adapter.features();
        let required = wgpu::Features::MULTI_DRAW_INDIRECT
            | wgpu::Features::PUSH_CONSTANTS
            | wgpu::Features::TEXTURE_BINDING_ARRAY
            | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING;

        let surface_caps = surface.get_capabilities(&adapter);

        // Find a sRGB surface format or use the first.
        let format = surface_caps
            .formats
            .iter()
            .find(|cap| cap.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let mut surface_config = surface
            .get_default_config(&adapter, width, height)
            .expect("surface get default configuration");
        surface_config.format = format;
        // surface_config.present_mode = wgpu::PresentMode::AutoNoVsync;
        surface_config.present_mode = wgpu::PresentMode::AutoVsync;

        let surface = super::surface::Surface::new(surface, surface_config);

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: required & supported,
                required_limits: wgpu::Limits {
                    max_bind_groups: 6,
                    max_color_attachment_bytes_per_sample: 56,
                    max_push_constant_size: 16,
                    ..Default::default()
                },
                ..Default::default()
            },
            None,
        ))
        .expect("request device");

        surface.configure(&device);

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture_bind_group_layout"),
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

        let mip_maps = MipMaps::new(&device, &texture_bind_group_layout, Self::TEXTURE_FORMAT);

        Self {
            device: device.clone(),
            queue: queue.clone(),
            surface,
            mip_maps,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let size = UVec2::new(width, height);
        self.surface.resize(&self.device, size);
    }

    pub fn create_texture_view(&self, label: &str, image: &image::RgbaImage) -> wgpu::TextureView {
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

        self.mip_maps
            .generate_mip_maps(&self.device, &self.queue, &texture, mip_level_count);

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

global!(Renderer, scoped_renderer, renderer);

pub use global::ScopedGlobal as ScopedRendererGlobal;

/// A single object passed around during the rendering of a single frame.
pub struct Frame {
    /// The encoder to use for creating render passes.
    pub encoder: wgpu::CommandEncoder,

    /// The window surface.
    pub surface: wgpu::TextureView,
}
