use crate::{
    engine::{growing_buffer::GrowingBuffer, renderer::Renderer},
    game::scenes::world::render::{
        camera_render_pipeline::{self, CameraEnvironmentLayout},
        model_render_pipeline,
        render_layouts::RenderLayouts,
        terrain_render_pipeline::gpu::ChunkInstanceData,
        ui_render_pipeline::{self, UiStateLayout},
    },
};

/// Set of data that changes on each frame.
pub struct RenderWorld {
    pub camera_env_buffer: wgpu::Buffer,
    pub camera_env_bind_group: wgpu::BindGroup,

    /// Buffer holding terrain chunk instance data for chunks to be rendered per frame.
    pub terrain_chunk_instances_buffer: GrowingBuffer<ChunkInstanceData>,

    /// Buffer holding instance data for strata to be rendered per frame.
    pub strata_instances_buffer: GrowingBuffer<ChunkInstanceData>,

    pub model_instances: GrowingBuffer<model_render_pipeline::gpu::ModelInstanceData>,

    pub ui_state_buffer: wgpu::Buffer,
    pub ui_state_bind_group: wgpu::BindGroup,

    pub ui_rects_buffer: GrowingBuffer<ui_render_pipeline::gpu::Rect>,
}

impl RenderWorld {
    pub fn new(index: usize, renderer: &Renderer, layouts: &mut RenderLayouts) -> Self {
        let camera_env_buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cameras"),
            size: std::mem::size_of::<camera_render_pipeline::gpu::CameraEnvironment>()
                as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_env_bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("cmaera_bind_group_{index}")),
                layout: layouts.get::<CameraEnvironmentLayout>(renderer),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_env_buffer.as_entire_binding(),
                }],
            });

        let capacity = 1 << 7;
        let terrain_chunk_instances_buffer = GrowingBuffer::new(
            renderer,
            capacity,
            wgpu::BufferUsages::VERTEX,
            format!("terrain_chunk_instances:{index}"),
        );

        let capacity = 1 << 7;
        let strata_instances_buffer = GrowingBuffer::new(
            renderer,
            capacity,
            wgpu::BufferUsages::VERTEX,
            format!("strata_instances:{index}"),
        );

        let model_instances = GrowingBuffer::new(
            renderer,
            1 << 7,
            wgpu::BufferUsages::VERTEX,
            format!("model_instances:{index}"),
        );

        let ui_state_buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui_state_buffer"),
            size: std::mem::size_of::<ui_render_pipeline::gpu::State>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        let ui_state_bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("ui_state_bind_group_{index}")),
                layout: layouts.get::<UiStateLayout>(renderer),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ui_state_buffer.as_entire_binding(),
                }],
            });

        let ui_rects_buffer = GrowingBuffer::new(
            renderer,
            1024,
            wgpu::BufferUsages::VERTEX,
            format!("ui_rects_buffer:{index}"),
        );

        Self {
            camera_env_buffer,
            camera_env_bind_group,

            terrain_chunk_instances_buffer,

            strata_instances_buffer,

            model_instances,

            ui_state_buffer,
            ui_state_bind_group,
            ui_rects_buffer,
        }
    }
}
