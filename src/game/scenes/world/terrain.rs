use std::{f32::consts::PI, path::PathBuf};

use glam::{IVec2, UVec2, Vec4};
use tracing::info;
use wgpu::util::DeviceExt;

use crate::{
    engine::{
        gizmos::{GizmoVertex, GizmosRenderer},
        prelude::*,
    },
    game::{
        config::CampaignDef, data_dir::data_dir, geometry_buffers::GeometryBuffers,
        height_map::HeightMap, image::images, math::BoundingSphere, shadows::ShadowCascades,
    },
    wgsl_shader,
};

use super::strata::Strata;

struct ChunkMesh {
    indices_buffer: wgpu::Buffer,
    wireframe_indices_buffer: wgpu::Buffer,
}

impl ChunkMesh {
    fn new(device: &wgpu::Device) -> Self {
        // level 0 = 0..384
        // level 1 = 0..96
        // level 2 = 0..24
        // level 3 = 0..6
        let mut indices = Vec::<u32>::with_capacity(384_usize + 96_usize + 24_usize + 6_usize);

        // level 0 = 0..512
        // level 1 = 0..128
        // level 2 = 0..32
        // level 3 = 0..8
        let mut wireframe_indices =
            Vec::<u32>::with_capacity(512_usize + 128_usize + 32_usize + 8_usize);

        for level in 0..=Terrain::LOD_MAX {
            let cell_count = Terrain::CELLS_PER_CHUNK >> level;
            let scale = 1 << level;

            for y in 0..cell_count {
                for x in 0..cell_count {
                    let i0 = (y * Terrain::VERTICES_PER_CHUNK + x) * scale;
                    let i1 = (y * Terrain::VERTICES_PER_CHUNK + (x + 1)) * scale;
                    let i2 = ((y + 1) * Terrain::VERTICES_PER_CHUNK + (x + 1)) * scale;
                    let i3 = ((y + 1) * Terrain::VERTICES_PER_CHUNK + x) * scale;

                    indices.extend_from_slice(&[i0, i1, i2, i2, i3, i0]);
                    wireframe_indices.extend_from_slice(&[i0, i1, i1, i2, i2, i3, i3, i0]);
                }
            }
        }

        let indices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chunk_indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let wireframe_indices_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("chunk_indices"),
                contents: bytemuck::cast_slice(&wireframe_indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        Self {
            indices_buffer,
            wireframe_indices_buffer,
        }
    }
}

#[derive(Clone, Copy, Default, bytemuck::NoUninit)]
#[repr(C)]
struct TerrainData {
    size: UVec2,
    nominal_edge_size: f32,
    altitude_map_height_base: f32,
    water_level: f32,

    water_trans_depth: f32,
    water_trans_low: f32,
    water_trans_high: f32,

    _padding: f32,
}

const CHUNK_INSTANCE_FLAG_STRATA_NORTH: u32 = 1 << 0;
const CHUNK_INSTANCE_FLAG_STRATA_EAST: u32 = 1 << 1;
const CHUNK_INSTANCE_FLAG_STRATA_SOUTH: u32 = 1 << 2;
const CHUNK_INSTANCE_FLAG_STRATA_WEST: u32 = 1 << 3;

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
struct GpuChunkInstance {
    center: Vec3,
    radius: f32,

    min_elevation: f32,
    max_elevation: f32,

    lod_index: u32,
    flags: u32,
}

impl std::fmt::Debug for GpuChunkInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GpuChunkInstance")
            .field("center", &self.center)
            .field("radius", &self.radius)
            .field("min_elevation", &self.min_elevation)
            .field("max_elevation", &self.max_elevation)
            .field("flags", &self.flags)
            .finish()
    }
}

pub struct Terrain {
    /// Height data for the terrain.
    pub height_map: HeightMap,

    /// The total amount of chunks of the terrain.
    total_chunks: u32,

    /// Pipeline to render the terrain.
    terrain_pipeline: wgpu::RenderPipeline,

    /// Pipeline to render the water.
    water_pipeline: wgpu::RenderPipeline,

    /// Pipeline to render a wireframe over the terrain (ignoring water).
    wireframe_pipeline: wgpu::RenderPipeline,

    /// Bind group containing all data required for rendering.
    render_bind_group: wgpu::BindGroup,

    terrain_data: Tracked<TerrainData>,
    terrain_data_buffer: wgpu::Buffer,

    strata: Strata,

    /// Pipeline that calculates LOD for each chunk and culls them in the camera frustum.
    process_chunks_pipeline: wgpu::ComputePipeline,

    /// The bind group with all the data required for processing all the chunks.
    process_chunks_bind_group: wgpu::BindGroup,

    /// Holds draw args for each terrain chunk.
    terrain_draw_args_buffer: wgpu::Buffer,

    /// Holds draw args for each water chunk.
    water_draw_args_buffer: wgpu::Buffer,

    /// Holds draw args for each water chunk.
    wireframe_draw_args_buffer: wgpu::Buffer,

    /// The mesh we use to render chunks.
    chunk_mesh: ChunkMesh,

    /// Each node: (normal, elevation)
    nodes: Vec<Vec4>,

    render_wireframe: bool,
    lod_level: usize,

    normals_lookup: Vec<Vec3>,

    /// An instance for each chunk to render for the terrain.
    chunk_instances: Vec<GpuChunkInstance>,
}

impl Terrain {
    pub const LOD_MAX: u32 = 3;

    /// The amount of cells in a chunk. The example has 4 cells.
    ///
    /// +--+--+--+--+
    /// |  |  |  |  |
    /// +--+--+--+--+
    /// |  |  |  |  |
    /// +--+--+--+--+
    /// |  |  |  |  |
    /// +--+--+--+--+
    /// |  |  |  |  |
    /// +--+--+--+--+
    pub const CELLS_PER_CHUNK: u32 = 1 << Self::LOD_MAX;

    /// The amount of vertices in the chunk. Always 1 more than the amount of cells.
    pub const VERTICES_PER_CHUNK: u32 = Self::CELLS_PER_CHUNK + 1;

    pub fn new(
        campaign_def: &CampaignDef,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        environment_bind_group_layout: &wgpu::BindGroupLayout,
        shadow_cascades: &ShadowCascades,
    ) -> Result<Self, AssetError> {
        let renderer = renderer();

        let terrain_mapping = data_dir().load_terrain_mapping(&campaign_def.base_name)?;

        let water_level =
            terrain_mapping.water_level as f32 * terrain_mapping.altitude_map_height_base;

        let terrain_texture_view = {
            let path = PathBuf::from("trnhigh")
                .join(format!("{}.jpg", terrain_mapping.texture_map_base_name));
            info!("Loading high detail terrain texture: {}", path.display());

            let image = images().load_image_direct(&path)?;
            renderer.create_texture_view("terrain_texture", &image.data)
        };

        let water_texture_view = {
            let image = images().load_image_direct(
                PathBuf::from("textures")
                    .join("image_processor")
                    .join("water2.bmp"),
            )?;
            renderer.create_texture_view("water", &image.data)
        };

        let height_map = {
            let path = PathBuf::from("maps").join(format!("{}.pcx", &campaign_def.base_name));
            info!("Loading terrain height map: {}", path.display());
            data_dir().load_height_map(
                path,
                terrain_mapping.altitude_map_height_base,
                terrain_mapping.nominal_edge_size,
            )?
        };

        let chunk_instances =
            Self::build_chunk_instances(&height_map, terrain_mapping.nominal_edge_size);

        let chunk_instances_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("chunk_instances_buffer"),
                    contents: bytemuck::cast_slice(&chunk_instances),
                    usage: wgpu::BufferUsages::STORAGE,
                });

        let normals_lookup = Self::generate_normals_lookup_table();

        // let normals = {
        //     // Load the normals: textures\terrain\{}\{}_vn.dat
        //     let path = PathBuf::from("textures")
        //         .join("terrain")
        //         .join(&campaign_def.base_name)
        //         .join(format!("{}_vn.dat", &campaign_def.base_name));
        //     let mut reader = std::io::Cursor::new(asset_loader.load_raw(path)?);
        //     let mut normals =
        //         Vec::with_capacity((height_map.size.x as usize) + (height_map.size.y as usize));
        //     for _ in 0..(height_map.size.x) * (height_map.size.y) {
        //         let index = reader.read_u16::<LE>()?;
        //         let normal = normals_lookup[index as usize];
        //         println!("normal: {index} {normal}");
        //         normals.push(normal);
        //     }
        //     normals
        // };

        let nodes_x: u32 = height_map.size.x + 1;
        let nodes_y: u32 = height_map.size.y + 1;
        let total_nodes = nodes_x * nodes_y;

        let normals = {
            let mut normals = vec![Vec3::Z; total_nodes as usize];

            for y in 1..nodes_x as i32 {
                for x in 1..nodes_y as i32 {
                    let center_node = IVec2::new(x, y);

                    let center = height_map.node_world_position(center_node);
                    let x_pos = height_map.node_world_position(center_node + IVec2::X);
                    let y_pos = height_map.node_world_position(center_node + IVec2::Y);

                    let normal = (x_pos - center).cross(y_pos - center).normalize();

                    let index = y as usize * (height_map.size.y as usize + 1) + x as usize;
                    normals[index] = normal;
                }
            }
            normals
        };

        let chunks = height_map.size / UVec2::splat(Terrain::CELLS_PER_CHUNK);
        let total_chunks = chunks.x * chunks.y;

        info!("chunks: {} x {} ({})", chunks.x, chunks.y, total_chunks);

        info!(
            "terrain size: {} x {}, terrain heightmap size: {} x {}",
            terrain_mapping.map_dx, terrain_mapping.map_dy, height_map.size.x, height_map.size.y,
        );

        let (height_map_buffer, nodes) = {
            let mut nodes = Vec::with_capacity(total_nodes as usize);

            for y in 0..nodes_y {
                for x in 0..nodes_x {
                    let position = height_map.node_world_position(IVec2::new(x as i32, y as i32));
                    let normal = normals
                        .get(y as usize * (nodes_x as usize) + x as usize)
                        .unwrap_or(&Vec3::Z);
                    nodes.push(normal.extend(position.z));
                }
            }

            (
                renderer
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("height_map"),
                        contents: bytemuck::cast_slice(&nodes),
                        usage: wgpu::BufferUsages::STORAGE,
                    }),
                nodes,
            )
        };

        let terrain_data = TerrainData {
            size: height_map.size,
            nominal_edge_size: terrain_mapping.nominal_edge_size,
            altitude_map_height_base: terrain_mapping.altitude_map_height_base,
            water_level,

            water_trans_depth: terrain_mapping.water_trans_depth,
            water_trans_low: terrain_mapping.water_trans_low as f32 / 256.0,
            water_trans_high: terrain_mapping.water_trans_high as f32 / 256.0,

            _padding: 0.0,
        };

        let terrain_data_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("terrain_data"),
                    contents: bytemuck::cast_slice(&[terrain_data]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("terrain_data"),
                    entries: &[
                        // u_height_map
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // u_height_map
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // u_terrain_data
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // t_terrain_texture
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        // t_water_texture
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        // s_terrain_texture
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let sampler = renderer.create_sampler(
            "terrain",
            wgpu::AddressMode::Repeat,
            wgpu::FilterMode::Linear,
            wgpu::FilterMode::Linear,
        );

        let render_bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("terrain_data"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: height_map_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: chunk_instances_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Buffer(
                            terrain_data_buffer.as_entire_buffer_binding(),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&terrain_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&water_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });

        let module = renderer
            .device
            .create_shader_module(wgsl_shader!("terrain"));

        let terrain_pipeline_layout =
            renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("terrain_pipeline_layout"),
                    bind_group_layouts: &[
                        camera_bind_group_layout,
                        environment_bind_group_layout,
                        &bind_group_layout,
                        &shadow_cascades.shadow_maps_bind_group.layout,
                    ],
                    push_constant_ranges: &[],
                });

        let terrain_pipeline =
            renderer
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("terrain_render_pipeline"),
                    layout: Some(&terrain_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some("vertex_terrain"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[],
                    },
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: Some(GeometryBuffers::depth_stencil_state(
                        wgpu::CompareFunction::LessEqual,
                        true,
                    )),
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some("fragment_terrain"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: GeometryBuffers::opaque_targets(),
                    }),
                    multiview: None,
                    cache: None,
                });

        let water_pipeline = {
            let layout = renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("water_pipeline_layout"),
                    bind_group_layouts: &[
                        camera_bind_group_layout,
                        environment_bind_group_layout,
                        &bind_group_layout,
                    ],
                    push_constant_ranges: &[wgpu::PushConstantRange {
                        stages: wgpu::ShaderStages::VERTEX,
                        range: 0..8,
                    }],
                });

            renderer
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("water_render_pipeline"),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some("vertex_water"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[],
                    },
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: Some(GeometryBuffers::depth_stencil_state(
                        wgpu::CompareFunction::LessEqual,
                        false,
                    )),
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some("fragment_water"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: GeometryBuffers::alpha_targets(),
                    }),
                    multiview: None,
                    cache: None,
                })
        };

        let wireframe_pipeline = {
            let module = renderer
                .device
                .create_shader_module(wgsl_shader!("terrain_wireframe"));

            let layout = renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("terrain_pipeline_layout"),
                    bind_group_layouts: &[camera_bind_group_layout, &bind_group_layout],
                    push_constant_ranges: &[],
                });

            renderer
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("terrain_wireframe_pipeline"),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some("vertex_wireframe"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[],
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::LineList,
                        ..Default::default()
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some("fragment_wireframe"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: GeometryBuffers::opaque_targets(),
                    }),
                    multiview: None,
                    cache: None,
                })
        };

        // Process chunks

        let draw_args_descriptor = {
            let size_of_indirect_args = std::mem::size_of::<wgpu::util::DrawIndexedIndirectArgs>();
            wgpu::BufferDescriptor {
                label: Some("terrain_draw_args"),
                size: size_of_indirect_args as u64 * total_chunks as u64,
                usage: wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            }
        };
        let terrain_draw_args_buffer = renderer.device.create_buffer(&draw_args_descriptor);
        let water_draw_args_buffer = renderer.device.create_buffer(&draw_args_descriptor);
        let wireframe_draw_args_buffer = renderer.device.create_buffer(&draw_args_descriptor);

        let strata = Strata::new(
            height_map.size,
            camera_bind_group_layout,
            environment_bind_group_layout,
            &height_map_buffer,
            &terrain_data_buffer,
        )?;

        let process_chunks_bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("process_chunks"),
                    entries: &[
                        // u_terrain_data
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // u_chunk_instances
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // u_terrain_draw_args
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // u_water_draw_args
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // u_wireframe_draw_args
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let module = renderer
            .device
            .create_shader_module(wgsl_shader!("process_chunks"));

        let layout = renderer
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("process_chunks"),
                bind_group_layouts: &[
                    camera_bind_group_layout,
                    environment_bind_group_layout,
                    &process_chunks_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let process_chunks_pipeline =
            renderer
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("process_chunks"),
                    layout: Some(&layout),
                    module: &module,
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    cache: None,
                });

        let process_chunks_bind_group =
            renderer
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("process_chunks"),
                    layout: &process_chunks_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Buffer(
                                terrain_data_buffer.as_entire_buffer_binding(),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Buffer(
                                chunk_instances_buffer.as_entire_buffer_binding(),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Buffer(
                                terrain_draw_args_buffer.as_entire_buffer_binding(),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::Buffer(
                                water_draw_args_buffer.as_entire_buffer_binding(),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: wgpu::BindingResource::Buffer(
                                wireframe_draw_args_buffer.as_entire_buffer_binding(),
                            ),
                        },
                    ],
                });

        let chunk_mesh = ChunkMesh::new(&renderer.device);

        Ok(Self {
            height_map,
            total_chunks,

            terrain_pipeline,
            water_pipeline,
            wireframe_pipeline,

            terrain_data: Tracked::new(terrain_data),
            terrain_data_buffer,

            strata,

            process_chunks_pipeline,
            process_chunks_bind_group,
            terrain_draw_args_buffer,
            water_draw_args_buffer,
            wireframe_draw_args_buffer,

            chunk_mesh,

            nodes,

            render_bind_group,
            render_wireframe: false,
            lod_level: 0,
            normals_lookup,

            chunk_instances,
        })
    }

    fn build_chunk_instances(
        height_map: &HeightMap,
        nominal_edge_size: f32,
    ) -> Vec<GpuChunkInstance> {
        let chunk_count = (height_map.size + UVec2::splat(Self::CELLS_PER_CHUNK) - UVec2::ONE)
            / Self::CELLS_PER_CHUNK;

        let mut chunk_instances =
            Vec::with_capacity(chunk_count.x as usize * chunk_count.y as usize);

        for chunk_y in 0..chunk_count.y as i32 {
            for chunk_x in 0..chunk_count.x as i32 {
                // The [BoundingSphere] API is causing us to re-allocate a new positions vec each
                // time.
                // let mut positions = {
                //     let side = Self::CELLS_PER_CHUNK as usize + 1;
                //     Vec::with_capacity(side * side)
                // };

                // To touch each node in the chunk, we have to do 0..=CELLS_PER_CHUNK to get the
                // end edges.
                //
                // 0   1   2   3   4   5   6   7   8
                // +---+---+---+---+---+---+---+---+
                // | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
                // +---+---+---+---+---+---+---+---+
                let chunk_min = IVec2::new(
                    chunk_x * Self::CELLS_PER_CHUNK as i32,
                    chunk_y * Self::CELLS_PER_CHUNK as i32,
                );
                let chunk_max = chunk_min + IVec2::splat(Self::CELLS_PER_CHUNK as i32);

                let mut min_elevation = f32::MAX;
                let mut max_elevation = f32::MIN;
                for node_y in chunk_min.y..=chunk_max.y {
                    for node_x in chunk_min.x..=chunk_max.x {
                        let elevation = height_map.node_elevation(IVec2::new(node_x, node_y));
                        min_elevation = min_elevation.min(elevation);
                        max_elevation = max_elevation.max(elevation);
                    }
                }

                let world_min = Vec3::new(
                    chunk_min.x as f32 * nominal_edge_size,
                    chunk_min.y as f32 * nominal_edge_size,
                    min_elevation,
                );

                let world_max = Vec3::new(
                    chunk_max.x as f32 * nominal_edge_size,
                    chunk_max.y as f32 * nominal_edge_size,
                    max_elevation,
                );

                let center = (world_min + world_max) * 0.5;
                let half_diagonal = (world_max - world_min) * 0.5;
                let radius = half_diagonal.length();

                let sphere = BoundingSphere { center, radius };

                let mut flags = 0_u32;
                if chunk_x == 0 {
                    flags |= CHUNK_INSTANCE_FLAG_STRATA_EAST;
                } else if chunk_x == chunk_count.x as i32 - 1 {
                    flags |= CHUNK_INSTANCE_FLAG_STRATA_WEST;
                }
                if chunk_y == 0 {
                    flags |= CHUNK_INSTANCE_FLAG_STRATA_SOUTH;
                } else if chunk_y == chunk_count.y as i32 - 1 {
                    flags |= CHUNK_INSTANCE_FLAG_STRATA_NORTH;
                }

                chunk_instances.push(GpuChunkInstance {
                    center: sphere.center,
                    radius: sphere.radius,
                    min_elevation,
                    max_elevation,
                    lod_index: 0, // Calculated in the process_chunks compute shader.
                    flags,
                });
            }
        }

        chunk_instances
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        frame: &mut Frame,
        in_editor: bool,
        geometry_buffers: &GeometryBuffers,
        shadow_cascades: &ShadowCascades,
        camera_bind_group: &wgpu::BindGroup,
        environment_bind_group: &wgpu::BindGroup,
        frustum_camera_bind_group: &wgpu::BindGroup,
    ) {
        let _z = tracy_client::span!("render terrain");

        // Make sure the terrain data is up to date if it changed.
        self.terrain_data.if_changed(|terrain_data| {
            renderer().queue.write_buffer(
                &self.terrain_data_buffer,
                0,
                bytemuck::cast_slice(&[*terrain_data]),
            );
        });

        // Always use the main camera for frustum culling.
        self.process_chunks(frame, frustum_camera_bind_group, environment_bind_group);

        self.strata.render(
            frame,
            geometry_buffers,
            camera_bind_group,
            environment_bind_group,
        );

        self.render_terrain(
            frame,
            geometry_buffers,
            shadow_cascades,
            camera_bind_group,
            environment_bind_group,
        );

        if in_editor && self.render_wireframe {
            self.render_wireframe(frame, geometry_buffers, camera_bind_group);
        }
    }

    pub fn render_gizmos(&self, vertices: &mut Vec<GizmoVertex>) {
        if false {
            // Render terrain chunk bounding spheres.
            for chunk_instance in self.chunk_instances.iter() {
                let transform = Mat4::from_translation(chunk_instance.center);
                let radius = chunk_instance.radius;
                vertices.extend(GizmosRenderer::create_iso_sphere(transform, radius, 32));
            }
        }

        if false {
            // Normals lookup.

            for (index, normal) in self.normals_lookup.iter().enumerate() {
                let color = if index == 0 {
                    Vec4::new(1.0, 0.0, 0.0, 1.0)
                } else if index == 0x1ff {
                    Vec4::new(0.0, 1.0, 0.0, 1.0)
                } else {
                    Vec4::new(1.0, 1.0, 1.0, 0.5)
                };
                vertices.push(GizmoVertex::new(Vec3::ZERO, color));
                vertices.push(GizmoVertex::new(*normal * 1_000.0, color));
            }
        }

        if false {
            // Normals
            for y in 0..self.height_map.size.y + 1 {
                for x in 0..self.height_map.size.x + 1 {
                    use glam::Vec4Swizzles;

                    let index = y as usize * (self.height_map.size.x + 1) as usize + x as usize;

                    let position = self
                        .height_map
                        .node_world_position(IVec2::new(x as i32, y as i32));
                    let normal = if x >= self.height_map.size.x || y >= self.height_map.size.y {
                        Vec3::Z
                    } else {
                        self.nodes.get(index).unwrap_or(&Vec3::Z.extend(1.0)).xyz()
                    };

                    vertices.push(GizmoVertex::new(position, Vec4::new(1.0, 0.0, 0.0, 1.0)));
                    vertices.push(GizmoVertex::new(
                        position + normal * 100.0,
                        Vec4::new(0.0, 1.0, 0.0, 1.0),
                    ));
                }
            }
        }
    }

    #[cfg(feature = "egui")]
    pub fn debug_panel(&mut self, ui: &mut egui::Ui) {
        use egui::widgets::DragValue;
        ui.checkbox(&mut self.render_wireframe, "Draw wireframe");

        ui.horizontal(|ui| {
            ui.label("Water level");

            let mut water_level = self.terrain_data.water_level;
            if ui
                .add(DragValue::new(&mut water_level).speed(1.0))
                .changed()
            {
                self.terrain_data.water_level = water_level;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Water depth");

            let mut depth = self.terrain_data.water_trans_depth;
            if ui.add(DragValue::new(&mut depth).speed(0.1)).changed() {
                self.terrain_data.water_trans_depth =
                    depth.clamp(0.0, self.terrain_data.water_level);
            }
        });

        ui.horizontal(|ui| {
            ui.label("Water trans");

            let mut low = self.terrain_data.water_trans_low;
            if ui.add(DragValue::new(&mut low).speed(0.001)).changed() {
                self.terrain_data.water_trans_low = low.clamp(0.0, 1.0);
            }

            let mut high = self.terrain_data.water_trans_high;
            if ui.add(DragValue::new(&mut high).speed(0.001)).changed() {
                self.terrain_data.water_trans_high = high.clamp(0.0, 1.0);
            }
        });

        for level in 0..=Self::LOD_MAX as usize {
            if ui
                .add(egui::widgets::RadioButton::new(
                    self.lod_level == level,
                    format!("level {level}"),
                ))
                .clicked()
            {
                self.lod_level = level;
            }
        }
    }
}

impl Terrain {
    /// Run the process_chunks compute shader to cull chunks not in the camera frustum and to set
    /// the LOD level.
    pub fn process_chunks(
        &self,
        frame: &mut Frame,
        camera_bind_group: &wgpu::BindGroup,
        environment_bind_group: &wgpu::BindGroup,
    ) {
        let mut compute_pass = frame
            .encoder
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("process_chunks"),
                timestamp_writes: None,
            });

        compute_pass.set_pipeline(&self.process_chunks_pipeline);
        compute_pass.set_bind_group(0, camera_bind_group, &[]);
        compute_pass.set_bind_group(1, environment_bind_group, &[]);
        compute_pass.set_bind_group(2, &self.process_chunks_bind_group, &[]);
        compute_pass.dispatch_workgroups(self.total_chunks.div_ceil(64), 1, 1);
    }

    fn render_terrain(
        &self,
        frame: &mut Frame,
        geometry_buffers: &GeometryBuffers,
        shadow_cascades: &ShadowCascades,
        camera_bind_group: &wgpu::BindGroup,
        environment_bind_group: &wgpu::BindGroup,
    ) {
        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("terrain_chunks"),
                color_attachments: &geometry_buffers.opaque_attachments(),
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &geometry_buffers.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        render_pass.set_pipeline(&self.terrain_pipeline);
        render_pass.set_index_buffer(
            self.chunk_mesh.indices_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, environment_bind_group, &[]);
        render_pass.set_bind_group(2, &self.render_bind_group, &[]);
        render_pass.set_bind_group(3, &shadow_cascades.shadow_maps_bind_group.bind_group, &[]);

        render_pass.multi_draw_indexed_indirect(
            &self.terrain_draw_args_buffer,
            0,
            self.total_chunks,
        );
    }

    pub fn render_water(
        &self,
        frame: &mut Frame,
        geometry_buffers: &GeometryBuffers,
        camera_bind_group: &wgpu::BindGroup,
        environment_bind_group: &wgpu::BindGroup,
    ) {
        let _z = tracy_client::span!("render water");

        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("water"),
                color_attachments: &geometry_buffers.alpha_attachments(),
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &geometry_buffers.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        render_pass.set_pipeline(&self.water_pipeline);
        render_pass.set_index_buffer(
            self.chunk_mesh.indices_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );

        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, environment_bind_group, &[]);
        render_pass.set_bind_group(2, &self.render_bind_group, &[]);

        render_pass.multi_draw_indexed_indirect(&self.water_draw_args_buffer, 0, self.total_chunks);
    }

    fn render_wireframe(
        &self,
        frame: &mut Frame,
        geometry_buffers: &GeometryBuffers,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("terrain_wireframe_render_pass"),
                color_attachments: &geometry_buffers.opaque_attachments(),
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        render_pass.set_pipeline(&self.wireframe_pipeline);
        render_pass.set_index_buffer(
            self.chunk_mesh.wireframe_indices_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.render_bind_group, &[]);

        render_pass.multi_draw_indexed_indirect(
            &self.wireframe_draw_args_buffer,
            0,
            self.total_chunks,
        );
    }

    fn generate_normals_lookup_table() -> Vec<Vec3> {
        let mut normals = Vec::with_capacity(1024); // 16 x 64 = 1024

        // x            y           z
        // 1            0           0
        // 0.995185     0.0980171   0
        // 0.980785     0.19509     0
        // 0.95694      0.290285    0
        // 0.92388      0.382683    0
        // 0.881921     0.471397    0
        // 0.83147      0.55557     0

        const INC: f32 = PI / 32.0; // 0.09817477 ~ PI/32

        for pitch in 0..16 {
            let z = ((pitch as f32) * INC).sin();

            for yaw in 0..64 {
                let angle = (yaw as f32) * INC;
                let x = angle.cos();
                let y = angle.sin();

                normals.push(Vec3 { x, y, z }.normalize());
            }
        }

        normals
    }
}
