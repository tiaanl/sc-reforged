use glam::UVec2;

pub struct Surface {
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
}

pub struct SurfaceDesc {
    pub size: UVec2,
    pub format: wgpu::TextureFormat,
    /// Logical-to-physical pixel ratio reported by the OS (1.0 on most
    /// displays, 2.0 on macOS Retina). Used by the UI layer to map between
    /// physical events / framebuffer and logical (DPI-independent) UI coords.
    pub scale_factor: f32,
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

    pub fn get_texture(&self, device: &wgpu::Device) -> Option<wgpu::SurfaceTexture> {
        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => Some(surface_texture),
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                self.configure(device);
                Some(surface_texture)
            }
            wgpu::CurrentSurfaceTexture::Timeout => todo!(),
            wgpu::CurrentSurfaceTexture::Occluded => todo!(),
            wgpu::CurrentSurfaceTexture::Outdated => todo!(),
            wgpu::CurrentSurfaceTexture::Lost => todo!(),
            wgpu::CurrentSurfaceTexture::Validation => todo!(),
        }
    }
}
