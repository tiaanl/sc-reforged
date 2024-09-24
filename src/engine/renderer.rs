use std::{cell::RefCell, sync::Arc};

pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: RefCell<wgpu::SurfaceConfiguration>,
}

impl Renderer {
    pub fn new(window: Arc<winit::window::Window>) -> Self {
        let winit::dpi::PhysicalSize { width, height } = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .expect("request adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::CLEAR_TEXTURE,
                ..Default::default()
            },
            None,
        ))
        .expect("request device");

        let surface = instance.create_surface(window).expect("create surface");

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

        surface.configure(&device, &surface_config);

        Self {
            device,
            queue,
            surface,
            surface_config: RefCell::new(surface_config),
        }
    }

    pub fn resize(&self, width: u32, height: u32) {
        let mut surface_config = self.surface_config.borrow_mut();
        surface_config.width = width;
        surface_config.height = height;

        self.surface.configure(&self.device, &surface_config);
    }
}
