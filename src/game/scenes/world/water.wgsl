#import world::camera
#import world::fog

@group(0) @binding(0) var t_water: texture_2d<f32>;
@group(0) @binding(1) var s_water: sampler;

@group(1) @binding(0) var<uniform> u_camera: camera::Camera;
@group(2) @binding(0) var<uniform> u_fog: fog::Fog;

@group(3) @binding(0) var t_depth: texture_depth_2d;
@group(3) @binding(1) var s_depth: sampler;

struct Water {
    start: f32,
    end: f32,
    alpha: f32,
}

@group(4) @binding(0) var<uniform> u_water: Water;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) world_position: vec3<f32>,
}

@vertex
fn vertex_main(vertex: VertexInput) -> VertexOutput {
    let world_position = vertex.position;
    let proj_view = u_camera.mat_projection * u_camera.mat_view;
    let clip_position = proj_view * vec4(world_position, 1.0);

    return VertexOutput(
        clip_position,
        vertex.normal,
        vertex.tex_coord,
        world_position,
    );
}

@fragment
fn fragment_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let depth = textureLoad(t_depth, vec2<i32>(vertex.clip_position.xy), 0);
    let water_depth = vertex.clip_position.z;

    if water_depth > depth {
        discard;
    }

    let tex_color = textureSample(t_water, s_water, vertex.tex_coord);

    let fog_factor = fog::fog_factor(u_fog, vertex.world_position, u_camera.position.xyz);
    let final_color = mix(tex_color, vec4(u_fog.color, 1.0), fog_factor);

    let diff = abs(water_depth - depth) / vertex.clip_position.w;
    let fade = smoothstep(u_water.start, u_water.end, diff) * u_water.alpha;

    return vec4(final_color.xyz, fade);
}
