struct CameraEnv {
    proj_view: mat4x4<f32>,
    frustum: array<vec4<f32>, 6>,
    position: vec4<f32>,
    forward: vec4<f32>,

    sun_dir: vec4<f32>,   // x, y, z, 0
    sun_color: vec4<f32>, // r, g, b, 1
    fog_color: vec4<f32>, // r, g, b, 1
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
    @location(1) tex_coord: vec2<f32>,
}

fn get_node(coord: vec2<u32>) -> vec4<f32> {
    let clamped = clamp(coord, vec2<u32>(0, 0), u_terrain_data.cells_dim - vec2<u32>(1, 1));
    let index = clamped.y * u_terrain_data.cells_dim.x + clamped.x;
    return u_height_map[index];
}

/*
fn get_stitched_node(
    chunk_lod: u32,
    chunk_coord: vec2<u32>,
    node_coord: vec2<u32>,
) -> vec4<f32> {
    let abs_node_coord = chunk_coord + (node_coord << chunk_lod);

    var normal_and_height = get_node(abs_node_coord);

    let last = terrain::CELLS_PER_CHUNK >> chunk_lod;

    // If last is one, the amount of cells in this chunk is 1, so no stitching is required.
    if last == 1u {
       return normal_and_height;
    }

    let scale = 1u << chunk_lod;
    let chunks_size = u_terrain_data.cells_dim / terrain::CELLS_PER_CHUNK;

    // Check if neighbors are valid.
    let has_neg_x = chunk_coord.x > 0u;
    let has_pos_x = (chunk_coord.x + 1u) < chunks_size.x;
    let has_neg_y = chunk_coord.y > 0u;
    let has_pos_y = (chunk_coord.y + 1u) < chunks_size.y;

    // -X
    if node_coord.x == 0u && has_neg_x {
        let nidx = (chunk_coord.y * chunks_size.x) + (chunk_coord.x - 1u);
        if u_height_map[nidx].lod > chunk_lod && (node_coord.y & 1u) == 1u {
            let a = get_node_normal_and_height(abs_node_coord - vec2<u32>(0u, scale));
            let b = get_node_normal_and_height(abs_node_coord + vec2<u32>(0u, scale));
            normal_and_height = vec4<f32>(normalize(a.xyz + b.xyz), 0.5 * (a.w + b.w));
        }
    }

    // +X
    if node_coord.x == last && has_pos_x {
        let nidx = (chunk_coord.y * chunks_size.x) + (chunk_coord.x + 1u);
        if u_height_map[nidx].lod > chunk_lod && (node_coord.y & 1u) == 1u {
            let a = get_node_normal_and_height(abs_node_coord - vec2<u32>(0u, scale));
            let b = get_node_normal_and_height(abs_node_coord + vec2<u32>(0u, scale));
            normal_and_height = vec4<f32>(normalize(a.xyz + b.xyz), 0.5 * (a.w + b.w));
        }
    }

    // -Y
    if node_coord.y == 0u && has_neg_y {
        let nidx = ((chunk_coord.y - 1u) * chunks_size.x) + chunk_coord.x;
        if u_height_map[nidx].lod > chunk_lod && (node_coord.x & 1u) == 1u {
            let a = get_node_normal_and_height(abs_node_coord - vec2<u32>(scale, 0u));
            let b = get_node_normal_and_height(abs_node_coord + vec2<u32>(scale, 0u));
            normal_and_height = vec4<f32>(normalize(a.xyz + b.xyz), 0.5 * (a.w + b.w));
        }
    }

    // +Y
    if node_coord.y == last && has_pos_y {
        let nidx = ((chunk_coord.y + 1u) * chunks_size.x) + chunk_coord.x;
        if u_height_map[nidx].lod > chunk_lod && (node_coord.x & 1u) == 1u {
            let a = get_node_normal_and_height(abs_node_coord - vec2<u32>(scale, 0u));
            let b = get_node_normal_and_height(abs_node_coord + vec2<u32>(scale, 0u));
            normal_and_height = vec4<f32>(normalize(a.xyz + b.xyz), 0.5 * (a.w + b.w));
        }
    }

    return normal_and_height;
}
*/

@vertex
fn vertex_terrain(
    @builtin(vertex_index) vertex_index: u32,
    chunk: InstanceInput,
) -> VertexOutput {
    let chunk_coord = chunk.coord;

    let node_coord = vec2<u32>(
        (vertex_index % 9) << chunk.lod,
        (vertex_index / 9) << chunk.lod,
    );

    let abs_node_coord = chunk_coord * 8 + node_coord;

    let node = get_node(abs_node_coord);
    // let node = get_stitched_node(chunk.lod, chunk_coord, node_coord);

    let world_position = vec3<f32>(
        f32(abs_node_coord.x) * u_terrain_data.cell_size,
        f32(abs_node_coord.y) * u_terrain_data.cell_size,
        node.w,
    );

    let clip_position = u_camera_env.proj_view * vec4<f32>(world_position, 1.0);

    let tex_coord = vec2<f32>(
        f32(abs_node_coord.x) / f32(u_terrain_data.cells_dim.x),
        f32(abs_node_coord.y) / f32(u_terrain_data.cells_dim.y),
    );

    return VertexOutput(
        clip_position,
        world_position,
        tex_coord,
    );
}

@fragment
fn fragment_terrain(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let base_color = textureSample(u_terrain_texture, u_terrain_sampler, vertex.tex_coord);

    let distance = length(vertex.world_position - u_camera_env.position.xyz);

    let d = diffuse_with_fog(
        u_camera_env,
        vec3<f32>(0.0, 0.0, 1.0),
        base_color.rgb,
        distance,
        1.0,
    );

    return vec4<f32>(d, 1.0);
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
    let ambient = vec3<f32>(env.sun_dir.w, env.sun_color.w, env.fog_color.w);
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

    let fog_factor = linear_fog_factor(
        env.fog_near_fraction, // near
        env.fog_distance, // far
        distance,
    );

    return mix(lit_color, env.fog_color.rgb, fog_factor);
}

fn linear_fog_factor(fog_near: f32, fog_far: f32, distance: f32) -> f32 {
    return clamp((distance - fog_near) / (fog_far - fog_near), 0.0, 1.0);
}
