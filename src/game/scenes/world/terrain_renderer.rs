use std::ops::Range;

use ahash::HashSet;
use bitflags::bitflags;
use glam::{IVec2, Vec3};
use wgpu::util::DeviceExt;

use crate::{
    engine::prelude::{Frame, renderer},
    game::{geometry_buffers::GeometryBuffers, image::images, shadows::ShadowCascades},
    wgsl_shader,
};

use super::terrain::Terrain;

bitflags! {
    pub struct LodFlags : u8 {
        const NORTH = 1 << 0;
        const EAST = 1 << 1;
        const SOUTH = 1 << 2;
        const WEST = 1 << 3;
    }
}

pub struct ChunkSnapshot {
    pub index: u32,
    pub lod: u8,
    pub lod_flags: LodFlags,
}

pub struct TerrainSnapshot {
    pub chunks: Vec<ChunkSnapshot>,
}

pub struct TerrainRenderer {
    /// GPU buffer holding indices for solid and writeframe mesh indices for multiple LOD's.
    chunk_indices: ChunkIndices,

    /// Bind group for all the terrain data on the GPU.
    terrain_bind_group: wgpu::BindGroup,

    /// Pipeline to render the terrain.
    terrain_pipeline: wgpu::RenderPipeline,
}

impl TerrainRenderer {
    pub fn new(
        terrain: &Terrain,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        environment_bind_group_layout: &wgpu::BindGroupLayout,
        shadow_cascades: &ShadowCascades,
    ) -> Self {
        let device = &renderer().device;

        let chunk_indices = ChunkIndices::new(device);

        let terrain_data_buffer = {
            let terrain_data = gpu::TerrainData::from(terrain);
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("terrain_data"),
                contents: bytemuck::bytes_of(&terrain_data),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            })
        };

        let nodes_buffer = {
            let nodes_x = terrain.height_map.size.x as i32 + 1;
            let nodes_y = terrain.height_map.size.y as i32 + 1;

            let mut nodes = Vec::with_capacity(nodes_x as usize * nodes_y as usize);

            for y in 0..nodes_y {
                for x in 0..nodes_x {
                    let node_index = y as usize * nodes_x as usize + x as usize;
                    let normal = terrain.normals[node_index];
                    let elevation = terrain.height_map.node_elevation(IVec2::new(x, y));
                    nodes.push(normal.extend(elevation));
                }
            }

            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("terrain_nodes"),
                contents: bytemuck::cast_slice(&nodes),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            })
        };

        let terrain_texture_view = {
            let terrain_image = images()
                .get(terrain.terrain_image)
                .expect("Image terrain image.");
            renderer().create_texture_view("terrain", &terrain_image.data)
        };

        let water_texture_view = {
            let image = images()
                .get(terrain.water_image)
                .expect("Invalid water image.");

            renderer().create_texture_view("terrain_water", &image.data)
        };

        let sampler = renderer().create_sampler(
            "terrain",
            wgpu::AddressMode::Repeat,
            wgpu::FilterMode::Linear,
            wgpu::FilterMode::Linear,
        );

        let terrain_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("terrain_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let terrain_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("terrain_bind_group"),
            layout: &terrain_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: terrain_data_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: nodes_buffer.as_entire_binding(),
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

        let module = device.create_shader_module(wgsl_shader!("terrain_renderer"));

        let terrain_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("terrain_pipeline_layout"),
                bind_group_layouts: &[
                    camera_bind_group_layout,
                    environment_bind_group_layout,
                    &shadow_cascades.shadow_maps_bind_group.layout,
                    &terrain_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let terrain_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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

        Self {
            chunk_indices,
            terrain_bind_group,
            terrain_pipeline,
        }
    }

    pub fn render_terrain(
        &self,
        frame: &mut Frame,
        geometry_buffers: &GeometryBuffers,
        camera_bind_group: &wgpu::BindGroup,
        environment_bind_group: &wgpu::BindGroup,
        shadow_cascades: &ShadowCascades,
        snapshot: &TerrainSnapshot,
    ) {
        // Upload any new data to the GPU.
        self.upload_snapshot(snapshot);

        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("terrain"),
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
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, environment_bind_group, &[]);
        render_pass.set_bind_group(2, &shadow_cascades.shadow_maps_bind_group.bind_group, &[]);
        render_pass.set_bind_group(3, &self.terrain_bind_group, &[]);

        render_pass.set_index_buffer(
            self.chunk_indices.indices_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );

        for chunk in snapshot.chunks.iter() {
            let chunk_index = chunk.index as u32;
            render_pass.draw_indexed(
                self.chunk_indices.indices(0),
                0,
                chunk_index..chunk_index + 1,
            );
        }
    }

    /// Take data from the snapshot and upload it to the GPU.
    fn upload_snapshot(&self, snapshot: &TerrainSnapshot) {}
}

struct ChunkIndices {
    indices_buffer: wgpu::Buffer,
    _wireframe_indices_buffer: wgpu::Buffer,
}

impl ChunkIndices {
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

            for y in 0..cell_count {
                for x in 0..cell_count {
                    let i0 = y * Terrain::VERTICES_PER_CHUNK + x;
                    let i1 = y * Terrain::VERTICES_PER_CHUNK + (x + 1);
                    let i2 = (y + 1) * Terrain::VERTICES_PER_CHUNK + (x + 1);
                    let i3 = (y + 1) * Terrain::VERTICES_PER_CHUNK + x;

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
            _wireframe_indices_buffer: wireframe_indices_buffer,
        }
    }

    fn indices(&self, lod: u32) -> Range<u32> {
        debug_assert!(lod <= Terrain::LOD_MAX);

        const RANGES: [Range<u32>; 4] = [0..384, 384..480, 480..504, 504..510];
        RANGES[lod as usize].clone()
    }

    fn _wireframe_indices(&self, lod: u32) -> Range<u32> {
        debug_assert!(lod <= Terrain::LOD_MAX);

        const RANGES: [Range<u32>; 4] = [0..512, 512..640, 640..672, 672..680];
        RANGES[lod as usize].clone()
    }
}

mod gpu {
    use super::Terrain;
    use bytemuck::NoUninit;

    #[derive(Clone, Copy, NoUninit)]
    #[repr(C)]
    pub struct TerrainData {
        pub cell_count: [u32; 2],
        pub chunk_count: [u32; 2],
        pub nominal_edge_size: f32,
        pub _pad0: [f32; 3],
        pub water_elevation: f32,
        pub water_trans_depth: f32,
        pub water_trans_low: f32,
        pub water_trans_high: f32,
        pub _pad1: [f32; 4],
    }

    impl From<&Terrain> for TerrainData {
        fn from(terrain: &Terrain) -> Self {
            Self {
                cell_count: terrain.height_map.size.to_array(),
                chunk_count: (terrain.height_map.size / Terrain::CELLS_PER_CHUNK).to_array(),
                nominal_edge_size: terrain.nominal_edge_size,
                _pad0: Default::default(),
                water_elevation: terrain.water_elevation,
                water_trans_depth: terrain.water_trans_depth,
                water_trans_low: terrain.water_trans_low,
                water_trans_high: terrain.water_trans_high,
                _pad1: Default::default(),
            }
        }
    }
}
