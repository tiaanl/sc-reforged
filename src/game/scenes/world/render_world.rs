use bytemuck::NoUninit;

use crate::{
    engine::{gizmos::GizmoVertex, prelude::Renderer},
    game::scenes::world::systems::RenderStore,
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
    pub _pad: [u32; 6],
}

#[derive(Clone, Copy, Default, NoUninit)]
#[repr(C)]
pub struct ChunkInstanceData {
    pub coord: [u32; 2],
    pub lod: u32,
    pub flags: u32,
}

/// Set of data that changes on each frame.
pub struct RenderWorld {
    /// The [RenderWorld] index.
    pub index: usize,

    pub camera_env: CameraEnvironment,

    pub camera_env_buffer: wgpu::Buffer,
    pub camera_env_bind_group: wgpu::BindGroup,

    /// A list of terrain chunks to render.
    pub terrain_chunk_instances: Vec<ChunkInstanceData>,
    /// A list of strata blocks to render.  This is a different list from `terrain_chunk_instances`,
    /// because strata's only render on the edge chunks.
    pub strata_instances: Vec<ChunkInstanceData>,
    /// Amount of instances per side. [south, west, north, east]
    pub strata_instances_side_count: [u32; 4],

    /// Buffer holding terrain chunk instance data for chunks to be rendered per frame.
    pub terrain_chunk_instances_buffer: wgpu::Buffer,
    /// Current capacity of `terrain_chunk_instance_buffer`.
    pub terrain_chunk_instances_buffer_capacity: u32,

    /// Buffer holding instance data for strata to be rendered per frame.
    pub strata_instances_buffer: wgpu::Buffer,
    /// Current capacity of the `strata_chunk_instance_buffer`.
    pub strata_instances_buffer_capacity: u32,

    pub gizmo_vertices: Vec<GizmoVertex>,
    pub gizmo_vertices_buffer: wgpu::Buffer,
    pub gizmo_vertices_buffer_capacity: u32,
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

        let terrain_chunk_instances = Vec::default();
        let terrain_chunk_instances_buffer_capacity = 1 << 7;
        let terrain_chunk_instances_buffer = Self::create_buffer_with_capacity::<ChunkInstanceData>(
            &renderer.device,
            index,
            "terrain_chunk_instances",
            terrain_chunk_instances_buffer_capacity,
            wgpu::BufferUsages::VERTEX,
        );

        let strata_instances_buffer_capacity = 1 << 7;
        let strata_instances = Vec::with_capacity(strata_instances_buffer_capacity as usize);
        let strata_instances_buffer = Self::create_buffer_with_capacity::<ChunkInstanceData>(
            &renderer.device,
            index,
            "strata_instances",
            strata_instances_buffer_capacity,
            wgpu::BufferUsages::VERTEX,
        );

        let gizmo_vertices = Vec::default();
        let gizmo_vertices_buffer_capacity = 1024;
        let gizmo_vertices_buffer = Self::create_buffer_with_capacity::<GizmoVertex>(
            &renderer.device,
            index,
            "gizmo_vertices_buffer",
            gizmo_vertices_buffer_capacity,
            wgpu::BufferUsages::VERTEX,
        );

        Self {
            index,

            camera_env: CameraEnvironment::default(),
            camera_env_buffer,
            camera_env_bind_group,

            terrain_chunk_instances,
            strata_instances,
            strata_instances_side_count: [0; 4],

            terrain_chunk_instances_buffer,
            terrain_chunk_instances_buffer_capacity,

            strata_instances_buffer,
            strata_instances_buffer_capacity,

            gizmo_vertices,
            gizmo_vertices_buffer,
            gizmo_vertices_buffer_capacity,
        }
    }

    /// Ensure that we can hold the required amount of instances.
    pub fn ensure_terrain_chunk_instance_capacity(&mut self, device: &wgpu::Device, capacity: u32) {
        if self.terrain_chunk_instances_buffer_capacity < capacity {
            let new_size = capacity.next_power_of_two();

            tracing::warn!(
                "Resizing terrain chunk instances buffer ({}) to {}",
                self.index,
                new_size
            );

            self.terrain_chunk_instances_buffer =
                Self::create_buffer_with_capacity::<ChunkInstanceData>(
                    device,
                    self.index,
                    "terrain_chunk_instances",
                    new_size,
                    wgpu::BufferUsages::VERTEX,
                );

            self.terrain_chunk_instances_buffer_capacity = new_size;
        }
    }

    /// Ensure that we can hold the required amount of instances.
    pub fn ensure_strata_instance_capacity(&mut self, device: &wgpu::Device, capacity: u32) {
        if self.strata_instances_buffer_capacity < capacity {
            let new_size = capacity.next_power_of_two();

            tracing::warn!(
                "Resizing strata instances buffer ({}) to {}",
                self.index,
                new_size
            );

            self.strata_instances_buffer = Self::create_buffer_with_capacity::<ChunkInstanceData>(
                device,
                self.index,
                "strata_instances",
                new_size,
                wgpu::BufferUsages::VERTEX,
            );

            self.strata_instances_buffer_capacity = new_size;
        }
    }

    pub fn ensure_gizmo_vertices_capacity(&mut self, device: &wgpu::Device, capacity: u32) {
        if self.gizmo_vertices_buffer_capacity < capacity {
            let new_size = capacity.next_power_of_two();

            tracing::warn!(
                "Resizing gizmo vertices buffer ({}) to {}",
                self.index,
                new_size
            );

            self.gizmo_vertices_buffer = Self::create_buffer_with_capacity::<GizmoVertex>(
                device,
                self.index,
                "gizmo_vertices",
                new_size,
                wgpu::BufferUsages::VERTEX,
            );

            self.gizmo_vertices_buffer_capacity = new_size;
        }
    }

    fn create_buffer_with_capacity<T>(
        device: &wgpu::Device,
        index: usize,
        name: &str,
        capacity: u32,
        usages: wgpu::BufferUsages,
    ) -> wgpu::Buffer
    where
        T: bytemuck::NoUninit,
    {
        let size = std::mem::size_of::<T>() as u64 * capacity as u64;

        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{name}:{index}")),
            size,
            usage: usages | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
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
}
