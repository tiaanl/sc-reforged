#[macro_export]
macro_rules! wgsl_shader {
    ($name:literal) => {{ wgpu::include_wgsl!(concat!(env!("OUT_DIR"), "/shaders/", $name, ".wgsl")) }};
}
