use wgpu::util::DeviceExt;

use super::{NewSystemContext, RenderWorld};
use crate::{game::scenes::world::new_terrain::NewTerrain, wgsl_shader};

pub struct StrataSystem {
    vertex_buffer: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
}

impl StrataSystem {
    const STRATA_DESCENT: f32 = -20_000.0;

    pub fn new(context: &mut NewSystemContext) -> Self {
        let device = &context.renderer.device;

        // TODO: Don't duplicate the shader from the terrain system.
        let module = device.create_shader_module(wgsl_shader!("new_terrain"));

        let camera_bind_group_layout = context
            .render_store
            .get_bind_group_layout(RenderWorld::CAMERA_BIND_GROUP_LAYOUT_ID)
            .expect("Requires camera bind group layout!");
        let terrain_bind_group_layout = context
            .render_store
            .get_bind_group_layout(RenderWorld::TERRAIN_BIND_GROUP_LAYOUT_ID)
            .expect("Requires terrain bind group layout!");

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("strata_pipeline_layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &terrain_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("strata_render_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("strata_vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<StrataVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3,
                        1 => Float32x3,
                        2 => Uint32x2,
                    ],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                cull_mode: Some(wgpu::Face::Back),
                // polygon_mode: wgpu::PolygonMode::Line,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("strata_fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: context.renderer.surface.format(),
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        let vertices = generate_strata_mesh();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("strata_vertex_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            vertex_buffer,
            pipeline,
        }
    }
}

impl super::System for StrataSystem {
    fn queue(&mut self, context: &mut super::QueueContext) {
        let mut render_pass =
            context
                .frame
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("strata_render_pass"),
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
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_bind_group(0, &render_world.camera_env_bind_group, &[]);
        if let Some(bind_group) = context
            .render_store
            .get_bind_group(RenderWorld::TERRAIN_BIND_GROUP_ID)
        {
            render_pass.set_bind_group(1, &bind_group, &[]);
        }

        render_pass.draw(0..18, 0..1); // south
        render_pass.draw(18..36, 0..1); // west
        render_pass.draw(36..54, 0..1); // north
        render_pass.draw(54..72, 0..1); // east
    }
}

#[derive(Clone, Copy, Debug, bytemuck::NoUninit)]
#[repr(C)]
struct StrataVertex {
    position: [f32; 3],
    normal: [f32; 3],
    node_coord: [u32; 2],
}

fn generate_strata_mesh() -> Vec<StrataVertex> {
    let descent = StrataSystem::STRATA_DESCENT;
    let cells_per_chunk = NewTerrain::CELLS_PER_CHUNK;
    let nodes_per_chunk = NewTerrain::NODES_PER_CHUNK;

    // TODO: Precalc the capacity.
    let mut vertices = Vec::default();

    let push = |vertices: &mut Vec<StrataVertex>, x, y, node_x, node_y, normal| {
        vertices.push(StrataVertex {
            position: [x, y, descent],
            normal,
            node_coord: [node_x, node_y],
        });
        vertices.push(StrataVertex {
            position: [x, y, 0.0],
            normal,
            node_coord: [node_x, node_y],
        });
    };

    // South
    for x in 0..nodes_per_chunk {
        push(&mut vertices, x as f32, 0.0, x, 0, [0.0, -1.0, 0.0]);
    }

    // West
    for y in 0..nodes_per_chunk {
        push(
            &mut vertices,
            cells_per_chunk as f32,
            y as f32,
            nodes_per_chunk,
            y,
            [1.0, 0.0, 0.0],
        );
    }

    // North
    for x in 0..nodes_per_chunk {
        push(
            &mut vertices,
            cells_per_chunk as f32 - x as f32,
            cells_per_chunk as f32,
            nodes_per_chunk,
            nodes_per_chunk,
            [0.0, 1.0, 0.0],
        );
    }

    // East
    for y in 0..nodes_per_chunk {
        push(
            &mut vertices,
            0.0,
            cells_per_chunk as f32 - y as f32,
            0,
            nodes_per_chunk,
            [-1.0, 0.0, 0.0],
        );
    }

    vertices
}
