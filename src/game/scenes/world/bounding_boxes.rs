use glam::{vec2, vec3, Mat4};
use wgpu::{util::DeviceExt, vertex_attr_array};

use crate::{engine::prelude::*, game::camera::Ray};

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
pub struct RawBoundingBox {
    transform: Mat4,
    min: Vec3,
    highlight: u32,
    max: Vec3,
    _padding: u32,
}

impl RawBoundingBox {
    pub fn new(transform: Mat4, min: Vec3, max: Vec3, highlight: bool) -> Self {
        Self {
            transform,
            min,
            highlight: if highlight { 1 } else { 0 },
            max,
            _padding: 0,
        }
    }
}

impl RawBoundingBox {
    pub fn intersect_ray(&self, ray: &Ray) -> Option<f32> {
        // Compute inverse transform
        let inverse_transform = self.transform.inverse();

        // Transform ray into the box's local space
        let local_origin = inverse_transform.transform_point3(ray.origin);
        let local_direction = inverse_transform.transform_vector3(ray.direction);

        // Ray-AABB intersection
        let mut t_min = 0.0_f32;
        let mut t_max = f32::MAX;

        for i in 0..3 {
            let o = local_origin[i];
            let d = local_direction[i];
            let min_val = self.min[i];
            let max_val = self.max[i];

            if d.abs() < 1e-8 {
                // The ray is parallel to this axis
                if o < min_val || o > max_val {
                    // No intersection
                    return None;
                }
            } else {
                let inv_d = 1.0 / d;
                let mut t0 = (min_val - o) * inv_d;
                let mut t1 = (max_val - o) * inv_d;

                if t0 > t1 {
                    std::mem::swap(&mut t0, &mut t1);
                }

                // Narrow down the intersection range
                t_min = t_min.max(t0);
                t_max = t_max.min(t1);

                if t_max < t_min {
                    // No intersection
                    return None;
                }
            }
        }

        // t_min is the first point of intersection
        if t_min < 0.0 && t_max < 0.0 {
            // Both intersections are behind the origin
            None
        } else if t_min < 0.0 {
            // Origin is inside the box, intersection going outward at t_max
            Some(t_max)
        } else {
            Some(t_min)
        }
    }
}

pub struct BoundingBoxRenderer {
    pipeline: wgpu::RenderPipeline,
    index_buffer: wgpu::Buffer,
    wireframe_pipeline: wgpu::RenderPipeline,
    wireframe_index_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
}

impl BoundingBoxRenderer {
    const INDICES: &[u32] = &[
        0, 1, 2, 2, 3, 0, //
        4, 5, 6, 6, 7, 4, //
        8, 9, 10, 10, 11, 8, //
        12, 13, 14, 14, 15, 12, //
        16, 17, 18, 18, 19, 16, //
        20, 21, 22, 22, 23, 20, //
    ];

    const WIREFRAME_INDICES: &[u32] = &[
        0, 1, 1, 2, 2, 3, 3, 0, // front face
        4, 5, 5, 6, 6, 7, 7, 4, // back face
        0, 4, 1, 7, 2, 6, 3, 5, // connecting edges
    ];

    const VERTICES: &[Vertex] = {
        macro_rules! v {
            ($x:expr,$y:expr,$z:expr,$nx:expr,$ny:expr,$nz:expr,$u:expr,$v:expr) => {{
                Vertex {
                    position: vec3($x, $y, $z),
                    normal: vec3($nx, $ny, $nz),
                    tex_coord: vec2($u, $v),
                }
            }};
        }

        const MIN: f32 = -0.5;
        const MAX: f32 = 0.5;

        &[
            // Bottom face
            v!(MIN, MIN, MIN, 0.0, 0.0, -1.0, 0.0, 0.0),
            v!(MIN, MAX, MIN, 0.0, 0.0, -1.0, 0.0, 1.0),
            v!(MAX, MAX, MIN, 0.0, 0.0, -1.0, 1.0, 1.0),
            v!(MAX, MIN, MIN, 0.0, 0.0, -1.0, 1.0, 0.0),
            // Top face
            v!(MIN, MIN, MAX, 0.0, 0.0, 1.0, 0.0, 0.0),
            v!(MAX, MIN, MAX, 0.0, 0.0, 1.0, 1.0, 0.0),
            v!(MAX, MAX, MAX, 0.0, 0.0, 1.0, 1.0, 1.0),
            v!(MIN, MAX, MAX, 0.0, 0.0, 1.0, 0.0, 1.0),
            // Front
            v!(MIN, MIN, MIN, 0.0, -1.0, 0.0, 0.0, 0.0),
            v!(MAX, MIN, MIN, 0.0, -1.0, 0.0, 1.0, 0.0),
            v!(MAX, MIN, MAX, 0.0, -1.0, 0.0, 1.0, 1.0),
            v!(MIN, MIN, MAX, 0.0, -1.0, 0.0, 0.0, 1.0),
            // Back
            v!(MIN, MAX, MIN, 0.0, 1.0, 0.0, 0.0, 0.0),
            v!(MIN, MAX, MAX, 0.0, 1.0, 0.0, 0.0, 1.0),
            v!(MAX, MAX, MAX, 0.0, 1.0, 0.0, 1.0, 1.0),
            v!(MAX, MAX, MIN, 0.0, 1.0, 0.0, 1.0, 0.0),
            // Left
            v!(MIN, MIN, MIN, -1.0, 0.0, 0.0, 0.0, 0.0),
            v!(MIN, MIN, MAX, -1.0, 0.0, 0.0, 0.0, 1.0),
            v!(MIN, MAX, MAX, -1.0, 0.0, 0.0, 1.0, 1.0),
            v!(MIN, MAX, MIN, -1.0, 0.0, 0.0, 1.0, 0.0),
            // Right
            v!(MAX, MIN, MIN, 1.0, 0.0, 0.0, 0.0, 0.0),
            v!(MAX, MAX, MIN, 1.0, 0.0, 0.0, 1.0, 0.0),
            v!(MAX, MAX, MAX, 1.0, 0.0, 0.0, 1.0, 1.0),
            v!(MAX, MIN, MAX, 1.0, 0.0, 0.0, 0.0, 1.0),
        ]
    };

    pub fn new(renderer: &Renderer, camera_bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        let shader = renderer.create_shader_module(
            "bounding_boxes_shader_module",
            include_str!("bounding_boxes.wgsl"),
        );

        let pipeline_layout =
            renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("bounding_box_renderer_render_pipeline_layout"),
                    bind_group_layouts: &[camera_bind_group_layout],
                    push_constant_ranges: &[],
                });

        macro_rules! create_render_pipeline {
            ($renderer:expr, $fragment_entry:literal, $primitive:expr) => {{
                renderer
                    .device
                    .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("bounding_box_renderer_render_pipeline"),
                        layout: Some(&pipeline_layout),
                        vertex: wgpu::VertexState {
                            module: &shader,
                            entry_point: None,
                            compilation_options: wgpu::PipelineCompilationOptions::default(),
                            buffers: &[
                                Vertex::vertex_buffer_layout(),
                                wgpu::VertexBufferLayout {
                                    array_stride: std::mem::size_of::<RawBoundingBox>() as wgpu::BufferAddress,
                                    step_mode: wgpu::VertexStepMode::Instance,
                                    attributes: &vertex_attr_array![
                                        10 => Float32x4,  // model_mat_0
                                        11 => Float32x4,  // model_mat_1
                                        12 => Float32x4,  // model_mat_2
                                        13 => Float32x4,  // model_mat_3
                                        14 => Float32x3,  // min
                                        15 => Uint32,     // highlight
                                        16 => Float32x3,  // max
                                        17 => Uint32,     // padding
                                    ],
                                },
                            ],
                        },
                        primitive: $primitive,
                        depth_stencil: None,
                        multisample: wgpu::MultisampleState::default(),
                        fragment: Some(wgpu::FragmentState {
                            module: &shader,
                            entry_point: Some($fragment_entry),
                            compilation_options: wgpu::PipelineCompilationOptions::default(),
                            targets: &[Some(wgpu::ColorTargetState {
                                format: renderer.surface_config.format,
                                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                                write_mask: wgpu::ColorWrites::ALL,
                            })],
                        }),
                        multiview: None,
                        cache: None,
                    })
            }};
        }

        let pipeline = create_render_pipeline!(
            renderer,
            "fragment_main",
            wgpu::PrimitiveState {
                front_face: wgpu::FrontFace::Cw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            }
        );

        let index_buffer = renderer.create_index_buffer("bounding_box_index_buffer", Self::INDICES);

        let wireframe_pipeline = create_render_pipeline!(
            renderer,
            "fragment_main_wireframe",
            wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            }
        );

        let wireframe_index_buffer =
            renderer.create_index_buffer("bounding_box_index_buffer", Self::WIREFRAME_INDICES);

        let vertex_buffer =
            renderer.create_vertex_buffer("bounding_boxes_vertex_buffer", Self::VERTICES);

        Self {
            pipeline,
            index_buffer,
            wireframe_pipeline,
            wireframe_index_buffer,
            vertex_buffer,
        }
    }

    pub fn render_all(
        &self,
        frame: &mut Frame,
        camera_bind_group: &wgpu::BindGroup,
        bounding_boxes: &[RawBoundingBox],
    ) {
        let device = frame.device.clone();

        let mut render_pass =
            frame.begin_basic_render_pass("bounding_box_renderer_render_pass", false);

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bounding_box_renderer_transforms"),
            contents: bytemuck::cast_slice(bounding_boxes),
            usage: wgpu::BufferUsages::VERTEX,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, buffer.slice(..));
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.draw_indexed(
            0..Self::INDICES.len() as u32,
            0,
            0..bounding_boxes.len() as u32,
        );

        render_pass.set_pipeline(&self.wireframe_pipeline);
        render_pass.set_index_buffer(
            self.wireframe_index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, buffer.slice(..));
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.draw_indexed(
            0..Self::WIREFRAME_INDICES.len() as u32,
            0,
            0..bounding_boxes.len() as u32,
        );
    }
}
