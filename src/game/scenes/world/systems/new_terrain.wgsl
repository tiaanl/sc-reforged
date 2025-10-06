struct CameraEnv {
    proj_view: mat4x4<f32>,
    frustum: array<vec4<f32>, 6>,
    position: vec4<f32>,
    forward: vec4<f32>,

    sun_dir: vec4<f32>,       // x, y, z, 0
    sun_color: vec4<f32>,     // r, g, b, 1
    ambient_color: vec4<f32>, // r, g, b, 1
    fog_color: vec4<f32>,     // r, g, b, 1
    fog_distance: f32,
    fog_near_fraction: f32,
}

@group(0) @binding(0) var<uniform> u_camera_env: CameraEnv;

struct TerrainData {
    cells_dim: vec2<u32>,
    chunks_dim: vec2<u32>,
    cell_size: f32,
}

@group(1) @binding(0) var<uniform> u_terrain_data: TerrainData;
@group(1) @binding(1) var<storage, read> u_height_map: array<vec4<f32>>;
@group(1) @binding(2) var u_terrain_texture: texture_2d<f32>;
@group(1) @binding(3) var u_terrain_sampler: sampler;

struct InstanceInput {
    @location(0) coord: vec2<u32>,
    @location(1) lod: u32,
    @location(2) flags: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
}

fn get_node(coord: vec2<u32>) -> vec4<f32> {
    let clamped = clamp(coord, vec2<u32>(0, 0), u_terrain_data.cells_dim - vec2<u32>(1, 1));
    let index = clamped.y * u_terrain_data.cells_dim.x + clamped.x;
    return u_height_map[index];
}

const NORTH_FLAG: u32 = (1u << 0u);
const EAST_FLAG: u32 = (1u << 1u);
const SOUTH_FLAG: u32 = (1u << 2u);
const WEST_FLAG: u32 = (1u << 3u);

fn get_stitched_node(
    chunk_coord: vec2<u32>,
    node_coord: vec2<u32>,
    abs_node_coord: vec2<u32>,
    chunk: InstanceInput,
) -> vec4<f32> {
    var normal_and_height = get_node(abs_node_coord);

    let last = terrain::CELLS_PER_CHUNK >> chunk.lod;

    // If last is one, the amount of cells in this chunk is 1, so no stitching is required.
    if last == 1u {
       return normal_and_height;
    }

    let scale = 1u << chunk.lod;

    let do_east = node_coord.x == 0u && (chunk.flags & EAST_FLAG) != 0u;
    let do_west = node_coord.x == last && (chunk.flags & WEST_FLAG) != 0u;
    let do_south = node_coord.y == 0u && (chunk.flags & SOUTH_FLAG) != 0u;
    let do_north = node_coord.y == last && (chunk.flags & NORTH_FLAG) != 0u;

    // -X / +X
    if (do_east || do_west) && (node_coord.y & 1u) != 0u {
        let a = get_node(abs_node_coord - vec2<u32>(0u, scale));
        let b = get_node(abs_node_coord + vec2<u32>(0u, scale));
        normal_and_height = vec4<f32>(normalize(a.xyz + b.xyz), 0.5 * (a.w + b.w));
    }

    // -Y / +Y
    if (do_south || do_north) && (node_coord.x & 1u) != 0u {
        let a = get_node(abs_node_coord - vec2<u32>(scale, 0u));
        let b = get_node(abs_node_coord + vec2<u32>(scale, 0u));
        normal_and_height = vec4<f32>(normalize(a.xyz + b.xyz), 0.5 * (a.w + b.w));
    }

    return normal_and_height;
}

@vertex
fn vertex_terrain(
    @builtin(vertex_index) vertex_index: u32,
    chunk: InstanceInput,
) -> VertexOutput {
    let node_coord = vec2<u32>(
        vertex_index % 9,
        vertex_index / 9,
    );

    let abs_node_coord = chunk.coord * 8 +
        vec2<u32>(node_coord.x << chunk.lod, node_coord.y << chunk.lod);

    let node = get_stitched_node(
        chunk.coord,
        node_coord,
        abs_node_coord,
        chunk,
    );

    let world_position = vec3<f32>(
        f32(abs_node_coord.x) * u_terrain_data.cell_size,
        f32(abs_node_coord.y) * u_terrain_data.cell_size,
        node.w, // Height from the height map.
    );

    let normal = node.xyz;  // Normal from the height map.

    let clip_position = u_camera_env.proj_view * vec4<f32>(world_position, 1.0);

    let tex_coord = vec2<f32>(
        f32(abs_node_coord.x) / f32(u_terrain_data.cells_dim.x + 1),
        f32(abs_node_coord.y) / f32(u_terrain_data.cells_dim.y + 1),
    );

    return VertexOutput(
        clip_position,
        world_position,
        normal,
        tex_coord,
    );
}

@fragment
fn fragment_terrain(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let base_color = textureSample(u_terrain_texture, u_terrain_sampler, vertex.tex_coord);

    let distance = length(vertex.world_position - u_camera_env.position.xyz);

    let d = diffuse_with_fog(
        u_camera_env,
        vertex.normal,
        base_color.rgb,
        distance,
        1.0,
    );

    return vec4<f32>(d, 1.0);
}

struct StrataVertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) node_coord: vec2<u32>,
}

struct StrataVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) node_coord: vec2<u32>,
}

@vertex
fn strata_vertex(input: StrataVertexInput, @builtin(vertex_index) vertex_index: u32) -> StrataVertexOutput {
    let node_coord = input.node_coord;

    let node = get_node(node_coord);

    var z = input.position.z;
    if (vertex_index & 1u) != 0 {
        z = node.w;
    }

    let world_position = vec3<f32>(
        input.position.x * 200.0,
        input.position.y * 200.0,
        z,
    );

    let clip_position = u_camera_env.proj_view * vec4<f32>(world_position, 1.0);

    return StrataVertexOutput(
        clip_position,
        input.normal,
        node_coord,
    );
}

@fragment
fn strata_fragment(vertex: StrataVertexOutput) -> @location(0) vec4<f32> {
    let lit = diffuse(u_camera_env, vertex.normal, vec3<f32>(1.0, 0.0, 0.0), 1.0);

    return vec4<f32>(lit, 1.0);
}

/// Diffuse + ambient lighting, modulated by shadow visibility.
fn diffuse(
    env: CameraEnv,
    normal: vec3<f32>,
    base_color: vec3<f32>,
    visibility: f32,              // 0 = full shadow, 1 = fully lit
) -> vec3<f32> {
    let N = normalize(normal);
    let L = -normalize(env.sun_dir.xyz); // from fragment toward sun

    let n_dot_l = max(dot(N, L), 0.0);

    // Direct sunlight (scaled by visibility)
    let sun_light = env.sun_color.rgb * n_dot_l * visibility;

    // Ambient term (not shadowed)
    let ambient = env.ambient_color.rgb;
    let ambient_color = env.sun_color.rgb * ambient;

    let lighting = sun_light + ambient_color;

    return lighting * base_color;
}

/// Same as diffuse_with_fog(), but blends in a shadow term.
fn diffuse_with_fog(
    env: CameraEnv,
    normal: vec3<f32>,
    base_color: vec3<f32>,
    distance: f32,
    visibility: f32,
) -> vec3<f32> {
    let lit_color = diffuse(env, normal, base_color, visibility);

    let fog_near = env.fog_distance * env.fog_near_fraction;
    let fog_far = env.fog_distance;

    let fog_factor = linear_fog_factor(fog_near, fog_far, distance);

    return mix(lit_color, env.fog_color.rgb, fog_factor);
}

fn linear_fog_factor(fog_near: f32, fog_far: f32, distance: f32) -> f32 {
    return clamp((distance - fog_near) / (fog_far - fog_near), 0.0, 1.0);
}
