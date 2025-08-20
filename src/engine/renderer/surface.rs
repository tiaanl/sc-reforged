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

    pub fn resize(&mut self, device: &wgpu::Device, size: glam::UVec2) {
        self.surface_config.width = size.x;
        self.surface_config.height = size.y;
        self.configure(device);
    }

    pub fn get_texture(&self, device: &wgpu::Device) -> wgpu::SurfaceTexture {
        match self.surface.get_current_texture() {
            Ok(texture) => texture,
            Err(wgpu::SurfaceError::Outdated) => {
                self.configure(device);
                self.get_texture(device)
            }
            Err(err) => panic!("Current texture not available! {err}"),
        }
    }
}
