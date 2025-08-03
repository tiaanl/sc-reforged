use std::path::PathBuf;

use glam::UVec2;
use wgpu::util::DeviceExt;

use crate::engine::prelude::*;
use crate::game::data_dir::data_dir;
use crate::game::geometry_buffers::GeometryBuffers;
use crate::game::scenes::world::terrain::Terrain;

pub struct Strata {
    mesh: GpuIndexedMesh,
    instances_buffer: wgpu::Buffer,
    instances_count: u32,
    bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
}

impl Strata {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        shaders: &mut Shaders,
        terrain_size: UVec2,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        height_map_buffer: &wgpu::Buffer,
        terrain_buffer: &wgpu::Buffer,
    ) -> Result<Self, AssetError> {
        let renderer = renderer();

        let mesh = {
            let mut mesh = IndexedMesh::default();

            macro_rules! v {
                ($x:expr, $y:expr, $z:expr, $nx:expr, $ny:expr, $nz:expr, $tx:expr, $ty:expr, $vx:expr, $vy:expr) => {{
                    mesh.vertices.push(StrataVertex {
                        position: Vec3::new($x, $y, $z),
                        normal: Vec3::new($nx, $ny, $nz),
                        tex_coord: Vec2::new($tx, $ty),
                        vertex_index: UVec2::new($vx, $vy),
                    })
                }};
            }

            // -y
            for v in 0..=8 {
                let vv = v as f32;
                v!(vv, 0.0, 0.0, 0.0, -1.0, 0.0, vv / 8.0, 0.0, v, 0);
                v!(vv, 0.0, 1.0, 0.0, -1.0, 0.0, vv / 8.0, 1.0, v, 0);
            }

            // x
            for v in 0..=8 {
                let vv = v as f32;
                v!(8.0, vv, 0.0, 1.0, 0.0, 0.0, vv / 8.0, 0.0, 8, v);
                v!(8.0, vv, 1.0, 1.0, 0.0, 0.0, vv / 8.0, 1.0, 8, v);
            }

            // // y
            for v in 0..=8 {
                let vv = v as f32;
                v!(8.0 - vv, 8.0, 0.0, 0.0, 1.0, 0.0, vv / 8.0, 0.0, 8 - v, 8);
                v!(8.0 - vv, 8.0, 1.0, 0.0, 1.0, 0.0, vv / 8.0, 1.0, 8 - v, 8);
            }

            // // -x
            for v in 0..=8 {
                let vv = v as f32;
                v!(0.0, 8.0 - vv, 0.0, -1.0, 0.0, 0.0, vv / 8.0, 0.0, 0, 8 - v);
                v!(0.0, 8.0 - vv, 1.0, -1.0, 0.0, 0.0, vv / 8.0, 1.0, 0, 8 - v);
            }

            for s in 0..4 {
                let start = s * 8 + s;
                for i in start..(start + 8) {
                    let i = i * 2;
                    mesh.indices
                        .extend_from_slice(&[i, i + 1, i + 2, i + 2, i + 1, i + 3]);
                }
            }

            mesh.to_gpu()
        };

        let (instances_buffer, instances_count) = {
            let chunks = terrain_size / UVec2::splat(Terrain::CELLS_PER_CHUNK);
            let mut instances = Vec::default();
            for x in 0..chunks.x {
                instances.push(UVec2::new(x, 0));
                instances.push(UVec2::new(x, chunks.y - 1));
            }
            for y in 0..chunks.y {
                instances.push(UVec2::new(0, y));
                instances.push(UVec2::new(chunks.x - 1, y));
            }

            (
                renderer
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("strata_instances_buffer"),
                        contents: bytemuck::cast_slice(&instances),
                        usage: wgpu::BufferUsages::VERTEX,
                    }),
                instances.len() as u32,
            )
        };

        let texture_view = {
            let path = PathBuf::from("textures").join("shared").join("strata.bmp");
            tracing::info!("Loading strata texture: {}", path.display());

            let image = data_dir().load_image(&path)?;
            renderer.create_texture_view("terrain_strata", &image.data)
        };

        let bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("strata_bind_group_layout"),
                    entries: &[
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
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
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

        let sampler = renderer.create_sampler(
            "strata",
            wgpu::AddressMode::Repeat,
            wgpu::FilterMode::Linear,
            wgpu::FilterMode::Linear,
        );

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("strata_bind_group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: height_map_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: terrain_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });

        let module = shaders.create_shader(
            "strata",
            include_str!("strata.wgsl"),
            "strata.wgsl",
            Default::default(),
        );

        let pipeline_layout =
            renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("strata_render_pipeline_layout"),
                    bind_group_layouts: &[camera_bind_group_layout, &bind_group_layout],
                    push_constant_ranges: &[],
                });

        let render_pipeline =
            renderer
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("strata_render_pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: None,
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[
                            StrataVertex::layout(),
                            wgpu::VertexBufferLayout {
                                array_stride: std::mem::size_of::<UVec2>() as wgpu::BufferAddress,
                                step_mode: wgpu::VertexStepMode::Instance,
                                attributes: &wgpu::vertex_attr_array![
                                    4 => Uint32x2,
                                ],
                            },
                        ],
                    },
                    primitive: wgpu::PrimitiveState {
                        cull_mode: Some(wgpu::Face::Back),
                        ..Default::default()
                    },
                    depth_stencil: Some(GeometryBuffers::depth_stencil_state(
                        wgpu::CompareFunction::LessEqual,
                        true,
                    )),
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: None,
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: GeometryBuffers::opaque_targets(),
                    }),
                    multiview: None,
                    cache: None,
                });

        Ok(Self {
            mesh,
            instances_buffer,
            instances_count,
            bind_group,
            render_pipeline,
        })
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        geometry_buffers: &GeometryBuffers,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("strata_render_pass"),
                color_attachments: &geometry_buffers.opaque_color_attachments(),
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

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_index_buffer(self.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_vertex_buffer(0, self.mesh.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instances_buffer.slice(..));
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.bind_group, &[]);
        render_pass.draw_indexed(0..self.mesh.index_count, 0, 0..self.instances_count);
    }
}

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
struct StrataVertex {
    position: Vec3,
    normal: Vec3,
    tex_coord: Vec2,
    vertex_index: UVec2,
}

impl BufferLayout for StrataVertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x3,
            2 => Float32x2,
            3 => Uint32x2,
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<StrataVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: ATTRS,
        }
    }
}
