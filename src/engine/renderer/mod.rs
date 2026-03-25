mod frame;
mod mipmaps;
// mod render;
mod render_context;
mod surface;

use std::sync::Arc;

use winit::window::Window;

pub use frame::Frame;
pub use render_context::RenderContext;
pub use surface::{Surface, SurfaceDesc};

pub fn create(window: Arc<Window>) -> (surface::Surface, RenderContext) {
    let winit::dpi::PhysicalSize { width, height } = window.inner_size();

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });

    let surface = instance.create_surface(window).expect("create surface");

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptionsBase {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
    }))
    .expect("Could not request an adapter.");

    let supported = adapter.features();
    let required = wgpu::Features::MULTI_DRAW_INDIRECT_COUNT
        | wgpu::Features::POLYGON_MODE_LINE
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

    let surface = surface::Surface::new(surface, surface_config);

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

    surface.configure(&device);

    let context = RenderContext::new(device, queue);

    (surface, context)
}
