use bytemuck::NoUninit;

use crate::{engine::prelude::Renderer, game::scenes::world::systems::RenderStore};

#[derive(Clone, Copy, Debug, Default, NoUninit)]
#[repr(C)]
pub struct CameraEnvironment {
    pub proj_view: [[f32; 4]; 4],
    pub frustum: [[f32; 4]; 6],
    pub position: [f32; 4], // x, y, z, 1
    pub forward: [f32; 4],  // x, y, z, 0

    pub sun_dir: [f32; 4],   // x, y, z, 0
    pub sun_color: [f32; 4], // r, g, b, 1
    pub fog_color: [f32; 4], // r, g, b, 1
    pub fog_distance: f32,
    pub fog_near_fraction: f32,
    pub _pad: [u32; 2],
}

#[derive(Clone, Copy, Default, NoUninit)]
#[repr(C)]
pub struct ChunkInstanceData {
    pub coord: [u32; 2],
    pub lod: u32,
    pub flags: u32,
}

pub struct RenderWorld {
    /// The [RenderWorld] index.
    pub index: usize,

    pub camera_env: CameraEnvironment,

    pub camera_env_buffer: wgpu::Buffer,
    pub camera_env_bind_group: wgpu::BindGroup,

    pub terrain_chunk_instances: Vec<ChunkInstanceData>,
    /// A buffer containing instance data for chunks to be rendered per frame.
    pub terrain_chunk_instances_buffer: wgpu::Buffer,
    /// Current capacity of the `terrain_chunk_instance_buffer`.
    pub terrain_chunk_instances_buffer_capacity: u32,
}

impl RenderWorld {
    pub const CAMERA_BIND_GROUP_LAYOUT_ID: &str = "camera_bind_group_layout";

    pub fn new(index: usize, renderer: &Renderer, render_store: &mut RenderStore) -> Self {
        // Make sure the camera bind group layout is in the store.
        if render_store
            .get_bind_group_layout(Self::CAMERA_BIND_GROUP_LAYOUT_ID)
            .is_none()
        {
            let bind_group_layout =
                renderer
                    .device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: Some(Self::CAMERA_BIND_GROUP_LAYOUT_ID),
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
                    });

            render_store
                .store_bind_group_layout(Self::CAMERA_BIND_GROUP_LAYOUT_ID, bind_group_layout);
        }

        // SAFETY: We can unwrap here, because we ensured the bind group layout is in the store
        //         above.
        let camera_bind_group_layout = render_store
            .get_bind_group_layout(Self::CAMERA_BIND_GROUP_LAYOUT_ID)
            .unwrap();

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
                layout: camera_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_env_buffer.as_entire_binding(),
                }],
            });

        let terrain_chunk_instances = Vec::default();
        let terrain_chunk_instances_buffer_capacity = 1 << 7;
        let terrain_chunk_instances_buffer = Self::create_terrain_chunk_instance_buffer(
            &renderer.device,
            index,
            terrain_chunk_instances_buffer_capacity,
        );

        Self {
            index,

            camera_env: CameraEnvironment::default(),
            camera_env_buffer,
            camera_env_bind_group,

            terrain_chunk_instances,
            terrain_chunk_instances_buffer,
            terrain_chunk_instances_buffer_capacity,
        }
    }

    /// Ensure that we can hold the required amount of instances.
    pub fn ensure_terrain_chunk_instance_capacity(&mut self, device: &wgpu::Device, count: u32) {
        if self.terrain_chunk_instances_buffer_capacity < count {
            let new_size = count.next_power_of_two();

            tracing::warn!(
                "Resizing terrain chunk instances buffer ({}) to {}",
                self.index,
                new_size
            );

            self.terrain_chunk_instances_buffer =
                Self::create_terrain_chunk_instance_buffer(device, self.index, new_size)
        }
    }

    fn create_terrain_chunk_instance_buffer(
        device: &wgpu::Device,
        index: usize,
        count: u32,
    ) -> wgpu::Buffer {
        let size = std::mem::size_of::<ChunkInstanceData>() as u64 * count as u64;

        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("terrain_chunk_instance_data:{index}")),
            size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }
}
