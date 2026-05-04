#import camera_env::{CameraEnv, diffuse_with_fog};

@group(0) @binding(0)
var<uniform> u_camera_env: CameraEnv;

struct TerrainData {
    cells_dim: vec2<u32>,
    chunks_dim: vec2<u32>,
    cell_size: f32,
    strata_descent: f32,
}

@group(1) @binding(0) var<uniform> u_terrain_data: TerrainData;
@group(1) @binding(1) var<storage, read> u_height_map: array<vec4<f32>>;
@group(1) @binding(2) var u_terrain_texture: texture_2d<f32>;
@group(1) @binding(3) var u_strata_texture: texture_2d<f32>;
@group(1) @binding(4) var u_terrain_sampler: sampler;

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
    @location(3) flags: u32,
}

fn get_node(coord: vec2<u32>) -> vec4<f32> {
    let clamped = clamp(coord, vec2<u32>(0, 0), u_terrain_data.cells_dim - vec2<u32>(1, 1));
    let index = clamped.y * u_terrain_data.cells_dim.x + clamped.x;
    return u_height_map[index];
}

const CELLS_PER_CHUNK: u32 = 8u;

const NORTH: u32 = 0u;
const EAST: u32 = 1u;
const SOUTH: u32 = 2u;
const WEST: u32 = 3u;

const NORTH_FLAG: u32 = 1u << NORTH;
const EAST_FLAG: u32 = 1u << EAST;
const SOUTH_FLAG: u32 = 1u << SOUTH;
const WEST_FLAG: u32 = 1u << WEST;

const HIGHLIGHT_FLAG: u32 = (1u << 15u);

fn get_stitched_node(
    chunk_coord: vec2<u32>,
    node_coord: vec2<u32>,
    abs_node_coord: vec2<u32>,
    chunk: InstanceInput,
) -> vec4<f32> {
    var normal_and_height = get_node(abs_node_coord);

    let last = CELLS_PER_CHUNK >> chunk.lod;

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

fn make_vertex_terrain(
    chunk: InstanceInput,
    vertex_index: u32,
    z_offset: f32,
) -> VertexOutput {
    let node_coord = vec2<u32>(vertex_index % 9u, vertex_index / 9u);

    let abs_node_coord = chunk.coord * 8u + vec2<u32>(
        node_coord.x << chunk.lod,
        node_coord.y << chunk.lod,
    );

    let node = get_stitched_node(chunk.coord, node_coord, abs_node_coord, chunk);

    let world_position = vec3<f32>(
        f32(abs_node_coord.x) * u_terrain_data.cell_size,
        f32(abs_node_coord.y) * u_terrain_data.cell_size,
        node.w + z_offset,
    );

    let clip_position = u_camera_env.proj_view * vec4<f32>(world_position, 1.0);

    let tex_coord = vec2<f32>(
        f32(abs_node_coord.x) / f32(u_terrain_data.cells_dim.x),
        f32(abs_node_coord.y) / f32(u_terrain_data.cells_dim.y),
    );

    return VertexOutput(
        clip_position,
        world_position,
        node.xyz,
        tex_coord,
        chunk.flags,
    );
}

@vertex
fn vertex_terrain(
    @builtin(vertex_index) vertex_index: u32,
    chunk: InstanceInput,
) -> VertexOutput {
    return make_vertex_terrain(chunk, vertex_index, 0.0);
}

@fragment
fn fragment_terrain(vertex: VertexOutput) -> geometry_buffer::OpaqueGeometryBuffer {
    let base_color = textureSample(u_terrain_texture, u_terrain_sampler, vertex.tex_coord);
    let distance = length(vertex.world_position - u_camera_env.position.xyz);

    let d = diffuse_with_fog(
        u_camera_env,
        vertex.normal,
        base_color.rgb,
        distance,
        1.0,
    );

    if (vertex.flags & HIGHLIGHT_FLAG) == HIGHLIGHT_FLAG {
        let c = mix(d, vec3<f32>(0.0, 1.0, 0.0), 0.1);
        return geometry_buffer::to_opaque_geometry_buffer(c);
    }

    return geometry_buffer::to_opaque_geometry_buffer(d);
}

const STRATA_SIDE_SHIFT: u32 = 8u;
const STRATA_SIDE_MASK:  u32 = 3u;

fn get_strata_side(flags: u32) -> u32 {
    return (flags >> STRATA_SIDE_SHIFT) & STRATA_SIDE_MASK;
}

fn strata_normal_from_side(side: u32) -> vec3<f32> {
    switch side {
        case SOUTH: { return vec3<f32>( 0.0, -1.0,  0.0); }
        case WEST:  { return vec3<f32>( 1.0,  0.0,  0.0); }
        case NORTH: { return vec3<f32>( 0.0,  1.0,  0.0); }
        default:    { return vec3<f32>(-1.0,  0.0,  0.0); } // EAST
    }
}

fn strata_node_coord_from_side(side: u32, u: u32, cells: u32) -> vec2<u32> {
    // u runs 0..cells inclusive (i.e. nodes per chunk - 1 at this LOD).
    // Matches the original CPU ordering:
    // South: [x, 0]
    // West:  [CELLS, y]
    // North: [CELLS - x, CELLS]
    // East:  [0, CELLS - y]
    switch side {
        case SOUTH: { return vec2<u32>(u, 0u); }
        case WEST:  { return vec2<u32>(cells, u); }
        case NORTH: { return vec2<u32>(cells - u, cells); }
        default:    { return vec2<u32>(0u, cells - u); } // EAST
    }
}

@vertex
fn strata_vertex(
    chunk: InstanceInput,
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    let side: u32 = get_strata_side(chunk.flags);

    // Each node along the edge is emitted twice: bottom then top (based on parity).
    // U is in LOD-space (0..cells at this LOD).
    let u: u32 = vertex_index >> 1u;
    let is_top: bool = (vertex_index & 1u) != 0u;

    let cells_lod: u32 = CELLS_PER_CHUNK >> chunk.lod;
    let node_coord_lod: vec2<u32> = strata_node_coord_from_side(side, u, cells_lod);
    let normal: vec3<f32> = strata_normal_from_side(side);

    let scale = 1u << chunk.lod;
    let abs_node_coord = chunk.coord * 8u + (node_coord_lod * scale);

    let node = get_stitched_node(
        chunk.coord,
        node_coord_lod,
        abs_node_coord,
        chunk,
    );

    let top_z = node.w;
    let bottom_z = u_terrain_data.strata_descent;
    let z = select(bottom_z, top_z, is_top);

    let cell_size = u_terrain_data.cell_size;
    let world_position = vec3<f32>(f32(abs_node_coord.x) * cell_size, f32(abs_node_coord.y) * cell_size, z);

    let clip_position = u_camera_env.proj_view * vec4<f32>(world_position, 1.0);

    // U runs along the edge: X for south/north, Y for west/east.
    let edge_index_u: u32 = select(
        node_coord_lod.x * scale,
        node_coord_lod.y * scale,
        (side & 1u) != 0u,
    );

    // Flip U on south(2) and west(3) so texture flows consistently clockwise.
    let flip_u = (side == SOUTH) || (side == WEST);

    let cells_per_chunk: u32 = u_terrain_data.cells_dim.x / u_terrain_data.chunks_dim.x;
    let edge_index_u_flipped: u32 =
        select(edge_index_u, (cells_per_chunk - edge_index_u), flip_u);

    // Normalize to [0,1] across the edge length.
    let tex_coord = vec2<f32>(
        f32(edge_index_u_flipped) / f32(cells_per_chunk),
        z / (f32(cells_per_chunk) * cell_size),
    );

    return VertexOutput(
        clip_position,
        world_position,
        normal,
        tex_coord,
        chunk.flags,
    );
}

@fragment
fn strata_fragment(vertex: VertexOutput) -> geometry_buffer::OpaqueGeometryBuffer {
    let base_color = textureSample(u_strata_texture, u_terrain_sampler, vertex.tex_coord);
    let distance = length(vertex.world_position - u_camera_env.position.xyz);

    let d = diffuse_with_fog(
        u_camera_env,
        vertex.normal,
        base_color.rgb,
        distance,
        1.0,
    );

    return geometry_buffer::to_opaque_geometry_buffer(d);
}

@vertex
fn vertex_wireframe(
    @builtin(vertex_index) vertex_index: u32,
    chunk: InstanceInput,
) -> VertexOutput {
    return make_vertex_terrain(chunk, vertex_index, 1.0);
}

@fragment
fn fragment_wireframe(_vertex: VertexOutput) -> geometry_buffer::OpaqueGeometryBuffer {
    // Bright green lines, no fog applied so they are clearer.
    let line_color = vec3<f32>(0.0, 1.0, 0.1);
    return geometry_buffer::to_opaque_geometry_buffer(line_color);
}
