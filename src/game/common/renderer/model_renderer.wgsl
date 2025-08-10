#import world::camera
#import world::environment
#import world::geometry_buffers

@group(0) @binding(0) var<uniform> u_camera: camera::Camera;

@group(1) @binding(0) var<uniform> u_environment: environment::Environment;

@group(2) @binding(0) var t_texture: texture_2d<f32>;
@group(2) @binding(1) var s_sampler: sampler;

@group(3) @binding(0) var t_positions: texture_2d<f32>;
@group(3) @binding(1) var t_rotations: texture_2d<f32>;

struct VertexInput {
    @location(0)
    position: vec3<f32>,

    @location(1)
    normal: vec3<f32>,

    @location(2)
    tex_coord: vec2<f32>,

    @location(3)
    node_index: u32,
}

struct InstanceInput {
    @location(4)
    col0: vec4<f32>,

    @location(5)
    col1: vec4<f32>,

    @location(6)
    col2: vec4<f32>,

    @location(7)
    col3: vec4<f32>,

    @location(8)
    entity_id: u32,

    @location(9)
    time: f32,
}

struct VertexOutput {
    @builtin(position)
    position: vec4<f32>,

    @location(0)
    tex_coord: vec2<f32>,

    @location(1)
    world_position: vec3<f32>,

    @location(2)
    normal: vec3<f32>,

    @location(3)
    entity_id: u32,
}

fn quat_to_mat3(q: vec4<f32>) -> mat3x3<f32> {
    let x2 = q.x + q.x;
    let y2 = q.y + q.y;
    let z2 = q.z + q.z;

    let xx2 = q.x * x2;
    let yy2 = q.y * y2;
    let zz2 = q.z * z2;
    let xy2 = q.x * y2;
    let xz2 = q.x * z2;
    let yz2 = q.y * z2;
    let wx2 = q.w * x2;
    let wy2 = q.w * y2;
    let wz2 = q.w * z2;

    return mat3x3<f32>(
        vec3<f32>(1.0 - (yy2 + zz2), xy2 + wz2,        xz2 - wy2),
        vec3<f32>(xy2 - wz2,        1.0 - (xx2 + zz2), yz2 + wx2),
        vec3<f32>(xz2 + wy2,        yz2 - wx2,        1.0 - (xx2 + yy2)),
    );
}

fn transform_from_pos_rot(position: vec3<f32>, rotation: vec4<f32>) -> mat4x4<f32> {
    let rot = quat_to_mat3(rotation);
    return mat4x4<f32>(
        vec4<f32>(rot[0], 0.0),
        vec4<f32>(rot[1], 0.0),
        vec4<f32>(rot[2], 0.0),
        vec4<f32>(position, 1.0),
    );
}

@vertex
fn vertex(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model_mat = mat4x4<f32>(
        instance.col0,
        instance.col1,
        instance.col2,
        instance.col3,
    );

    let entity_id = instance.entity_id;

    // let transform = model_mat * u_nodes[vertex.node_index].transform;

    const FPS: f32 = 30.0;
    const looping: u32 = 1;
    let frame_count = max(textureDimensions(t_positions, 0).x, 1u);
    let anim_time = instance.time;

    var before_frame: u32;
    var after_frame: u32;
    var t: f32;

    {
        let last_frame = frame_count - 1;

        // Fractional frame position.
        var frac = anim_time * FPS;

        // Wrap or clamp into [0, frame_count).
        if (looping > 0u && frame_count > 1u) {
            frac = frac - floor(frac / f32(frame_count)) * f32(frame_count);
        } else {
            frac = clamp(frac, 0.0, f32(last_frame));
        }

        let before = u32(floor(frac));
        var after: u32;
        if (looping > 0u && frame_count > 1u) {
            after = (before + 1) % frame_count;
        } else {
            after = min(before + 1, last_frame);
        }

        let denom = max(1.0, f32(after) - f32(before));
        t = clamp((frac - f32(before)) / denom, 0.0, 1.0);

        before_frame = before;
        after_frame = after;
    }

    let anim_tex_coord_before = vec2<i32>(i32(before_frame), i32(vertex.node_index));
    let before_position = textureLoad(t_positions, anim_tex_coord_before, 0);
    let before_rotation = textureLoad(t_rotations, anim_tex_coord_before, 0);

    let anim_tex_coord_after = vec2<i32>(i32(after_frame), i32(vertex.node_index));
    let after_position = textureLoad(t_positions, anim_tex_coord_after, 0);
    let after_rotation = textureLoad(t_rotations, anim_tex_coord_after, 0);

    let position = mix(before_position, after_position, t);
    let rotation = mix(before_rotation, after_rotation, t);

    let node_mat = transform_from_pos_rot(position.xyz, rotation);

    let transform = model_mat * node_mat;

    let world_position = transform * (vec4<f32>(vertex.position, 1.0));

    let normal_mat = mat3x3<f32>(
        transform[0].xyz,
        transform[1].xyz,
        transform[2].xyz,
    );
    let world_normal = normalize(normal_mat * vertex.normal);

    let clip_position = u_camera.mat_projection * u_camera.mat_view * world_position;

    return VertexOutput(
        clip_position,
        vertex.tex_coord,
        world_position.xyz,
        world_normal,
        entity_id,
    );
}

@fragment
fn fragment_opaque(vertex: VertexOutput) -> geometry_buffers::GeometryBuffers {
    let base_color = textureSample(t_texture, s_sampler, vertex.tex_coord);

    if base_color.a < 1e-4 {
        discard;
    }

    let world_position = vertex.world_position;
    let world_normal = vertex.normal;

    let distance = length(world_position - u_camera.position);

    let diffuse = environment::diffuse_with_fog(
        u_environment,
        world_normal.xyz,
        base_color.rgb,
        distance,
    );

    return geometry_buffers::to_geometry_buffer(
        vec4<f32>(diffuse, 1.0),
        world_position,
        vertex.entity_id,
    );
}

@fragment
fn fragment_alpha(vertex: VertexOutput) -> geometry_buffers::GeometryBuffers {
    let base_color = textureSample(t_texture, s_sampler, vertex.tex_coord);

    let world_position = vertex.world_position;
    let world_normal = vertex.normal;

    let distance = length(world_position - u_camera.position);

    let diffuse = environment::diffuse_with_fog(
        u_environment,
        world_normal.xyz,
        base_color.rgb,
        distance,
    );

    return geometry_buffers::to_geometry_buffer(
        vec4<f32>(diffuse, base_color.a),
        world_position,
        vertex.entity_id,
    );
}
