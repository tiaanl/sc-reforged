#import world::camera
#import world::environment

@group(1) @binding(0) var<uniform> u_camera: camera::Camera;
@group(2) @binding(0) var<uniform> u_environment: environment::Environment;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let uv = vec2<f32>(f32(vertex_index >> 1u), f32(vertex_index & 1u)) * 2.0;
    let clip_position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);

    return VertexOutput(clip_position, uv);
}

@group(0) @binding(0) var t_albedo: texture_2d<f32>;
@group(0) @binding(1) var t_position: texture_2d<f32>;
@group(0) @binding(2) var t_normal: texture_2d<f32>;
@group(0) @binding(3) var t_alpha_accumulation: texture_2d<f32>;
@group(0) @binding(4) var t_alpha_revealage: texture_2d<f32>;
@group(0) @binding(5) var t_ids: texture_2d<u32>;

fn get_frag(texture: texture_2d<f32>, uv: vec2<f32>) -> vec4<f32> {
    let texture_size = vec2<f32>(textureDimensions(texture));
    let frag_coords = vec2<i32>(uv * texture_size);

    return textureLoad(texture, frag_coords, 0);
}

fn get_id(texture: texture_2d<u32>, uv: vec2<f32>) -> u32 {
    let texture_size = vec2<f32>(textureDimensions(texture));
    let frag_coords = vec2<i32>(uv * texture_size);
    return textureLoad(texture, frag_coords, 0).x;
}

@fragment
fn fragment_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let id = get_id(t_ids, vertex.uv);

    if (id == 0xFFFFFFFF) {
        return vec4<f32>(u_environment.fog_color.xyz, 1.0);
    }

    let albedo = get_frag(t_albedo, vertex.uv);
    let position = get_frag(t_position, vertex.uv);
    let normal = get_frag(t_normal, vertex.uv);

    let distance = length(position.xyz - u_camera.position);

    let diffuse = environment::diffuse_with_fog(u_environment, normal.xyz, albedo.xyz, distance);

    return vec4<f32>(diffuse, 1.0);
}
