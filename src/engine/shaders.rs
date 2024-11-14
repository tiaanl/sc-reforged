use std::collections::HashMap;

use naga_oil::compose::{
    ComposableModuleDescriptor, Composer, NagaModuleDescriptor, ShaderLanguage,
};

use super::renderer::Renderer;

#[derive(Default)]
pub struct Shaders {
    composer: Composer,
}

impl Shaders {
    pub fn add_module(&mut self, source: &str, path: &str) {
        self.composer
            .add_composable_module(ComposableModuleDescriptor {
                source,
                file_path: path,
                language: ShaderLanguage::Wgsl,
                as_name: None,
                additional_imports: &[],
                shader_defs: HashMap::default(),
            })
            .expect("Could not add module.");
    }

    pub fn create_shader(
        &mut self,
        renderer: &Renderer,
        label: &str,
        source: &str,
        path: &str,
    ) -> wgpu::ShaderModule {
        let module = match self.composer.make_naga_module(NagaModuleDescriptor {
            source,
            file_path: path,
            shader_type: naga_oil::compose::ShaderType::Wgsl,
            shader_defs: HashMap::default(),
            additional_imports: &[],
        }) {
            Ok(module) => module,
            Err(err) => {
                panic!("Could not create shader module. {}", err);
            }
        };

        renderer
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Naga(std::borrow::Cow::Owned(module)),
            })
    }
}
