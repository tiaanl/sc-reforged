#import world::camera
#import world::environment

@group(0) @binding(0) var<uniform> u_camera: camera::Camera;
@group(1) @binding(0) var<uniform> u_environment: environment::Environment;
@group(2) @binding(0) var t_color: texture_2d<f32>;
@group(2) @binding(1) var t_oit_accumulation: texture_2d<f32>;
@group(2) @binding(2) var t_oit_revealage: texture_2d<f32>;
@group(2) @binding(3) var t_position: texture_2d<f32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let clip_position = fullscreen::clip_position(vertex_index);
    return VertexOutput(clip_position);
}

@fragment
fn fragment_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let dimensions = textureDimensions(t_position);
    let tex_coord = fullscreen::tex_coord(dimensions, vertex.position.xy);

    let position_at_fragment = textureLoad(t_position, tex_coord, 0).xyz;

    let distance_to_camera = length(position_at_fragment - u_camera.position);

    return vec4<f32>(vec3(distance_to_camera, distance_to_camera, distance_to_camera) / 20000.0, 1.0);
}
