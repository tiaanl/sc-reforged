#import world::camera

@group(0) @binding(0) var<uniform> u_camera: camera::Camera;

struct Transforms {
    transforms: array<mat4x4<f32>>,
};
@group(1) @binding(0) var<storage, read> u_transforms: Transforms;

@group(2) @binding(0) var t_terrain_texture: texture_2d<f32>;
@group(2) @binding(1) var s_terrain_texture: sampler;

struct VertexInput {
    @builtin(instance_index) node_id: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
}

@vertex
fn vertex_main(vertex: VertexInput) -> VertexOutput {
    let node_id = vertex.node_id;
    let model = u_transforms.transforms[node_id];
    return VertexOutput(
        u_camera.mat_projection * u_camera.mat_view * model * vec4(vertex.position, 1.0),
        vertex.normal,
        vertex.tex_coord,
    );
}

@fragment
fn fragment_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let sun_dir = normalize(vec3(0.0, 1.0, 0.0));
    let c = dot(sun_dir, vertex.normal) * 0.5 + 0.5;

    let color = textureSample(t_terrain_texture, s_terrain_texture, vertex.tex_coord);
    // return color * vec4(c, c, c, 1.0);

    return color;
}
