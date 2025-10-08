use std::path::PathBuf;

use ahash::HashMap;
use glam::{IVec2, UVec2, Vec3, ivec2};
use wgpu::util::DeviceExt;

use crate::{
    game::{
        image::images,
        scenes::world::{
            new_terrain::NewTerrain,
            render_world::{ChunkInstanceData, RenderWorld},
            systems::{ExtractContext, NewSystemContext, PrepareContext, QueueContext, System},
        },
    },
    wgsl_shader,
};

pub struct TerrainSystem {
    /// Dimensions of the terrain in chunks.
    _chunks_dim: UVec2,

    /// Buffer holding indices to render a single chunk at various LOD's.
    chunk_indices_buffer: wgpu::Buffer,
    /// Buffer holding indices to render a wireframe over a single chunk of various LOD's.
    _chunk_wireframe_indices_buffer: wgpu::Buffer,

    /// Bind group for all terrain GPU resources.
    terrain_bind_group: wgpu::BindGroup,

    /// A *transient* cache of LOD's for the current frame.
    chunk_lod_cache: HashMap<IVec2, u32>,

    /// Pipeline to render the terrain chunks.
    terrain_pipeline: wgpu::RenderPipeline,

    /// Pipeline to render the stratas.
    strata_pipeline: wgpu::RenderPipeline,

    /// Buffer holding vertices for the strata.
    strata_vertex_buffer: wgpu::Buffer,

    /// Buffer holding indices for the strata at different sides and lod's.
    strata_index_buffer: wgpu::Buffer,
}

impl TerrainSystem {
    const STRATA_DESCENT: f32 = -20_000.0;

    const INDEX_RANGES: [std::ops::Range<u32>; 4] = [0..384, 384..480, 480..504, 504..510];
    const _WIREFRAME_INDEX_RANGES: [std::ops::Range<u32>; 4] =
        [0..512, 512..640, 640..672, 672..680];

    pub fn new(context: &mut NewSystemContext) -> Self {
        let NewSystemContext {
            renderer,
            render_store,
            sim_world,
        } = context;

        let height_map = &sim_world.terrain.height_map;

        let cells_dim = height_map.size;
        let chunks_dim = cells_dim / NewTerrain::CELLS_PER_CHUNK;

        let terrain_data_buffer = {
            #[derive(Clone, Copy, bytemuck::NoUninit)]
            #[repr(C)]
            struct TerrainData {
                cells_dim: [u32; 2],
                chunks_dim: [u32; 2],
                cell_size: f32,
                strata_descent: f32,
                _pad: [u32; 2],
            }

            let terrain_data = TerrainData {
                cells_dim: cells_dim.to_array(),
                chunks_dim: chunks_dim.to_array(),
                cell_size: height_map.cell_size,
                strata_descent: Self::STRATA_DESCENT,
                _pad: Default::default(),
            };

            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("terrain_data_buffer"),
                    contents: bytemuck::bytes_of(&terrain_data),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                })
        };

        let height_map_buffer = {
            let data: Vec<_> = height_map
                .nodes
                .iter()
                .map(|node| node.to_array())
                .collect();

            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("height_map_buffer"),
                    contents: bytemuck::cast_slice(&data),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                })
        };

        let terrain_texture = {
            let image = images().get(sim_world.terrain.terrain_texture).unwrap();
            renderer.create_texture_view("terrain_texture", &image.data)
        };

        let strata_texture = {
            let path = PathBuf::from("textures").join("shared").join("strata.bmp");
            let image = images()
                .load_image_direct(path)
                .expect("Could not load strata texture.");
            renderer.create_texture_view("strata", &image.data)
        };

        let terrain_sampler = renderer.create_sampler(
            "terrain_sampler",
            wgpu::AddressMode::Repeat,
            wgpu::FilterMode::Linear,
            wgpu::FilterMode::Linear,
        );

        let terrain_bind_group_layout = {
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
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
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                })
        };

        let terrain_bind_group = {
            renderer
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("terrain_bind_group"),
                    layout: &terrain_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: terrain_data_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: height_map_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&terrain_texture),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(&strata_texture),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: wgpu::BindingResource::Sampler(&terrain_sampler),
                        },
                    ],
                })
        };

        let (chunk_indices_buffer, chunk_wireframe_indices_buffer) = {
            let chunk_indices = ChunkIndices::default();

            let chunk_indices_buffer =
                renderer
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("chunk_indices_buffer"),
                        contents: bytemuck::cast_slice(&chunk_indices.indices),
                        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                    });

            let chunk_wireframe_indices_buffer =
                renderer
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("chunk_indices_buffer"),
                        contents: bytemuck::cast_slice(&chunk_indices.wireframe_indices),
                        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                    });

            (chunk_indices_buffer, chunk_wireframe_indices_buffer)
        };

        let module = renderer
            .device
            .create_shader_module(wgsl_shader!("new_terrain"));

        let camera_bind_group_layout = render_store
            .get_bind_group_layout(RenderWorld::CAMERA_BIND_GROUP_LAYOUT_ID)
            .expect("Requires camera bind_group_layout");

        let layout = renderer
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("terrain_pipeline_layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &terrain_bind_group_layout],
                push_constant_ranges: &[],
            });

        let instance_attrs = wgpu::vertex_attr_array![
            0 => Uint32x2,
            1 => Uint32,
            2 => Uint32,
        ];

        let terrain_pipeline =
            renderer
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("terrain_pipeline"),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some("vertex_terrain"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<ChunkInstanceData>()
                                as wgpu::BufferAddress,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &instance_attrs,
                        }],
                    },
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::LessEqual,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some("fragment_terrain"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: renderer.surface.format(),
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview: None,
                    cache: None,
                });

        let strata_pipeline =
            renderer
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("strata_render_pipeline"),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some("strata_vertex"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[
                            wgpu::VertexBufferLayout {
                                array_stride: std::mem::size_of::<ChunkInstanceData>()
                                    as wgpu::BufferAddress,
                                step_mode: wgpu::VertexStepMode::Instance,
                                attributes: &instance_attrs,
                            },
                            wgpu::VertexBufferLayout {
                                array_stride: std::mem::size_of::<StrataVertex>()
                                    as wgpu::BufferAddress,
                                step_mode: wgpu::VertexStepMode::Vertex,
                                attributes: &wgpu::vertex_attr_array![
                                    3 => Float32x3,  // normal
                                    4 => Uint32x2,  // node_coord
                                ],
                            },
                        ],
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleStrip,
                        cull_mode: Some(wgpu::Face::Back),
                        // polygon_mode: wgpu::PolygonMode::Line,
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::LessEqual,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some("strata_fragment"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: renderer.surface.format(),
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview: None,
                    cache: None,
                });

        let strata_vertex_buffer = {
            let vertices = generate_strata_vertices();

            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("strata_vertices"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                })
        };

        let strata_index_buffer = {
            let indices = generate_strata_indices();

            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("strata_indices"),
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                })
        };

        Self {
            _chunks_dim: chunks_dim,

            chunk_indices_buffer,
            _chunk_wireframe_indices_buffer: chunk_wireframe_indices_buffer,

            terrain_bind_group,

            terrain_pipeline,
            strata_pipeline,

            strata_vertex_buffer,
            strata_index_buffer,

            chunk_lod_cache: HashMap::default(),
        }
    }

    /// Build a list of instances per LOD.
    /// `chunk_instances` *must* be sorted by LOD.
    fn build_draw_commands(
        chunk_instances: &[ChunkInstanceData],
        ranges: &[std::ops::Range<u32>],
    ) -> [(std::ops::Range<u32>, std::ops::Range<u32>); NewTerrain::LOD_COUNT as usize] {
        // TODO: This is probably not needed.
        debug_assert!(chunk_instances.is_sorted_by_key(|instance| instance.lod));

        let mut counts = [0_u32; NewTerrain::LOD_COUNT as usize];

        // Count the number of each LOD.
        for instance in chunk_instances {
            let lod = instance.lod.min(NewTerrain::LOD_MAX) as usize;
            counts[lod] += 1;
        }

        // Create starting indices by accumulating the LOD counts.
        let mut offsets = [0_u32; NewTerrain::LOD_COUNT as usize];
        let mut acc = 0;
        for i in 0..counts.len() {
            offsets[i] = acc;
            acc += counts[i];
        }

        [
            (ranges[0].clone(), offsets[0]..offsets[0] + counts[0]),
            (ranges[1].clone(), offsets[1]..offsets[1] + counts[1]),
            (ranges[2].clone(), offsets[2]..offsets[2] + counts[2]),
            (ranges[3].clone(), offsets[3]..offsets[3] + counts[3]),
        ]
    }
}

impl System for TerrainSystem {
    fn extract(&mut self, context: &mut ExtractContext) {
        self.chunk_lod_cache.clear();

        let camera_position = context.sim_world.computed_camera.position;
        let camera_forward = context.sim_world.computed_camera.forward;
        let camera_far = context.sim_world.camera.far;

        let terrain_chunk_instances = &mut context.render_world.terrain_chunk_instances;
        let strata_instances = &mut context.render_world.strata_instances;
        let strata_instances_side_count = &mut context.render_world.strata_instances_side_count;

        terrain_chunk_instances.clear();
        strata_instances.clear();
        *strata_instances_side_count = [0; 4];

        for visible_coord in context.sim_world.visible_chunks.iter() {
            let mut lod_at = |coord: IVec2| {
                context.sim_world.terrain.chunk_at(coord).map(|chunk| {
                    *self.chunk_lod_cache.entry(coord).or_insert({
                        let center = chunk.bounding_box.center();
                        calculate_lod(camera_position, camera_forward, camera_far, center)
                    })
                })
            };

            let center_lod = lod_at(*visible_coord).expect("Center chunk is always valid!");

            let mut flags = 0_u32;
            let neighbors = [ivec2(0, 1), ivec2(-1, 0), ivec2(0, -1), ivec2(1, 0)]
                .map(|offset| lod_at(*visible_coord + offset));
            for (i, neighbor_lod) in neighbors.iter().enumerate() {
                // A higher LOD means the resolution is lower, so we check greater than here.
                if neighbor_lod.unwrap_or(center_lod) > center_lod {
                    flags |= 1 << i;
                }
            }

            let chunk_instance = ChunkInstanceData {
                coord: visible_coord.as_uvec2().to_array(),
                lod: center_lod,
                flags,
            };

            terrain_chunk_instances.push(chunk_instance);

            const SOUTH: u32 = 0;
            const WEST: u32 = 1;
            const NORTH: u32 = 2;
            const EAST: u32 = 3;

            if visible_coord.x == 0 {
                let chunk_instance = ChunkInstanceData {
                    flags: chunk_instance.flags | (EAST << 8),
                    ..chunk_instance
                };
                strata_instances.push(chunk_instance);
                strata_instances_side_count[EAST as usize] += 1;
            } else if visible_coord.x == self._chunks_dim.x as i32 - 1 {
                let chunk_instance = ChunkInstanceData {
                    flags: chunk_instance.flags | (WEST << 8),
                    ..chunk_instance
                };
                strata_instances.push(chunk_instance);
                strata_instances_side_count[WEST as usize] += 1;
            }

            if visible_coord.y == 0 {
                let chunk_instance = ChunkInstanceData {
                    flags: chunk_instance.flags | (SOUTH << 8),
                    ..chunk_instance
                };
                strata_instances.push(chunk_instance);
                strata_instances_side_count[SOUTH as usize] += 1;
            } else if visible_coord.y == self._chunks_dim.y as i32 - 1 {
                let chunk_instance = ChunkInstanceData {
                    flags: chunk_instance.flags | (NORTH << 8),
                    ..chunk_instance
                };
                strata_instances.push(chunk_instance);
                strata_instances_side_count[NORTH as usize] += 1;
            }
        }

        strata_instances.sort_unstable_by_key(|instance| instance.flags >> 8 & 0b11);

        context
            .render_world
            .terrain_chunk_instances
            .sort_unstable_by_key(|instance| instance.lod);
    }

    fn prepare(&mut self, context: &mut PrepareContext) {
        let PrepareContext {
            render_world,
            renderer,
            ..
        } = context;

        // Upload the chunk instance data.
        {
            render_world.ensure_terrain_chunk_instance_capacity(
                &renderer.device,
                render_world.terrain_chunk_instances.len() as u32,
            );

            renderer.queue.write_buffer(
                &render_world.terrain_chunk_instances_buffer,
                0,
                bytemuck::cast_slice(&render_world.terrain_chunk_instances),
            );
        }

        // Upload the strata data.
        {
            render_world.ensure_strata_instance_capacity(
                &renderer.device,
                render_world.strata_instances.len() as u32,
            );

            renderer.queue.write_buffer(
                &render_world.strata_instances_buffer,
                0,
                bytemuck::cast_slice(&render_world.strata_instances),
            );
        }
    }

    fn queue(&mut self, context: &mut QueueContext) {
        let mut render_pass =
            context
                .frame
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("terrain_render_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &context.frame.surface,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: context.depth_buffer,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

        let render_world = context.render_world;

        // Strata
        {
            render_pass.set_pipeline(&self.strata_pipeline);
            render_pass.set_vertex_buffer(0, render_world.strata_instances_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.strata_vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                self.strata_index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.set_bind_group(0, &render_world.camera_env_bind_group, &[]);
            render_pass.set_bind_group(1, &self.terrain_bind_group, &[]);

            // TODO: Reduce draw calls?  Right now this ends up being a *ver* low number of
            //       instances.  Is it worth optimizing?

            const INDEX_START: [u32; 4] = [0, 72, 112, 136];

            for (i, strata_instance) in render_world.strata_instances.iter().enumerate() {
                let side = strata_instance.flags >> 8 & 0b11;
                let lod = strata_instance.lod;

                let stride = 2 + ((NewTerrain::CELLS_PER_CHUNK * 2) >> lod);
                let start = INDEX_START[lod as usize] + stride * side;

                let indices = start..(start + stride);
                let instances = (i as u32)..(i as u32 + 1);

                render_pass.draw_indexed(indices.clone(), 0, instances);
            }
        }

        // Terrain Chunks
        {
            render_pass.set_pipeline(&self.terrain_pipeline);
            render_pass.set_vertex_buffer(0, render_world.terrain_chunk_instances_buffer.slice(..));
            render_pass.set_index_buffer(
                self.chunk_indices_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.set_bind_group(0, &render_world.camera_env_bind_group, &[]);
            render_pass.set_bind_group(1, &self.terrain_bind_group, &[]);

            let draw_commands = Self::build_draw_commands(
                &render_world.terrain_chunk_instances,
                &Self::INDEX_RANGES,
            );

            for (indices, instances) in draw_commands {
                if instances.is_empty() {
                    continue;
                }
                render_pass.draw_indexed(indices, 0, instances);
            }
        }
    }
}

fn calculate_lod(
    camera_position: Vec3,
    camera_forward: Vec3,
    camera_far: f32,
    chunk_center: Vec3,
) -> u32 {
    let far = camera_far.max(1e-6);
    let inv_step = NewTerrain::LOD_MAX as f32 / far;

    let forward_distance = (chunk_center - camera_position)
        .dot(camera_forward)
        .max(0.0);

    let t = forward_distance * inv_step;

    (t.floor() as i32).clamp(0, (NewTerrain::LOD_MAX - 1) as i32) as u32
}

struct ChunkIndices {
    indices: Vec<u32>,
    wireframe_indices: Vec<u32>,
}

impl Default for ChunkIndices {
    fn default() -> Self {
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

        for level in 0..=NewTerrain::LOD_MAX {
            let cell_count = NewTerrain::CELLS_PER_CHUNK >> level;

            for y in 0..cell_count {
                for x in 0..cell_count {
                    let i0 = y * NewTerrain::NODES_PER_CHUNK + x;
                    let i1 = y * NewTerrain::NODES_PER_CHUNK + (x + 1);
                    let i2 = (y + 1) * NewTerrain::NODES_PER_CHUNK + (x + 1);
                    let i3 = (y + 1) * NewTerrain::NODES_PER_CHUNK + x;

                    indices.extend_from_slice(&[i0, i1, i2, i2, i3, i0]);
                    wireframe_indices.extend_from_slice(&[i0, i1, i1, i2, i2, i3, i3, i0]);
                }
            }
        }

        Self {
            indices,
            wireframe_indices,
        }
    }
}

#[derive(Clone, Copy, Debug, bytemuck::NoUninit)]
#[repr(C)]
struct StrataVertex {
    normal: [f32; 3],
    node_coord: [u32; 2],
}

fn generate_strata_vertices() -> Vec<StrataVertex> {
    let mut vertices = Vec::with_capacity((9 + 9 + 7 + 7) * 2);

    // South
    for x in 0..NewTerrain::NODES_PER_CHUNK {
        vertices.push(StrataVertex {
            normal: [0.0, -1.0, 0.0],
            node_coord: [x, 0],
        });
        vertices.push(StrataVertex {
            normal: [0.0, -1.0, 0.0],
            node_coord: [x, 0],
        });
    }

    // West
    for y in 0..NewTerrain::NODES_PER_CHUNK {
        vertices.push(StrataVertex {
            normal: [1.0, 0.0, 0.0],
            node_coord: [NewTerrain::CELLS_PER_CHUNK, y],
        });
        vertices.push(StrataVertex {
            normal: [1.0, 0.0, 0.0],
            node_coord: [NewTerrain::CELLS_PER_CHUNK, y],
        });
    }

    // North
    for x in 0..NewTerrain::NODES_PER_CHUNK {
        vertices.push(StrataVertex {
            normal: [0.0, 1.0, 0.0],
            node_coord: [NewTerrain::CELLS_PER_CHUNK - x, NewTerrain::CELLS_PER_CHUNK],
        });
        vertices.push(StrataVertex {
            normal: [0.0, 1.0, 0.0],
            node_coord: [NewTerrain::CELLS_PER_CHUNK - x, NewTerrain::CELLS_PER_CHUNK],
        });
    }

    // East
    for y in 0..NewTerrain::NODES_PER_CHUNK {
        vertices.push(StrataVertex {
            normal: [-1.0, 0.0, 0.0],
            node_coord: [0, NewTerrain::CELLS_PER_CHUNK - y],
        });
        vertices.push(StrataVertex {
            normal: [-1.0, 0.0, 0.0],
            node_coord: [0, NewTerrain::CELLS_PER_CHUNK - y],
        });
    }

    vertices
}

fn generate_strata_indices() -> Vec<u32> {
    let mut indices: Vec<u32> = Vec::with_capacity(1024);

    for lod in 0..4 {
        for side in 0..4 {
            let start = side * 18;
            let end = start + 18;
            indices.extend((start..end).step_by(2 << lod).flat_map(|i| [i, i + 1]));
        }
    }

    indices
}
