use std::sync::Arc;

use glam::UVec2;
use winit::window::{Window, WindowId};

use super::{Gpu, RenderTarget};

#[derive(Clone, Copy, Debug)]
pub struct SurfaceDesc {
    pub size: UVec2,
    pub format: wgpu::TextureFormat,
    /// Logical-to-physical pixel ratio reported by the OS (1.0 on most
    /// displays, 2.0 on macOS Retina). Used by the UI layer to map between
    /// physical events / framebuffer and logical (DPI-independent) UI coords.
    pub scale_factor: f32,
}

/// The window we present to, along with everything needed to rebuild its
/// surface if the platform takes it away from us.
pub struct RenderWindow {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
}

impl RenderWindow {
    pub fn new(window: Arc<Window>) -> (Self, Gpu) {
        let instance = wgpu::Instance::default();

        let surface = instance
            .create_surface(Arc::clone(&window))
            .expect("create surface");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .expect("Could not request an adapter.");

        let supported = adapter.features();
        let required = wgpu::Features::MULTI_DRAW_INDIRECT_COUNT
            | wgpu::Features::POLYGON_MODE_LINE
            | wgpu::Features::TEXTURE_BINDING_ARRAY
            | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING;

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_features: required & supported,
            required_limits: wgpu::Limits {
                max_binding_array_elements_per_shader_stage: 1024,
                max_bind_groups: 6,
                max_color_attachment_bytes_per_sample: 56,
                ..Default::default()
            },
            ..Default::default()
        }))
        .expect("request device");

        let surface_config = Self::config_for(&surface, &adapter, window.inner_size());

        let render_window = Self {
            instance,
            adapter,
            window,
            surface,
            surface_config,
        };
        render_window.configure(&device);

        (render_window, Gpu::new(device, queue))
    }

    fn config_for(
        surface: &wgpu::Surface,
        adapter: &wgpu::Adapter,
        size: winit::dpi::PhysicalSize<u32>,
    ) -> wgpu::SurfaceConfiguration {
        let mut surface_config = surface
            .get_default_config(adapter, size.width, size.height)
            .expect("surface get default configuration");

        // Prefer an sRGB format so the hardware does the color space conversion.
        let capabilities = surface.get_capabilities(adapter);
        surface_config.format = capabilities
            .formats
            .iter()
            .find(|format| format.is_srgb())
            .copied()
            .unwrap_or(capabilities.formats[0]);

        // surface_config.present_mode = wgpu::PresentMode::AutoNoVsync;
        surface_config.present_mode = wgpu::PresentMode::AutoVsync;

        surface_config
    }

    pub fn has_id(&self, window_id: WindowId) -> bool {
        self.window.id() == window_id
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn desc(&self) -> SurfaceDesc {
        SurfaceDesc {
            size: UVec2::new(self.surface_config.width, self.surface_config.height),
            format: self.surface_config.format,
            scale_factor: self.window.scale_factor() as f32,
        }
    }

    pub fn configure(&self, device: &wgpu::Device) {
        self.surface.configure(device, &self.surface_config);
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: UVec2) {
        self.surface_config.width = size.x;
        self.surface_config.height = size.y;
        self.configure(device);
    }

    /// Acquire the frame to render into. [None] means the frame should be  skipped; the next redraw
    /// will try again.
    pub fn next_frame(&mut self, device: &wgpu::Device) -> Option<Frame> {
        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => {
                Some(Frame::new(surface_texture))
            }

            // Still usable, so render with it and reconfigure for the next frame.
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                self.configure(device);
                Some(Frame::new(surface_texture))
            }

            wgpu::CurrentSurfaceTexture::Timeout => {
                tracing::trace!("Timed out acquiring a surface texture, skipping frame.");
                None
            }

            wgpu::CurrentSurfaceTexture::Occluded => None,

            wgpu::CurrentSurfaceTexture::Outdated => {
                tracing::debug!("Surface is outdated, reconfiguring it.");
                self.configure(device);
                None
            }

            wgpu::CurrentSurfaceTexture::Lost => {
                tracing::warn!("Surface was lost, recreating it.");
                self.recreate(device);
                None
            }

            // Already reported to the device's error handler, so only skip the frame.
            wgpu::CurrentSurfaceTexture::Validation => {
                tracing::error!("Validation error while acquiring a surface texture.");
                None
            }
        }
    }

    fn recreate(&mut self, device: &wgpu::Device) {
        match self.instance.create_surface(Arc::clone(&self.window)) {
            Ok(surface) => {
                // The new surface can support a different set of formats, so
                // negotiate the configuration again instead of reusing ours.
                self.surface_config =
                    Self::config_for(&surface, &self.adapter, self.window.inner_size());
                self.surface = surface;
                self.configure(device);
            }
            Err(err) => tracing::error!("Could not recreate the lost surface - {err}"),
        }
    }
}

/// A frame acquired from the [RenderWindow], holding the surface texture until it is presented.
pub struct Frame {
    surface_texture: wgpu::SurfaceTexture,
    pub target: RenderTarget,
}

impl Frame {
    fn new(surface_texture: wgpu::SurfaceTexture) -> Self {
        let target = RenderTarget {
            size: UVec2::new(
                surface_texture.texture.width(),
                surface_texture.texture.height(),
            ),
            view: surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default()),
        };

        Self {
            surface_texture,
            target,
        }
    }

    /// Present the frame, after the work rendering into it was submitted.
    pub fn present(self, queue: &wgpu::Queue) {
        queue.present(self.surface_texture);
    }
}
