use glam::{Mat4, Vec3, Vec4};
use wgpu::{util::DeviceExt, vertex_attr_array};

use crate::{
    Frame,
    engine::{prelude::renderer, shaders::Shaders},
};

use super::renderer::BufferLayout;

#[derive(Clone, Copy, Debug, bytemuck::NoUninit)]
#[repr(C)]
pub struct GizmoVertex {
    pub position: Vec3,
    _padding: f32,
    pub color: Vec4,
}

impl GizmoVertex {
    pub fn new(position: Vec3, color: Vec4) -> Self {
        Self {
            position,
            _padding: 1.0,
            color,
        }
    }
}

impl BufferLayout for GizmoVertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        const VERTEX_ATTR_ARRAY: &[wgpu::VertexAttribute] = &vertex_attr_array![
            0 => Float32x4, // position
            1 => Float32x4, // color
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: VERTEX_ATTR_ARRAY,
        }
    }
}

pub struct GizmosRenderer {
    pipeline: wgpu::RenderPipeline,
}

impl GizmosRenderer {
    pub fn new(shaders: &mut Shaders, camera_bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        let renderer = renderer();

        let module = shaders.create_shader(
            "gizmos",
            include_str!("gizmos.wgsl"),
            "gizmos.wgsl",
            Default::default(),
        );

        let layout = renderer
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("gizmos_pipeline_layout"),
                bind_group_layouts: &[camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = renderer
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("gizmos_render_pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[GizmoVertex::layout()],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: None,
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

        Self { pipeline }
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        camera_bind_group: &wgpu::BindGroup,
        vertices: &[GizmoVertex],
    ) {
        if vertices.is_empty() {
            return;
        }

        let vertex_buffer =
            renderer()
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("gizmos_vertex_buffer"),
                    contents: bytemuck::cast_slice(vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gizmos_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.surface,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.draw(0..(vertices.len() as u32), 0..1);
    }

    pub fn create_axis(transform: Mat4, size: f32) -> Vec<GizmoVertex> {
        let zero = transform.project_point3(Vec3::ZERO);
        vec![
            GizmoVertex::new(zero, Vec4::new(1.0, 0.0, 0.0, 1.0)),
            GizmoVertex::new(
                transform.project_point3(Vec3::X * size),
                Vec4::new(1.0, 0.0, 0.0, 1.0),
            ),
            GizmoVertex::new(zero, Vec4::new(0.0, 1.0, 0.0, 1.0)),
            GizmoVertex::new(
                transform.project_point3(Vec3::Y * size),
                Vec4::new(0.0, 1.0, 0.0, 1.0),
            ),
            GizmoVertex::new(zero, Vec4::new(0.0, 0.0, 1.0, 1.0)),
            GizmoVertex::new(
                transform.project_point3(Vec3::Z * size),
                Vec4::new(0.0, 0.0, 1.0, 1.0),
            ),
        ]
    }

    pub fn create_iso_sphere(transform: Mat4, radius: f32, resolution: i32) -> Vec<GizmoVertex> {
        let mut vertices = Vec::new();
        let res = resolution.max(3);

        // Each axis defines the normal of the circle's plane.
        // For each axis, we need to pick two orthogonal vectors to define the circle.
        let axes = [
            (Vec3::Y, Vec3::Z, Vec4::new(1.0, 0.0, 0.0, 1.0)), // X: YZ plane (red)
            (Vec3::Z, Vec3::X, Vec4::new(0.0, 1.0, 0.0, 1.0)), // Y: ZX plane (green)
            (Vec3::X, Vec3::Y, Vec4::new(0.0, 0.5, 1.0, 1.0)), // Z: XY plane (blue)
        ];

        for (u, v, color) in axes {
            for i in 0..res {
                let theta0 = (i as f32) * std::f32::consts::TAU / (res as f32);
                let theta1 = ((i + 1) as f32) * std::f32::consts::TAU / (res as f32);

                let p0 = transform.transform_point3((u * theta0.cos() + v * theta0.sin()) * radius);
                let p1 = transform.transform_point3((u * theta1.cos() + v * theta1.sin()) * radius);

                vertices.push(GizmoVertex::new(p0, color));
                vertices.push(GizmoVertex::new(p1, color));
            }
        }

        vertices
    }
}
