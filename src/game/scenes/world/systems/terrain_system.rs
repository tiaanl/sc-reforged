use glam::{UVec2, uvec2};
use wgpu::util::DeviceExt;

use crate::{
    game::{
        image::images,
        scenes::world::{
            new_terrain::NewTerrain,
            render_world::{ChunkInstanceData, RenderWorld},
            systems::{
                ExtractContext, NewSystemContext, PreUpdateContext, PrepareContext, QueueContext,
                System,
            },
        },
    },
    wgsl_shader,
};

pub struct TerrainSystem {
    /// Dimensions of the terrain in chunks.
    chunks_dim: UVec2,

    /// Buffer holding indices to render a single chunk at various LOD's.
    chunk_indices_buffer: wgpu::Buffer,
    /// Buffer holding indices to render a wireframe over a single chunk of various LOD's.
    _chunk_wireframe_indices_buffer: wgpu::Buffer,
    /// Bind group for all terrain rendering resources.
    terrain_bind_group: wgpu::BindGroup,
    /// Pipeline to render the terrain chunks.
    pipeline: wgpu::RenderPipeline,
}

impl TerrainSystem {
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
                _pad: [u32; 3],
            }

            let terrain_data = TerrainData {
                cells_dim: cells_dim.to_array(),
                chunks_dim: chunks_dim.to_array(),
                cell_size: height_map.cell_size,
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

        let terrain_sampler = renderer.create_sampler(
            "terrain_sampler",
            wgpu::AddressMode::ClampToEdge,
            wgpu::FilterMode::Linear,
            wgpu::FilterMode::Linear,
        );

        let terrain_bind_group_layout =
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
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let terrain_bind_group = renderer
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
                        resource: wgpu::BindingResource::Sampler(&terrain_sampler),
                    },
                ],
            });

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

        let pipeline = {
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
                    bind_group_layouts: &[camera_bind_group_layout, &terrain_bind_group_layout],
                    push_constant_ranges: &[],
                });

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
                            attributes: &wgpu::vertex_attr_array![
                                0 => Uint32x2,
                                1 => Uint32,
                                2 => Uint32,
                            ],
                        }],
                    },
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
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
                })
        };

        Self {
            chunks_dim,

            chunk_indices_buffer,
            _chunk_wireframe_indices_buffer: chunk_wireframe_indices_buffer,

            terrain_bind_group,

            pipeline,
        }
    }
}

impl System for TerrainSystem {
    fn extract(&mut self, context: &mut ExtractContext) {
        context.render_world.terrain_chunk_instances = context
            .sim_world
            .visible_chunks
            .iter()
            .map(|coord| ChunkInstanceData {
                coord: coord.to_array(),
                lod: 0,
                flags: 0,
            })
            .collect();
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
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

        let render_world = context.render_world;

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, render_world.terrain_chunk_instances_buffer.slice(..));
        render_pass.set_index_buffer(
            self.chunk_indices_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.set_bind_group(0, &render_world.camera_env_bind_group, &[]);
        render_pass.set_bind_group(1, &self.terrain_bind_group, &[]);

        let vertex_count: u32 = 384;
        let instance_count: u32 = render_world.terrain_chunk_instances.len() as u32;
        render_pass.draw_indexed(0..vertex_count, 0, 0..instance_count);
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
