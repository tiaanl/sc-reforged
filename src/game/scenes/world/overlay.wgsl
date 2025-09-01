#import math::position_in_frustum;
#import shadows::Cascades;
#import world::camera::Camera;

@group(0) @binding(0) var<uniform> u_camera: Camera;

@group(1) @binding(0) var<uniform> u_cascades: Cascades;

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

    const COLORS = array<vec4<f32>, 4>(
        vec4<f32>(1.0, 0.0, 0.0, 1.0),
        vec4<f32>(0.0, 1.0, 0.0, 1.0),
        vec4<f32>(0.0, 0.0, 1.0, 1.0),
        vec4<f32>(1.0, 1.0, 0.0, 1.0),
    );

    for (var cascade_index = 0u; cascade_index < u_cascades.count; cascade_index += 1) {
        if position_in_frustum(u_cascades.cascades[cascade_index], world_position) {
            return mix(color, COLORS[cascade_index], 0.7);
        }
    }

    return color;
}
