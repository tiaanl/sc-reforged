use glam::Vec3;
use wgpu::util::DeviceExt;

use crate::{
    engine::{
        growing_buffer::GrowingBuffer,
        renderer::{Frame, Renderer},
        transform::Transform,
    },
    game::scenes::world::render::{GeometryBuffer, RenderStore, RenderWorld, pipeline::Pipeline},
    wgsl_shader,
};

pub struct RenderBox {
    pub transform: Transform,
    pub min: Vec3,
    pub max: Vec3,
}

#[derive(Default)]
pub struct BoxRenderSnapshot {
    pub boxes: Vec<RenderBox>,
}

impl BoxRenderSnapshot {
    pub fn clear(&mut self) {
        self.boxes.clear();
    }
}

pub struct BoxPipeline {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    instance_buffer: GrowingBuffer<gpu::Instance>,
    pipeline: wgpu::RenderPipeline,

    /// Cache for converting snapshot [Box] to [gpu::Instance]s
    instances_cache: Vec<gpu::Instance>,
}

impl BoxPipeline {
    pub fn new(renderer: &Renderer, render_store: &mut RenderStore) -> Self {
        let (vertex_buffer, index_buffer, index_count) = Self::create_box_mesh(renderer);

        let instance_buffer =
            GrowingBuffer::new(renderer, 256, wgpu::BufferUsages::VERTEX, "box_instances");

        let module = renderer.device.create_shader_module(wgsl_shader!("box"));

        let pipeline_layout =
            renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("box_pipeline_layout"),
                    bind_group_layouts: &[&render_store.camera_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = renderer
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("box_render_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[
                        wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<gpu::Vertex>() as wgpu::BufferAddress,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &wgpu::vertex_attr_array![
                                0 => Float32x3,  // position
                                1 => Float32x2,  // tex_coord
                            ],
                        },
                        wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<gpu::Instance>()
                                as wgpu::BufferAddress,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &wgpu::vertex_attr_array![
                                2 => Float32x4,  // transform[0]
                                3 => Float32x4,  // transform[1]
                                4 => Float32x4,  // transform[2]
                                5 => Float32x4,  // transform[3]
                                6 => Float32x4,  // min
                                7 => Float32x4,  // max
                            ],
                        },
                    ],
                },
                primitive: wgpu::PrimitiveState {
                    front_face: wgpu::FrontFace::Cw,
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
                    module: &module,
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: GeometryBuffer::alpha_targets(),
                }),
                multiview: None,
                cache: None,
            });

        Self {
            vertex_buffer,
            index_buffer,
            index_count,
            instance_buffer,
            pipeline,

            instances_cache: Vec::default(),
        }
    }
}

impl Pipeline for BoxPipeline {
    type Snapshot = BoxRenderSnapshot;

    fn prepare(
        &mut self,
        renderer: &Renderer,
        _render_store: &mut RenderStore,
        _render_world: &mut RenderWorld,
        snapshot: &Self::Snapshot,
    ) {
        self.instances_cache.clear();
        self.instances_cache
            .extend(snapshot.boxes.iter().map(|b| gpu::Instance {
                transform: b.transform.to_mat4().to_cols_array_2d(),
                min: b.min.extend(1.0).to_array(),
                max: b.max.extend(1.0).to_array(),
            }));

        self.instance_buffer.write(renderer, &self.instances_cache);
    }

    fn queue(
        &self,
        _render_store: &RenderStore,
        render_world: &RenderWorld,
        frame: &mut Frame,
        geometry_buffer: &GeometryBuffer,
        _snapshot: &Self::Snapshot,
    ) {
        let mut render_pass =
            geometry_buffer.begin_alpha_render_pass(&mut frame.encoder, "box_render_pass");

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &render_world.camera_env_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.index_count, 0, 0..self.instance_buffer.count);
    }
}

impl BoxPipeline {
    fn create_box_mesh(renderer: &Renderer) -> (wgpu::Buffer, wgpu::Buffer, u32) {
        const VERTICES: &[[f32; 5]] = &[
            [1.0, 1.0, 1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0, 1.0, 0.0],
            [1.0, 0.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 1.0, 1.0, 0.0],
            [0.0, 0.0, 1.0, 1.0, 1.0],
            [0.0, 0.0, 0.0, 0.0, 1.0],
            [0.0, 0.0, 0.0, 1.0, 1.0],
            [0.0, 0.0, 1.0, 0.0, 1.0],
            [0.0, 1.0, 1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0, 1.0, 1.0],
            [1.0, 0.0, 0.0, 0.0, 1.0],
            [0.0, 0.0, 0.0, 0.0, 0.0],
            [1.0, 1.0, 0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0, 1.0, 0.0],
            [1.0, 0.0, 1.0, 1.0, 1.0],
            [1.0, 0.0, 0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0, 1.0, 1.0],
            [0.0, 1.0, 1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0, 1.0, 0.0],
        ];

        const INDICES: &[u32] = &[
            0, 1, 2, 2, 3, 0, //
            4, 5, 6, 6, 7, 4, //
            8, 9, 10, 10, 11, 8, //
            12, 13, 14, 14, 15, 12, //
            16, 17, 18, 18, 19, 16, //
            20, 21, 22, 22, 23, 20, //
        ];

        let vertex_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("box_vertex_buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("box_index_buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsages::INDEX,
            });

        (vertex_buffer, index_buffer, INDICES.len() as u32)
    }
}

pub mod gpu {
    use bytemuck::NoUninit;

    #[derive(Clone, Copy, NoUninit)]
    #[repr(C)]
    pub struct Vertex {
        position: [f32; 3],
        tex_coord: [f32; 2],
    }

    #[derive(Clone, Copy, NoUninit)]
    #[repr(C)]
    pub struct Instance {
        pub transform: [[f32; 4]; 4],
        pub min: [f32; 4],
        pub max: [f32; 4],
    }
}
