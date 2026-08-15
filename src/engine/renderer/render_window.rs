use std::sync::Arc;

use glam::UVec2;
use wgpu::CurrentSurfaceTexture;
use winit::window::{Window, WindowId};

use super::{Gpu, RenderTarget};

/// Properties of the surface used by renderers that draw into the window.
#[derive(Clone, Copy, Debug)]
pub struct SurfaceDesc {
    /// Physical size of the surface in pixels.
    pub size: UVec2,
    /// Texture format used by the surface.
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
    /// Creates a renderable window surface and its associated GPU context.
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

        let supported_features = adapter.features();
        let desired_features = wgpu::Features::MULTI_DRAW_INDIRECT_COUNT
            | wgpu::Features::POLYGON_MODE_LINE
            | wgpu::Features::TEXTURE_BINDING_ARRAY
            | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING;

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_features: desired_features & supported_features,
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

    /// Chooses a supported surface configuration for the given window size.
    fn config_for(
        surface: &wgpu::Surface,
        adapter: &wgpu::Adapter,
        size: winit::dpi::PhysicalSize<u32>,
    ) -> wgpu::SurfaceConfiguration {
        let mut surface_config = surface
            .get_default_config(adapter, size.width.max(1), size.height.max(1))
            .expect("surface get default configuration");

        // Prefer an sRGB format so the hardware does the color space conversion.
        let capabilities = surface.get_capabilities(adapter);
        surface_config.format = capabilities
            .formats
            .iter()
            .find(|format| format.is_srgb())
            .copied()
            .unwrap_or(capabilities.formats[0]);

        surface_config.present_mode = wgpu::PresentMode::AutoVsync;

        surface_config
    }

    /// Returns whether this render window owns the window with `window_id`.
    pub fn has_id(&self, window_id: WindowId) -> bool {
        self.window.id() == window_id
    }

    /// Notifies the platform that the next submitted frame will be presented.
    pub fn pre_present_notify(&self) {
        self.window.pre_present_notify();
    }

    /// Requests that the event loop redraw this window.
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    /// Returns the current surface properties.
    pub fn desc(&self) -> SurfaceDesc {
        SurfaceDesc {
            size: UVec2::new(self.surface_config.width, self.surface_config.height),
            format: self.surface_config.format,
            scale_factor: self.window.scale_factor() as f32,
        }
    }

    /// Configures the surface with its current properties.
    fn configure(&self, device: &wgpu::Device) {
        self.surface.configure(device, &self.surface_config);
    }

    /// Resizes and reconfigures the surface.
    pub fn resize(&mut self, device: &wgpu::Device, size: UVec2) {
        self.surface_config.width = size.x.max(1);
        self.surface_config.height = size.y.max(1);
        self.configure(device);
    }

    /// Acquires the next frame to render into.
    ///
    /// Returns `None` when this frame should be skipped. The next redraw will
    /// attempt acquisition again.
    pub fn next_frame(&mut self, device: &wgpu::Device) -> Option<Frame> {
        match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(surface_texture) => Some(Frame::new(surface_texture)),

            // Release the old swap-chain texture before reconfiguring the
            // surface, then acquire a replacement.
            CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                drop(surface_texture);
                self.reconfigure_and_retry(device)
            }

            CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => None,

            CurrentSurfaceTexture::Outdated => self.reconfigure_and_retry(device),

            CurrentSurfaceTexture::Lost => {
                if self.recreate(device) {
                    self.acquire_after_recovery()
                } else {
                    None
                }
            }

            // Already reported to the device's error handler, so only skip the frame.
            CurrentSurfaceTexture::Validation => None,
        }
    }

    /// Reconfigures the existing surface and retries acquisition once.
    fn reconfigure_and_retry(&self, device: &wgpu::Device) -> Option<Frame> {
        self.configure(device);
        self.acquire_after_recovery()
    }

    /// Acquires one texture after surface recovery without recursively trying another recovery
    /// cycle.
    fn acquire_after_recovery(&self) -> Option<Frame> {
        match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(surface_texture)
            | CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                Some(Frame::new(surface_texture))
            }
            error => {
                tracing::warn!(
                    ?error,
                    "Could not acquire a surface texture after recovery."
                );
                None
            }
        }
    }

    /// Recreates and configures a lost surface, returning whether it succeeded.
    fn recreate(&mut self, device: &wgpu::Device) -> bool {
        match self.instance.create_surface(Arc::clone(&self.window)) {
            Ok(surface) => {
                // The new surface can support a different set of formats, so
                // negotiate the configuration again instead of reusing ours.
                self.surface_config =
                    Self::config_for(&surface, &self.adapter, self.window.inner_size());
                self.surface = surface;
                self.configure(device);
                true
            }
            Err(err) => {
                tracing::error!("Could not recreate the lost surface - {err}");
                false
            }
        }
    }
}

/// A frame acquired from the [RenderWindow], holding the surface texture until it is presented.
pub struct Frame {
    surface_texture: wgpu::SurfaceTexture,
    /// Render target backed by the acquired surface texture.
    pub target: RenderTarget,
}

impl Frame {
    /// Wraps an acquired surface texture as a renderable frame.
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
