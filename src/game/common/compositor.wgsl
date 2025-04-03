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

fn get_frag(texture: texture_2d<f32>, uv: vec2<f32>) -> vec4<f32> {
    let texture_size = vec2<f32>(textureDimensions(texture));
    let frag_coords = vec2<i32>(uv * texture_size);

    return textureLoad(texture, frag_coords, 0);
}

@fragment
fn fragment_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    // let fog_color = vec4<f32>(0.3, 0.3, 0.3, 1.0);
    // let fog_density = 0.66;

    let albedo = get_frag(t_albedo, vertex.uv);
    let position = get_frag(t_position, vertex.uv);
    let normal = get_frag(t_normal, vertex.uv);

    // let distance_from_camera = position.w;
    // let fog_start = 0.0;
    // let fog_end = 13300.0;
    // let fog_factor = clamp((distance_from_camera - fog_start) / (fog_end - fog_start), 0.0, 1.0);
    // let final_color = mix(albedo, fog_color, fog_factor);
    // return final_color;

    return albedo;
}
