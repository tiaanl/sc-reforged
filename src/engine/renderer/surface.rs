use glam::UVec2;

pub struct Surface {
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
}

impl Surface {
    pub fn new(
        surface: wgpu::Surface<'static>,
        surface_config: wgpu::SurfaceConfiguration,
    ) -> Self {
        Self {
            surface,
            surface_config,
        }
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.surface_config.format
    }

    pub fn size(&self) -> UVec2 {
        UVec2::new(self.surface_config.width, self.surface_config.height)
    }

    pub fn configure(&self, device: &wgpu::Device) {
        self.surface.configure(device, &self.surface_config);
    }

    pub fn resize(&mut self, device: &super::render::RenderDevice, size: glam::UVec2) {
        self.surface_config.width = size.x;
        self.surface_config.height = size.y;
        self.configure(device);
    }

    pub fn get_texture(&self) -> wgpu::SurfaceTexture {
        self.surface
            .get_current_texture()
            .expect("Could not get current texture in swap chain.")
    }
}
