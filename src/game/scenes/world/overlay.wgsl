#import world::camera

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

    let color = vec4(textureLoad(t_color, tex_coord, 0).xyz, 1.0);
    let world_position = textureLoad(t_position, tex_coord, 0).xyz;

    let light_position = u_environment.sun_proj_view * vec4(world_position, 1.0);
    if (light_position.w <= 0.0) {
        return color;
    }

    let eps = 1e-5;

    // Inside test in NDC: x,y in [-1,1], z in [0,1] for WebGPU
    let inside_xy = all(light_position.xy >= vec2<f32>(-1.0 - eps)) &&
                    all(light_position.xy <= vec2<f32>( 1.0 + eps));
    let inside_z = (light_position.z >= 0.0 - eps) && (light_position.z <= 1.0 + eps);

    if (inside_xy && inside_z) {
        return mix(color, vec4(1.0, 0.0, 0.0, 1.0), 0.5);
    }

    return color;
}
