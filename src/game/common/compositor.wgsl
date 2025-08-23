#import world::camera
#import world::environment

@group(0) @binding(0) var t_color: texture_2d<f32>;
@group(0) @binding(1) var t_oit_accumulation: texture_2d<f32>;
@group(0) @binding(2) var t_oit_revealage: texture_2d<f32>;
@group(0) @binding(3) var t_position: texture_2d<f32>;
@group(1) @binding(0) var<uniform> u_camera: camera::Camera;
@group(2) @binding(0) var<uniform> u_environment: environment::Environment;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let clip_position = fullscreen::clip_position(vertex_index);
    return VertexOutput(clip_position);
}

fn get_frag(texture: texture_2d<f32>, uv: vec2<f32>) -> vec4<f32> {
    let texture_size = vec2<f32>(textureDimensions(texture));
    let frag_coords = vec2<i32>(uv * texture_size);

    return textureLoad(texture, frag_coords, 0);
}

@fragment
fn fragment_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let dimensions = textureDimensions(t_color);
    let tex_coord = fullscreen::tex_coord(dimensions, vertex.position.xy);

    let color = textureLoad(t_color, tex_coord, 0);
    let accumulation = textureLoad(t_oit_accumulation, tex_coord, 0);
    let revealage = textureLoad(t_oit_revealage, tex_coord, 0);

    let weight = max(accumulation.a, 1e-6);
    let average = accumulation.rgb / weight;
    let transmittance = clamp(revealage.r, 0.0, 1.0);

    let rgb = average * (1.0 - transmittance);
    let out = rgb + color.rgb * transmittance;

    return vec4<f32>(out, 1.0);
}
