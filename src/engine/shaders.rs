use std::collections::HashMap;

use naga_oil::compose::{
    ComposableModuleDescriptor, Composer, NagaModuleDescriptor, ShaderDefValue, ShaderLanguage,
};
use wgpu::naga::valid::Capabilities;

use crate::engine::renderer::renderer;

pub struct Shaders {
    composer: Composer,
}

impl Shaders {
    pub fn new() -> Self {
        let composer = Composer::default().with_capabilities(
            Capabilities::PUSH_CONSTANT
                | Capabilities::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
        );
        Self { composer }
    }

    pub fn add_module(&mut self, source: &str, file_path: &str) {
        self.composer
            .add_composable_module(ComposableModuleDescriptor {
                source,
                file_path,
                language: ShaderLanguage::Wgsl,
                as_name: None,
                additional_imports: &[],
                shader_defs: HashMap::default(),
            })
            .expect("Could not add module.");
    }

    pub fn create_shader(
        &mut self,
        label: &str,
        source: &str,
        file_path: &str,
        shader_defs: HashMap<String, ShaderDefValue>,
    ) -> wgpu::ShaderModule {
        let module = match self.composer.make_naga_module(NagaModuleDescriptor {
            source,
            file_path,
            shader_type: naga_oil::compose::ShaderType::Wgsl,
            shader_defs,
            additional_imports: &[],
        }) {
            Ok(module) => module,
            Err(err) => {
                let msg = err.emit_to_string(&self.composer);
                panic!("Could not create shader module. {msg}");
            }
        };

        renderer()
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Naga(std::borrow::Cow::Owned(module)),
            })
    }
}
