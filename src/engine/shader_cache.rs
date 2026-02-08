use ahash::HashMap;

pub use shader_source::ShaderSource;

use shader_source::{shader_label, shader_source};

#[derive(Clone, Default)]
pub struct ShaderCache {
    modules: HashMap<ShaderSource, wgpu::ShaderModule>,
}

impl ShaderCache {
    pub fn get_or_create(
        &mut self,
        device: &wgpu::Device,
        source: ShaderSource,
    ) -> &wgpu::ShaderModule {
        self.modules.entry(source).or_insert_with_key(|source| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(shader_label(*source)),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_source(
                    *source,
                ))),
            })
        })
    }

    /// Optional: precompile everything so first-frame doesn't hitch.
    pub fn preload_all(&mut self, device: &wgpu::Device) {
        for &src in ShaderSource::ALL {
            let _ = self.get_or_create(device, src);
        }
    }
}

mod shader_source {
    include!(concat!(env!("OUT_DIR"), "/shader_source.rs"));
}
