use std::path::{Path, PathBuf};

use heck::ToUpperCamelCase;
use naga::{
    Module,
    back::wgsl,
    valid::{Capabilities, ValidationFlags, Validator},
};
use naga_oil::compose::{
    ComposableModuleDescriptor, Composer, NagaModuleDescriptor, ShaderLanguage, ShaderType,
};
use quote::{format_ident, quote};

fn main() {
    compile_shaders();
}

const COMMON: &[&str] = &[
    "src/game/common/fullscreen.wgsl",
    "src/game/scenes/world/render/shaders/camera_env.wgsl",
    "src/game/scenes/world/render/shaders/geometry_buffer.wgsl",
];

const SHADERS: &[&str] = &[
    "src/engine/gizmos.wgsl",
    "src/game/scenes/world/render/shaders/compositor.wgsl",
    "src/game/scenes/world/render/shaders/models.wgsl",
    "src/game/scenes/world/render/shaders/terrain.wgsl",
    "src/game/scenes/world/render/shaders/ui.wgsl",
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

    write_shaders_module();
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

fn write_shaders_module() {
    println!("cargo:rerun-if-changed=build.rs");

    let variants: Vec<_> = SHADERS
        .iter()
        .map(|path| {
            let stem = Path::new(path)
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_upper_camel_case();
            format_ident!("{stem}")
        })
        .collect();

    let all_items: Vec<_> = SHADERS
        .iter()
        .map(|path| {
            let stem = Path::new(path)
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_upper_camel_case();
            let v = format_ident!("{stem}");
            quote!(ShaderSource::#v)
        })
        .collect();

    let source_arms: Vec<_> = SHADERS
        .iter()
        .map(|path| {
            println!("cargo:rerun-if-changed={path}");

            let stem = Path::new(path)
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_upper_camel_case();
            let variant = format_ident!("{stem}");

            // Must match how you write to OUT_DIR/shaders
            let file_name = Path::new(path)
                .with_extension("wgsl")
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();

            let rel = format!("/shaders/{file_name}");

            quote! {
                ShaderSource::#variant => include_str!(concat!(env!("OUT_DIR"), #rel)),
            }
        })
        .collect();

    let label_arms: Vec<_> = SHADERS
        .iter()
        .map(|path| {
            let stem = Path::new(path)
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_upper_camel_case();
            let variant = format_ident!("{stem}");

            let label = stem;

            quote! {
                ShaderSource::#variant => #label,
            }
        })
        .collect();

    let tokens = quote! {
        #[allow(dead_code)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum ShaderSource {
            #( #variants, )*
        }

        impl ShaderSource {
            pub const ALL: &'static [ShaderSource] = &[
                #( #all_items, )*
            ];
        }

        #[allow(dead_code)]
        pub fn shader_label(source: ShaderSource) -> &'static str {
            match source {
                #( #label_arms )*
            }
        }

        #[allow(dead_code)]
        pub fn shader_source(source: ShaderSource) -> &'static str {
            match source {
                #( #source_arms )*
            }
        }
    };

    let out_file =
        std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("shader_source.rs");

    let file: syn::File = syn::parse2(tokens).unwrap();
    let out = prettyplease::unparse(&file);

    std::fs::write(out_file, out).unwrap();
}
