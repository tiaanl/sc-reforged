#import world::camera::{
    camera_forward,
    Camera,
};
#import world::geometry_buffers::{
    OpaqueGeometryBuffers,
    to_opaque_geometry_buffer,
};

@binding(0) @group(0)
var<uniform> camera: Camera;

const LEVELS = array<f32, 3>(
    0.0,
    2500.0,
    7500.0,
);

const RADIUS = array<f32, 3>(
    25650.0,
    17100.0,
    0.0,
);

fn rotate2d(v: vec2<f32>, angle: f32) -> vec2<f32> {
    let s = sin(angle);
    let c = cos(angle);
    return vec2<f32>(v.x * c - v.y * s, v.x * s + v.y * c);
}

@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 0.0); // Discard.
}

@fragment
fn fragment() -> OpaqueGeometryBuffers {
    return to_opaque_geometry_buffer(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, 0.0, 0.0),
        0,
    );
}
