use bytemuck::NoUninit;

use crate::{
    engine::{gizmos::GizmoVertex, growing_buffer::GrowingBuffer, renderer::Renderer},
    game::scenes::world::render::render_store::RenderStore,
};

#[derive(Clone, Copy, Debug, Default, NoUninit)]
#[repr(C)]
pub struct CameraEnvironment {
    pub proj_view: [[f32; 4]; 4],
    pub frustum: [[f32; 4]; 6],
    pub position: [f32; 4], // x, y, z, 1
    pub forward: [f32; 4],  // x, y, z, 0

    pub sun_dir: [f32; 4],       // x, y, z, 0
    pub sun_color: [f32; 4],     // r, g, b, 1
    pub ambient_color: [f32; 4], // r, g, b, 1
    pub fog_color: [f32; 4],     // r, g, b, 1
    pub fog_distance: f32,
    pub fog_near_fraction: f32,
    pub sim_time: f32,
    pub _pad: [u32; 5],
}

#[derive(Clone, Copy, Default, NoUninit)]
#[repr(C)]
pub struct ChunkInstanceData {
    pub coord: [u32; 2],
    pub lod: u32,
    pub flags: u32,
}

#[derive(Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
pub struct ModelInstanceData {
    pub transform: [[f32; 4]; 4],
    pub first_node_index: u32,
    pub flags: u32,
    pub _pad: [u32; 2],
}

#[derive(Clone, Copy, Debug, Default, NoUninit)]
#[repr(C)]
pub struct UiState {
    pub view_proj: [[f32; 4]; 4],
}

#[derive(Clone, Copy, Debug, Default, NoUninit)]
#[repr(C)]
pub struct RenderUiRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub color: [f32; 4],
}

/// Set of data that changes on each frame.
pub struct RenderWorld {
    pub camera_env: CameraEnvironment,

    pub camera_env_buffer: wgpu::Buffer,
    pub camera_env_bind_group: wgpu::BindGroup,

    /// A list of terrain chunks to render.
    pub terrain_chunk_instances: Vec<ChunkInstanceData>,
    /// Buffer holding terrain chunk instance data for chunks to be rendered per frame.
    pub terrain_chunk_instances_buffer: GrowingBuffer<ChunkInstanceData>,

    /// A list of strata blocks to render.  This is a different list from `terrain_chunk_instances`,
    /// because strata's only render on the edge chunks.
    pub strata_instances: Vec<ChunkInstanceData>,
    /// Amount of instances per side. [south, west, north, east]
    pub strata_instances_side_count: [u32; 4],
    /// Buffer holding instance data for strata to be rendered per frame.
    pub strata_instances_buffer: GrowingBuffer<ChunkInstanceData>,

    pub model_instances: GrowingBuffer<ModelInstanceData>,

    pub gizmo_vertices: Vec<GizmoVertex>,
    pub gizmo_vertices_buffer: GrowingBuffer<GizmoVertex>,

    pub ui_state: UiState,
    pub ui_state_buffer: wgpu::Buffer,
    pub ui_state_bind_group: wgpu::BindGroup,

    pub ui_rects: Vec<RenderUiRect>,
    pub ui_rects_buffer: GrowingBuffer<RenderUiRect>,
}

impl RenderWorld {
    pub fn new(index: usize, renderer: &Renderer, render_store: &RenderStore) -> Self {
        let camera_env_buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cameras"),
            size: std::mem::size_of::<CameraEnvironment>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_env_bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("cmaera_bind_group_{index}")),
                layout: &render_store.camera_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_env_buffer.as_entire_binding(),
                }],
            });

        let capacity = 1 << 7;
        let terrain_chunk_instances = Vec::with_capacity(capacity as usize);
        let terrain_chunk_instances_buffer = GrowingBuffer::new(
            renderer,
            capacity,
            wgpu::BufferUsages::VERTEX,
            format!("terrain_chunk_instances:{index}"),
        );

        let capacity = 1 << 7;
        let strata_instances = Vec::with_capacity(capacity as usize);
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

        let gizmo_vertices = Vec::default();
        let gizmo_vertices_buffer = GrowingBuffer::new(
            renderer,
            1024,
            wgpu::BufferUsages::VERTEX,
            format!("gizmo_vertices:{index}"),
        );

        let ui_state_buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui_state_buffer"),
            size: std::mem::size_of::<UiState>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        let ui_state_bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("ui_state_bind_group_{index}")),
                layout: &render_store.ui_state_bind_group_layout,
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
            camera_env: CameraEnvironment::default(),
            camera_env_buffer,
            camera_env_bind_group,

            terrain_chunk_instances,
            terrain_chunk_instances_buffer,

            strata_instances_buffer,
            strata_instances,
            strata_instances_side_count: [0; 4],

            model_instances,

            gizmo_vertices,
            gizmo_vertices_buffer,

            ui_state: UiState::default(),
            ui_state_buffer,
            ui_state_bind_group,
            ui_rects: Vec::default(),
            ui_rects_buffer,
        }
    }

    pub fn create_camera_bind_group_layout(renderer: &Renderer) -> wgpu::BindGroupLayout {
        renderer
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
    }

    pub fn create_ui_state_bind_group_layout(renderer: &Renderer) -> wgpu::BindGroupLayout {
        renderer
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ui_state_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
    }
}
