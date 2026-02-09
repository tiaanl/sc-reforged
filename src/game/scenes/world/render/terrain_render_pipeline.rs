use wgpu::util::DeviceExt;

use crate::{
    engine::{
        growing_buffer::GrowingBuffer,
        renderer::{Frame, Renderer},
        shader_cache::{ShaderCache, ShaderSource},
    },
    game::{
        AssetReader,
        scenes::world::{
            extract::{RenderSnapshot, TerrainChunk},
            render::{
                GeometryBuffer, RenderBindings, RenderLayouts,
                camera_render_pipeline::CameraEnvironmentLayout,
                per_frame::PerFrame,
                pipeline::{
                    BindGroupLayoutDescriptor, DepthState, FragmentState, PrimitiveState,
                    RenderPipelineDescriptor, VertexBufferLayout, VertexState,
                },
                render_pipeline::RenderPipeline,
            },
            sim_world::Terrain,
        },
    },
};

pub struct TerrainRenderPipeline {
    /// Buffer holding indices to render a single chunk at various LOD's.
    chunk_indices_buffer: wgpu::Buffer,
    /// Buffer holding indices to render a wireframe over a single chunk of various LOD's.
    chunk_wireframe_indices_buffer: wgpu::Buffer,

    /// Bind group for all terrain GPU resources.
    terrain_bind_group: wgpu::BindGroup,

    /// Pipeline to render the terrain chunks.
    terrain_pipeline: wgpu::RenderPipeline,

    /// Pipeline to render the terrain chunks as wireframe.
    terrain_wireframe_pipeline: wgpu::RenderPipeline,

    /// Buffer holding terrain chunk instance data for chunks to be rendered per frame.
    terrain_chunk_instances_buffer: PerFrame<GrowingBuffer<gpu::ChunkInstanceData>>,

    /// Pipeline to render the stratas.
    strata_pipeline: wgpu::RenderPipeline,

    /// Buffer holding instance data for strata to be rendered per frame.
    strata_instances_buffer: PerFrame<GrowingBuffer<gpu::ChunkInstanceData>>,
}

impl TerrainRenderPipeline {
    pub fn new(
        assets: &AssetReader,
        renderer: &Renderer,
        layouts: &mut RenderLayouts,
        shader_cache: &mut ShaderCache,
        sim_world: &bevy_ecs::world::World,
    ) -> Self {
        let terrain = sim_world.resource::<Terrain>();
        let height_map = &terrain.height_map;

        let cells_dim = height_map.size;
        let chunks_dim = cells_dim / Terrain::CELLS_PER_CHUNK;

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
            let image = assets.get_image(terrain.terrain_texture).unwrap();
            renderer.create_texture("terrain_texture", &image.data)
        };

        let strata_texture = {
            let image = assets
                .get_image(terrain.strata_texture)
                .expect("Could not load strate texture.");
            renderer.create_texture("strata", &image.data)
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

        let capacity = 1 << 7;
        let terrain_chunk_instances_buffer = PerFrame::new(|index| {
            GrowingBuffer::new(
                renderer,
                capacity,
                wgpu::BufferUsages::VERTEX,
                format!("terrain_chunk_instances:{index}"),
            )
        });

        let module = shader_cache.get_or_create(&renderer.device, ShaderSource::Terrain);

        let layout = renderer
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("terrain_pipeline_layout"),
                bind_group_layouts: &[
                    layouts.get::<CameraEnvironmentLayout>(renderer),
                    &terrain_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let instance_attrs = wgpu::vertex_attr_array![
            0 => Uint32x2,
            1 => Uint32,
            2 => Uint32,
        ];

        {
            let _terrain_pipeline_descriptor = RenderPipelineDescriptor {
                label: String::from("terrain"),
                layout: vec![BindGroupLayoutDescriptor {
                    label: String::from("terrain"),
                    entries: vec![],
                }],
                vertex: VertexState {
                    shader: ShaderSource::Terrain,
                    entry_point: String::from("vertex_terrain"),
                    buffers: vec![VertexBufferLayout {
                        array_stride: std::mem::size_of::<gpu::ChunkInstanceData>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: instance_attrs.to_vec(),
                    }],
                },
                primitive: PrimitiveState::default(),
                depth: Some(DepthState {
                    format: wgpu::TextureFormat::Depth32Float,
                    enabled: true,
                    compare: wgpu::CompareFunction::LessEqual,
                }),
                fragment: Some(FragmentState {
                    shader: ShaderSource::Terrain,
                    entry_point: String::from("fragment_terrain"),
                    targets: GeometryBuffer::opaque_targets().to_vec(),
                }),
            };
        }

        let terrain_pipeline =
            renderer
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("terrain_pipeline"),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module,
                        entry_point: Some("vertex_terrain"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<gpu::ChunkInstanceData>()
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
                        module,
                        entry_point: Some("fragment_terrain"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: GeometryBuffer::opaque_targets(),
                    }),
                    multiview: None,
                    cache: None,
                });

        let terrain_wireframe_pipeline =
            renderer
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("terrain_wireframe_pipeline"),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module,
                        entry_point: Some("vertex_wireframe"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<gpu::ChunkInstanceData>()
                                as wgpu::BufferAddress,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &instance_attrs,
                        }],
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::LineList,
                        cull_mode: None,
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: false,
                        depth_compare: wgpu::CompareFunction::LessEqual,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module,
                        entry_point: Some("fragment_wireframe"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: GeometryBuffer::opaque_targets(),
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
                        module,
                        entry_point: Some("strata_vertex"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<gpu::ChunkInstanceData>()
                                as wgpu::BufferAddress,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &instance_attrs,
                        }],
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleStrip,
                        cull_mode: Some(wgpu::Face::Back),
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
                        module,
                        entry_point: Some("strata_fragment"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: GeometryBuffer::opaque_targets(),
                    }),
                    multiview: None,
                    cache: None,
                });

        let capacity = 1 << 7;
        let strata_instances_buffer = PerFrame::new(|index| {
            GrowingBuffer::new(
                renderer,
                capacity,
                wgpu::BufferUsages::VERTEX,
                format!("strata_instances:{index}"),
            )
        });

        Self {
            chunk_indices_buffer,
            chunk_wireframe_indices_buffer,

            terrain_bind_group,

            terrain_pipeline,
            terrain_wireframe_pipeline,
            strata_pipeline,
            terrain_chunk_instances_buffer,

            strata_instances_buffer,
        }
    }
}

impl RenderPipeline for TerrainRenderPipeline {
    fn prepare(
        &mut self,
        _assets: &AssetReader,
        renderer: &Renderer,
        _bindings: &mut RenderBindings,
        snapshot: &RenderSnapshot,
    ) {
        let chunk_instances: Vec<_> = snapshot
            .terrain
            .chunks
            .iter()
            .map(|chunk| gpu::ChunkInstanceData {
                coord: chunk.coord.as_uvec2().to_array(),
                lod: chunk.lod,
                flags: chunk.flags,
            })
            .collect();

        let terrain_chunk_instances_buffer = self.terrain_chunk_instances_buffer.advance();
        terrain_chunk_instances_buffer.write(renderer, chunk_instances.as_slice());

        let strata_instances: Vec<_> = snapshot
            .terrain
            .strata
            .iter()
            .map(|chunk| gpu::ChunkInstanceData {
                coord: chunk.coord.as_uvec2().to_array(),
                lod: chunk.lod,
                flags: chunk.flags,
            })
            .collect();

        let strata_instances_buffer = self.strata_instances_buffer.advance();
        strata_instances_buffer.write(renderer, strata_instances.as_slice());
    }

    fn queue(
        &self,
        bindings: &RenderBindings,
        frame: &mut Frame,
        geometry_buffer: &GeometryBuffer,
        snapshot: &RenderSnapshot,
    ) {
        let mut render_pass =
            geometry_buffer.begin_opaque_render_pass(&mut frame.encoder, "terrain_render_pass");

        // Strata
        {
            render_pass.set_pipeline(&self.strata_pipeline);
            render_pass.set_vertex_buffer(0, self.strata_instances_buffer.current().slice(..));
            render_pass.set_bind_group(0, &bindings.camera_env_buffer.current().bind_group, &[]);
            render_pass.set_bind_group(1, &self.terrain_bind_group, &[]);

            for (i, strata_instance) in snapshot.terrain.strata.iter().enumerate() {
                let lod = strata_instance.lod;

                // Cells along this edge at this LOD.
                let cells = Terrain::CELLS_PER_CHUNK >> lod;
                let nodes = cells + 1;

                // 2 vertices per node: bottom + top
                let vertex_count = 2 * nodes;

                let vertices = 0..vertex_count;
                let instances = (i as u32)..(i as u32 + 1);

                render_pass.draw(vertices, instances);
            }
        }

        // Terrain Chunks
        {
            render_pass.set_pipeline(&self.terrain_pipeline);
            render_pass
                .set_vertex_buffer(0, self.terrain_chunk_instances_buffer.current().slice(..));
            render_pass.set_index_buffer(
                self.chunk_indices_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.set_bind_group(0, &bindings.camera_env_buffer.current().bind_group, &[]);
            render_pass.set_bind_group(1, &self.terrain_bind_group, &[]);

            let draw_commands =
                Self::build_draw_commands(&snapshot.terrain.chunks, &Self::INDEX_RANGES);

            for (indices, instances) in draw_commands {
                if instances.is_empty() {
                    continue;
                }
                render_pass.draw_indexed(indices, 0, instances);
            }
        }

        if snapshot.terrain.render_wireframe {
            render_pass.set_pipeline(&self.terrain_wireframe_pipeline);
            render_pass
                .set_vertex_buffer(0, self.terrain_chunk_instances_buffer.current().slice(..));
            render_pass.set_index_buffer(
                self.chunk_wireframe_indices_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.set_bind_group(0, &bindings.camera_env_buffer.current().bind_group, &[]);
            render_pass.set_bind_group(1, &self.terrain_bind_group, &[]);

            let draw_commands =
                Self::build_draw_commands(&snapshot.terrain.chunks, &Self::WIREFRAME_INDEX_RANGES);

            for (indices, instances) in draw_commands {
                if instances.is_empty() {
                    continue;
                }
                render_pass.draw_indexed(indices, 0, instances);
            }
        }
    }
}

impl TerrainRenderPipeline {
    const STRATA_DESCENT: f32 = -20_000.0;

    const INDEX_RANGES: [std::ops::Range<u32>; 4] = [0..384, 384..480, 480..504, 504..510];
    const WIREFRAME_INDEX_RANGES: [std::ops::Range<u32>; 4] =
        [0..512, 512..640, 640..672, 672..680];
}

impl TerrainRenderPipeline {
    /// Build a list of instances per LOD.
    /// `chunk_instances` *must* be sorted by LOD.
    fn build_draw_commands(
        chunk_instances: &[TerrainChunk],
        ranges: &[std::ops::Range<u32>],
    ) -> [(std::ops::Range<u32>, std::ops::Range<u32>); Terrain::LOD_COUNT as usize] {
        // TODO: This is probably not needed.
        debug_assert!(chunk_instances.is_sorted_by_key(|instance| instance.lod));

        let mut counts = [0_u32; Terrain::LOD_COUNT as usize];

        // Count the number of each LOD.
        for instance in chunk_instances {
            let lod = instance.lod.min(Terrain::LOD_MAX) as usize;
            counts[lod] += 1;
        }

        // Create starting indices by accumulating the LOD counts.
        let mut offsets = [0_u32; Terrain::LOD_COUNT as usize];
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

        for level in 0..=Terrain::LOD_MAX {
            let cell_count = Terrain::CELLS_PER_CHUNK >> level;

            for y in 0..cell_count {
                for x in 0..cell_count {
                    let i0 = y * Terrain::NODES_PER_CHUNK + x;
                    let i1 = y * Terrain::NODES_PER_CHUNK + (x + 1);
                    let i2 = (y + 1) * Terrain::NODES_PER_CHUNK + (x + 1);
                    let i3 = (y + 1) * Terrain::NODES_PER_CHUNK + x;

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

pub mod gpu {
    use bytemuck::NoUninit;

    #[derive(Clone, Copy, Default, NoUninit)]
    #[repr(C)]
    pub struct ChunkInstanceData {
        pub coord: [u32; 2],
        pub lod: u32,
        pub flags: u32,
    }
}
