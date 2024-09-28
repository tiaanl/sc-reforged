struct Camera {
    mat_projection: mat4x4<f32>,
    mat_view: mat4x4<f32>,
}
@group(0) @binding(0) var<uniform> u_camera: Camera;

struct Model {
    mat_model: mat4x4<f32>,
}
@group(1) @binding(0) var<uniform> u_model: Model;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) camera_direction: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
}

@vertex
fn vertex_main(vertex: VertexInput) -> VertexOutput {
    let view_forward = vec4(0.0, 0.0, -1.0, 0.0);
    let inverse = transpose(u_camera.mat_view); // No scaling, so just transpose.
    let camera_direction = normalize(inverse * view_forward);

    return VertexOutput(
        u_camera.mat_projection * u_camera.mat_view * u_model.mat_model * vec4(vertex.position, 1.0),
        camera_direction.xyz,
        vertex.tex_coord,
    );
}

fn sd_circle(p: vec2<f32>, r: f32) -> f32 {
    return length(p) - r;
}

@fragment
fn fragment_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let coord = 1.0 - (2.0 * vertex.tex_coord);
    let distance = sd_circle(coord, 0.3);
    if distance >= 0.2 {
        return vec4(1.0, 0.0, 0.0, 1.0);
    } else {
        return vec4(0.0, 1.0, 0.0, 1.0);
    }
}
