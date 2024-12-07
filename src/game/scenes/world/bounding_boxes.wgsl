struct Camera {
    mat_projection: mat4x4<f32>,
    mat_view: mat4x4<f32>,
}
@group(0) @binding(0) var<uniform> u_camera: Camera;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
}

struct InstanceInput {
    @location(10) model_mat_0: vec4<f32>,
    @location(11) model_mat_1: vec4<f32>,
    @location(12) model_mat_2: vec4<f32>,
    @location(13) model_mat_3: vec4<f32>,
    @location(14) min: vec3<f32>,
    @location(15) highlight: u32,
    @location(16) max: vec3<f32>,
    @location(17) _padding: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) highlight: u32,
}

@vertex
fn vertex_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    let size = instance.max.xyz - instance.min.xyz;
    let center = instance.min.xyz + size / 2.0;

    let model_mat = mat4x4<f32>(
        instance.model_mat_0,
        instance.model_mat_1,
        instance.model_mat_2,
        instance.model_mat_3,
    );

    let mvp = u_camera.mat_projection * u_camera.mat_view * model_mat;

    return VertexOutput(
        mvp * vec4((vertex.position * size) + center, 1.0),
        vertex.normal,
        vertex.tex_coord.xy,
        instance.highlight,
    );
}

@fragment
fn fragment_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    if vertex.highlight == 1 {
        return vec4(1.0, 0.5, 0.5, 0.2);
    } else {
        return vec4(0.5, 1.0, 0.5, 0.2);
    }
}

@fragment
fn fragment_main_wireframe(vertex: VertexOutput) -> @location(0) vec4<f32> {
    if vertex.highlight == 1 {
        return vec4(0.5, 0.0, 0.0, 1.0);
    } else {
        return vec4(0.0, 0.5, 0.0, 1.0);
    }
}
