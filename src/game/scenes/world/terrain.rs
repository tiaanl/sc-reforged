use std::{collections::HashMap, f32::consts::PI, path::PathBuf};

use glam::{IVec2, UVec2, Vec4};
use tracing::info;
use wgpu::util::DeviceExt;

use crate::{
    engine::{gizmos::GizmoVertex, prelude::*},
    game::{
        config::CampaignDef, data_dir::data_dir, geometry_buffers::GeometryBuffers,
        height_map::HeightMap,
    },
};

use super::strata::Strata;

struct ChunkMesh {
    vertices_buffer: wgpu::Buffer,
    indices_buffer: wgpu::Buffer,
    wireframe_indices_buffer: wgpu::Buffer,
}

impl ChunkMesh {
    fn new(renderer: &Renderer) -> Self {
        let mut vertices = Vec::with_capacity(
            Terrain::VERTICES_PER_CHUNK as usize * Terrain::VERTICES_PER_CHUNK as usize,
        );
        for y in 0..Terrain::VERTICES_PER_CHUNK {
            for x in 0..Terrain::VERTICES_PER_CHUNK {
                vertices.push(TerrainVertex {
                    index: UVec2::new(x, y),
                });
            }
        }

        let vertices_buffer = renderer.create_vertex_buffer("chunk_vertices", &vertices);

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

        let indices_buffer = renderer.create_index_buffer("chunk_indices", &indices);
        let wireframe_indices_buffer =
            renderer.create_index_buffer("chunk_wireframe_indices", &wireframe_indices);

        Self {
            vertices_buffer,
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

pub struct Terrain {
    /// Height data for the terrain.
    height_map: HeightMap,

    /// The total amount of chunks of the terrain.
    total_chunks: u32,

    /// Dictates the terrain resolution.
    pub max_view_distance: f32,

    /// Pipeline to render the terrain.
    terrain_pipeline: wgpu::RenderPipeline,

    /// Pipeline to render the water.
    water_pipeline: wgpu::RenderPipeline,

    /// Pipeline to render a wireframe over the terrain.
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

    /// The mesh we use to render chunks.
    chunk_mesh: ChunkMesh,

    chunk_data: Vec<ChunkData>,

    /// Each node: (normal, elevation)
    nodes: Vec<Vec4>,

    draw_wireframe: bool,
    draw_normals: bool,
    lod_level: usize,

    normals_lookup: Vec<Vec3>,
}

#[derive(Clone, Copy, Debug, bytemuck::NoUninit)]
#[repr(C)]
struct TerrainVertex {
    index: UVec2,
}

impl BufferLayout for TerrainVertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
            0 => Uint32x2,
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TerrainVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: ATTRS,
        }
    }
}

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
struct ChunkData {
    min: Vec3,
    _padding1: f32,
    max: Vec3,
    _padding2: f32,
}

impl std::fmt::Debug for ChunkData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChunkData")
            .field("min", &self.min)
            .field("max", &self.max)
            .finish()
    }
}

/// Fields copied from [wgpu::util::DrawIndexedIndirectArgs], but wgpu doesn't support bytemuck.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::NoUninit)]
struct ChunkDrawCall {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
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
        renderer: &Renderer,
        shaders: &mut Shaders,
        campaign_def: &CampaignDef,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self, AssetError> {
        let terrain_mapping = data_dir().load_terrain_mapping(&campaign_def.base_name)?;

        let water_level =
            terrain_mapping.water_level as f32 * terrain_mapping.altitude_map_height_base;

        shaders.add_module(include_str!("terrain_data.wgsl"), "terrain_data.wgsl");

        let terrain_texture_view = {
            let path = PathBuf::from("trnhigh")
                .join(format!("{}.jpg", terrain_mapping.texture_map_base_name));
            info!("Loading high detail terrain texture: {}", path.display());

            let image = data_dir().load_image(&path)?;
            renderer.create_texture_view("terrain_texture", &image.data)
        };

        let water_texture_view = {
            let image = data_dir().load_image(
                PathBuf::from("textures")
                    .join("image_processor")
                    .join("water2.bmp"),
            )?;
            renderer.create_texture_view("water", &image.data)
        };

        let height_map = {
            let path = PathBuf::from("maps").join(format!("{}.pcx", &campaign_def.base_name));
            info!("Loading terrain height map: {}", path.display());
            data_dir().load_height_map(path)?
        };

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

            let crate::game::config::TerrainMapping {
                nominal_edge_size,
                altitude_map_height_base,
                ..
            } = terrain_mapping;

            for y in 1..nodes_x as i32 {
                for x in 1..nodes_y as i32 {
                    let center = height_map.position_for_vertex(
                        IVec2::new(x, y),
                        nominal_edge_size,
                        altitude_map_height_base,
                    );
                    let x_pos = height_map.position_for_vertex(
                        IVec2::new(x + 1, y),
                        nominal_edge_size,
                        altitude_map_height_base,
                    );
                    let y_pos = height_map.position_for_vertex(
                        IVec2::new(x, y + 1),
                        nominal_edge_size,
                        altitude_map_height_base,
                    );

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

        let crate::game::config::TerrainMapping {
            nominal_edge_size,
            altitude_map_height_base,
            ..
        } = terrain_mapping;

        // Generate the array of chunks we use for frustum culling.
        let chunk_data = {
            let mut chunk_data = Vec::with_capacity(total_chunks as usize);
            for chunk_y in 0..chunks.y {
                for chunk_x in 0..chunks.x {
                    let y_range =
                        (chunk_y * Self::CELLS_PER_CHUNK)..=((chunk_y + 1) * Self::CELLS_PER_CHUNK);
                    let x_range =
                        (chunk_x * Self::CELLS_PER_CHUNK)..=((chunk_x + 1) * Self::CELLS_PER_CHUNK);

                    let mut min = Vec3::INFINITY;
                    let mut max = Vec3::NEG_INFINITY;
                    for y in y_range {
                        for x in x_range.clone() {
                            let position = height_map.position_for_vertex(
                                IVec2::new(x as i32, y as i32),
                                nominal_edge_size,
                                altitude_map_height_base,
                            );
                            min = min.min(position);
                            max = max.max(position);
                        }
                    }
                    chunk_data.push(ChunkData {
                        min,
                        _padding1: 0.0,
                        max,
                        _padding2: 0.0,
                    });
                }
            }
            chunk_data
        };

        info!(
            "terrain size: {} x {}, terrain heightmap size: {} x {}",
            terrain_mapping.map_dx, terrain_mapping.map_dy, height_map.size.x, height_map.size.y,
        );

        let (height_map_buffer, nodes) = {
            let mut nodes = Vec::with_capacity(total_nodes as usize);

            for y in 0..nodes_y {
                for x in 0..nodes_x {
                    let position = height_map.position_for_vertex(
                        IVec2::new(x as i32, y as i32),
                        nominal_edge_size,
                        altitude_map_height_base,
                    );
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
            water_trans_low: terrain_mapping.water_trans_low as f32 / 512.0,
            water_trans_high: terrain_mapping.water_trans_high as f32 / 512.0,

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
                        // u_terrain_data
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
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
                            binding: 2,
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
                            binding: 3,
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
                            binding: 4,
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
                        resource: wgpu::BindingResource::Buffer(
                            height_map_buffer.as_entire_buffer_binding(),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(
                            terrain_data_buffer.as_entire_buffer_binding(),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&terrain_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&water_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });

        let module = shaders.create_shader(
            renderer,
            "terrain",
            include_str!("terrain.wgsl"),
            "terrain.wgsl",
            HashMap::default(),
        );

        let terrain_pipeline_layout =
            renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("terrain_pipeline_layout"),
                    bind_group_layouts: &[camera_bind_group_layout, &bind_group_layout],
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
                        entry_point: Some("vertex_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[TerrainVertex::layout()],
                    },
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: Some(DepthBuffer::depth_stencil_state(
                        wgpu::CompareFunction::LessEqual,
                        true,
                    )),
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some("fragment_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: GeometryBuffers::opaque_targets(),
                    }),
                    multiview: None,
                    cache: None,
                });

        let water_pipeline = renderer
            .build_render_pipeline::<TerrainVertex>("water", &module)
            .with_vertex_entry("water_vertex_main")
            .with_fragment_entry("water_fragment_main")
            .binding(camera_bind_group_layout)
            .binding(&bind_group_layout)
            .push_constant(wgpu::ShaderStages::VERTEX, 0..8)
            .with_depth_compare(wgpu::CompareFunction::LessEqual)
            // .with_depth_writes(false)
            .with_blend(wgpu::BlendState::ALPHA_BLENDING)
            .build();

        let wireframe_pipeline = renderer
            .build_render_pipeline::<TerrainVertex>("terrain_wireframe", &module)
            .with_primitive(wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            })
            .with_vertex_entry("wireframe_vertex_main")
            .with_fragment_entry("wireframe_fragment_main")
            .binding(camera_bind_group_layout)
            .binding(&bind_group_layout)
            .push_constant(wgpu::ShaderStages::VERTEX, 0..8)
            .build();

        // Process chunks

        let chunk_data_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("chunk_data"),
                    contents: bytemuck::cast_slice(&chunk_data),
                    usage: wgpu::BufferUsages::STORAGE,
                });

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

        let strata = Strata::new(
            renderer,
            shaders,
            height_map.size,
            camera_bind_group_layout,
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
                        // u_chunk_data
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
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
                    ],
                });

        let module = shaders.create_shader(
            renderer,
            "process_chunks",
            include_str!("process_chunks.wgsl"),
            "process_chunks.wgsl",
            HashMap::default(),
        );

        let layout = renderer
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("process_chunks"),
                bind_group_layouts: &[camera_bind_group_layout, &process_chunks_bind_group_layout],
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
                                chunk_data_buffer.as_entire_buffer_binding(),
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
                    ],
                });

        let chunk_mesh = ChunkMesh::new(renderer);

        Ok(Self {
            height_map,
            total_chunks,

            max_view_distance: 13_300.0,
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

            chunk_mesh,

            chunk_data,

            nodes,

            render_bind_group,
            draw_wireframe: false,
            draw_normals: false,
            lod_level: 0,
            normals_lookup,
        })
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        geometry_buffers: &GeometryBuffers,
        camera_bind_group: &wgpu::BindGroup,
        frustum_camera_bind_group: &wgpu::BindGroup,
    ) {
        // Make sure the terrain data is up to date if it changed.
        self.terrain_data.if_changed(|terrain_data| {
            frame.queue.write_buffer(
                &self.terrain_data_buffer,
                0,
                bytemuck::cast_slice(&[*terrain_data]),
            );
        });

        // Always use the main camera for frustum culling.
        self.process_chunks(&frame.device, &frame.queue, frustum_camera_bind_group);

        self.strata
            .render(frame, geometry_buffers, camera_bind_group);

        self.render_terrain(frame, geometry_buffers, camera_bind_group);
    }

    pub fn render_gizmos(&self, vertices: &mut Vec<GizmoVertex>) {
        // if self.draw_wireframe {
        //     self.render_wireframe(frame, camera_bind_group);
        // }

        if false {
            let color = Vec4::new(0.0, 1.0, 0.0, 1.0);
            for data in self.chunk_data.iter() {
                let ChunkData { min, max, .. } = data;

                vertices.push(GizmoVertex::new(Vec3::new(min.x, min.y, min.z), color));
                vertices.push(GizmoVertex::new(Vec3::new(min.x, min.y, max.z), color));

                vertices.push(GizmoVertex::new(Vec3::new(min.x, min.y, min.z), color));
                vertices.push(GizmoVertex::new(Vec3::new(min.x, max.y, min.z), color));

                vertices.push(GizmoVertex::new(Vec3::new(min.x, min.y, min.z), color));
                vertices.push(GizmoVertex::new(Vec3::new(max.x, min.y, min.z), color));

                vertices.push(GizmoVertex::new(Vec3::new(min.x, max.y, min.z), color));
                vertices.push(GizmoVertex::new(Vec3::new(min.x, max.y, max.z), color));

                vertices.push(GizmoVertex::new(Vec3::new(min.x, max.y, min.z), color));
                vertices.push(GizmoVertex::new(Vec3::new(max.x, max.y, min.z), color));

                vertices.push(GizmoVertex::new(Vec3::new(max.x, min.y, min.z), color));
                vertices.push(GizmoVertex::new(Vec3::new(max.x, min.y, max.z), color));

                vertices.push(GizmoVertex::new(Vec3::new(max.x, min.y, min.z), color));
                vertices.push(GizmoVertex::new(Vec3::new(max.x, max.y, min.z), color));

                vertices.push(GizmoVertex::new(Vec3::new(min.x, min.y, max.z), color));
                vertices.push(GizmoVertex::new(Vec3::new(min.x, max.y, max.z), color));

                vertices.push(GizmoVertex::new(Vec3::new(min.x, min.y, max.z), color));
                vertices.push(GizmoVertex::new(Vec3::new(max.x, min.y, max.z), color));

                vertices.push(GizmoVertex::new(Vec3::new(max.x, min.y, max.z), color));
                vertices.push(GizmoVertex::new(Vec3::new(max.x, max.y, max.z), color));

                vertices.push(GizmoVertex::new(Vec3::new(min.x, max.y, max.z), color));
                vertices.push(GizmoVertex::new(Vec3::new(max.x, max.y, max.z), color));

                vertices.push(GizmoVertex::new(Vec3::new(max.x, max.y, min.z), color));
                vertices.push(GizmoVertex::new(Vec3::new(max.x, max.y, max.z), color));
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
            let nominal_edge_size = self.terrain_data.nominal_edge_size;
            let altitude_map_height_base = self.terrain_data.altitude_map_height_base;

            // Normals
            for y in 0..self.height_map.size.y + 1 {
                for x in 0..self.height_map.size.x + 1 {
                    use glam::Vec4Swizzles;

                    let index = y as usize * (self.height_map.size.x + 1) as usize + x as usize;

                    let position = self.height_map.position_for_vertex(
                        IVec2::new(x as i32, y as i32),
                        nominal_edge_size,
                        altitude_map_height_base,
                    );
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
        ui.checkbox(&mut self.draw_wireframe, "Draw wireframe");

        ui.add(DragValue::new(&mut self.max_view_distance).speed(10.0));

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
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("process_chunks"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("process_chunks"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.process_chunks_pipeline);
            compute_pass.set_bind_group(0, camera_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.process_chunks_bind_group, &[]);
            compute_pass.dispatch_workgroups(self.total_chunks.div_ceil(64), 1, 1);
        }

        queue.submit(std::iter::once(encoder.finish()));
    }

    #[inline]
    fn render_patch(
        render_pass: &mut wgpu::RenderPass,
        x: u32,
        y: u32,
        range: std::ops::Range<u32>,
    ) {
        render_pass.set_push_constants(wgpu::ShaderStages::VERTEX, 0, &x.to_ne_bytes());
        render_pass.set_push_constants(wgpu::ShaderStages::VERTEX, 4, &y.to_ne_bytes());
        render_pass.draw_indexed(range, 0, 0..1);
    }

    fn render_terrain(
        &self,
        frame: &mut Frame,
        geometry_buffers: &GeometryBuffers,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("terrain_chunks"),
                color_attachments: &geometry_buffers.opaque_color_attachments(),
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &frame.depth_buffer.texture_view,
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
        render_pass.set_vertex_buffer(0, self.chunk_mesh.vertices_buffer.slice(..));
        render_pass.set_index_buffer(
            self.chunk_mesh.indices_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.render_bind_group, &[]);

        render_pass.multi_draw_indexed_indirect(
            &self.terrain_draw_args_buffer,
            0,
            self.total_chunks,
        );
    }

    pub fn render_water(&self, frame: &mut Frame, camera_bind_group: &wgpu::BindGroup) {
        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("water"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.surface,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &frame.depth_buffer.texture_view,
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

        render_pass.set_vertex_buffer(0, self.chunk_mesh.vertices_buffer.slice(..));
        render_pass.set_index_buffer(
            self.chunk_mesh.indices_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );

        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.render_bind_group, &[]);

        render_pass.multi_draw_indexed_indirect(&self.water_draw_args_buffer, 0, self.total_chunks);
    }

    fn render_wireframe(&self, frame: &mut Frame, camera_bind_group: &wgpu::BindGroup) {
        let levels = [0..512, 512..640, 640..672, 672..680];

        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("terrain_chunks"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.surface,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        render_pass.set_pipeline(&self.wireframe_pipeline);
        render_pass.set_vertex_buffer(0, self.chunk_mesh.vertices_buffer.slice(..));
        render_pass.set_index_buffer(
            self.chunk_mesh.wireframe_indices_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );

        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.render_bind_group, &[]);

        for y in 0..self.height_map.size.y / Terrain::CELLS_PER_CHUNK {
            for x in 0..self.height_map.size.x / Terrain::CELLS_PER_CHUNK {
                Self::render_patch(&mut render_pass, x, y, levels[self.lod_level].clone());
            }
        }
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

    fn calculate_node_normals(&mut self, nominal_edge_size: f32, altitude_map_height_map: f32) {
        for y in 1..self.height_map.size.y as i32 {
            for x in 1..self.height_map.size.x as i32 {
                let center = self.height_map.position_for_vertex(
                    IVec2::new(x, y),
                    nominal_edge_size,
                    altitude_map_height_map,
                );
                let x_pos = self.height_map.position_for_vertex(
                    IVec2::new(x + 1, y),
                    nominal_edge_size,
                    altitude_map_height_map,
                );
                let y_pos = self.height_map.position_for_vertex(
                    IVec2::new(x, y + 1),
                    nominal_edge_size,
                    altitude_map_height_map,
                );
                let x_neg = self.height_map.position_for_vertex(
                    IVec2::new(x - 1, y),
                    nominal_edge_size,
                    altitude_map_height_map,
                );
                let y_neg = self.height_map.position_for_vertex(
                    IVec2::new(x, y - 1),
                    nominal_edge_size,
                    altitude_map_height_map,
                );

                let v1 = (x_pos - center).cross(y_pos - center);
                let v2 = (y_pos - center).cross(x_neg - center);
                let v3 = (x_neg - center).cross(y_neg - center);
                let v4 = (y_neg - center).cross(x_pos - center);

                let normal = (v1 + v2 + v3 + v4).normalize();

                let node = &mut self.nodes
                    [y as usize * (self.height_map.size.x as usize + 1) + x as usize];

                node.x = normal.x;
                node.y = normal.y;
                node.z = normal.z;
            }
        }
    }
}
