use std::path::{Path, PathBuf};

use naga::{
    Module,
    back::wgsl,
    valid::{Capabilities, ValidationFlags, Validator},
};
use naga_oil::compose::{
    ComposableModuleDescriptor, Composer, NagaModuleDescriptor, ShaderLanguage, ShaderType,
};

fn main() {
    compile_shaders();
}

const COMMON: &[&str] = &[
    // First
    "src/game/common/renderer/math.wgsl",
    // Rest
    "src/game/common/camera.wgsl",
    "src/game/common/fullscreen.wgsl",
    "src/game/common/geometry_buffers.wgsl",
    "src/game/common/shadows.wgsl",
    "src/game/common/renderer/animation.wgsl",
    "src/game/scenes/world/environment.wgsl",
    "src/game/scenes/world/terrain_data.wgsl",
    "src/game/scenes/world/frustum.wgsl",
];

const SHADERS: &[&str] = &[
    "src/game/common/compositor.wgsl",
    "src/game/common/renderer/model_renderer.wgsl",
    "src/game/common/renderer/model_renderer_shadows.wgsl",
    "src/game/scenes/world/overlay.wgsl",
    "src/game/scenes/world/terrain.wgsl",
    "src/engine/gizmos.wgsl",
    "src/game/scenes/world/strata.wgsl",
    "src/game/scenes/world/process_chunks.wgsl",
];

fn compile_shaders() {
    COMMON
        .iter()
        .chain(SHADERS)
        .for_each(|path| println!("cargo:rerun-if-changed={path}"));

    let mut composer = Composer::default().with_capabilities(
        Capabilities::PUSH_CONSTANT
            | Capabilities::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
    );

    for path in COMMON {
        add_support_shader(&mut composer, path);
    }

    let shaders_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("shaders");
    std::fs::create_dir_all(&shaders_dir).expect("Create OUT_DIR/shaders");

    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());

    for path in SHADERS {
        let module = create_shader_module(&mut composer, path);

        let info = validator
            .validate(&module)
            .unwrap_or_else(|e| panic!("Validation failed for {path}:\n{e:#?}"));

        let out_path = shaders_dir.join(
            PathBuf::from(path)
                .with_extension("wgsl")
                .file_name()
                .unwrap(),
        );

        let wgsl_text = wgsl::write_string(&module, &info, wgsl::WriterFlags::EXPLICIT_TYPES)
            .expect("WGSL write failed");

        std::fs::write(&out_path, wgsl_text).expect("write .wgsl");
    }
}

fn add_support_shader(composer: &mut Composer, path: impl AsRef<Path>) {
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("Could not read shader source {}", path.as_ref().display()));

    let result = composer.add_composable_module(ComposableModuleDescriptor {
        source: &source,
        file_path: &path.as_ref().display().to_string(),
        language: ShaderLanguage::Wgsl,
        as_name: None,
        ..Default::default()
    });

    if let Err(e) = result {
        let msg = e.emit_to_string(composer);
        panic!(
            "Could not register module {}: {msg}",
            path.as_ref().display()
        );
    }
}

fn create_shader_module(composer: &mut Composer, path: impl AsRef<Path>) -> Module {
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("Could not read shader source {}", path.as_ref().display()));

    match composer.make_naga_module(NagaModuleDescriptor {
        source: &source,
        file_path: &path.as_ref().display().to_string(),
        shader_type: ShaderType::Wgsl,
        ..Default::default()
    }) {
        Ok(m) => m,
        Err(err) => {
            let msg = err.emit_to_string(composer);
            panic!(
                "Could not compose shader {}:\n{msg}",
                path.as_ref().display()
            );
        }
    }
}
