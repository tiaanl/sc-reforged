use glam::{vec2, vec3, Mat4, Quat, Vec3, Vec4};

use crate::engine::{
    mesh::Vertex,
    renderer::{RenderPipelineConfig, Renderer},
};

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
struct BoundingBox {
    model_matrix: Mat4,
    min: Vec4,
    max: Vec4,
}

pub struct BoundingBoxes {
    pipeline: wgpu::RenderPipeline,
    index_buffer: wgpu::Buffer,
    wireframe_pipeline: wgpu::RenderPipeline,
    wireframe_index_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,

    boxes: Vec<BoundingBox>,
}

impl BoundingBoxes {
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

    pub fn new(renderer: &Renderer) -> Result<Self, ()> {
        let shader = renderer.create_shader_module(
            "bounding_boxes_shader_module",
            include_str!("bounding_boxes.wgsl"),
        );

        let pipeline = renderer.create_render_pipeline(
            RenderPipelineConfig::<Vertex>::new("bounding_boxes_render_pipeline", &shader)
                .primitive(wgpu::PrimitiveState {
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                })
                .blend_state(wgpu::BlendState::ALPHA_BLENDING)
                .bind_group_layout(renderer.uniform_bind_group_layout())
                .bind_group_layout(renderer.uniform_bind_group_layout()),
        );

        let index_buffer = renderer.create_index_buffer("bounding_box_index_buffer", Self::INDICES);

        let wireframe_pipeline = renderer.create_render_pipeline(
            RenderPipelineConfig::<Vertex>::new("bounding_boxes_render_pipeline", &shader)
                .fragment_entry("fragment_main_wireframe")
                .primitive(wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                })
                .blend_state(wgpu::BlendState::ALPHA_BLENDING)
                .bind_group_layout(renderer.uniform_bind_group_layout())
                .bind_group_layout(renderer.uniform_bind_group_layout()),
        );

        let wireframe_index_buffer =
            renderer.create_index_buffer("bounding_box_index_buffer", Self::WIREFRAME_INDICES);

        let vertex_buffer =
            renderer.create_vertex_buffer("bounding_boxes_vertex_buffer", Self::VERTICES);

        Ok(Self {
            pipeline,
            index_buffer,
            wireframe_pipeline,
            wireframe_index_buffer,

            vertex_buffer,

            boxes: vec![],
        })
    }

    pub fn insert(&mut self, position: Vec3, rotation: Quat, min: Vec3, max: Vec3) {
        let model_matrix = Mat4::from_rotation_translation(rotation, position);
        self.boxes.push(BoundingBox {
            model_matrix,
            min: Vec4::from((min, 0.0)),
            max: Vec4::from((max, 0.0)),
        });
    }

    pub fn render_all(
        &self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("bounding_boxes_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: renderer
                .render_pass_depth_stencil_attachment(wgpu::LoadOp::Load),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        let bind_groups = self
            .boxes
            .iter()
            .map(|b| {
                let buffer = renderer.create_uniform_buffer("bounding_box_buffer", *b);
                renderer.create_uniform_bind_group("bounding_box_bind_group", &buffer)
            })
            .collect::<Vec<_>>();

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        for bind_group in bind_groups.iter() {
            render_pass.set_bind_group(1, bind_group, &[]);
            render_pass.draw_indexed(0..Self::INDICES.len() as u32, 0, 0..1);
        }

        render_pass.set_pipeline(&self.wireframe_pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_index_buffer(
            self.wireframe_index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );

        for bind_group in bind_groups.iter() {
            render_pass.set_bind_group(1, bind_group, &[]);
            render_pass.draw_indexed(0..Self::WIREFRAME_INDICES.len() as u32, 0, 0..1);
        }
    }
}
