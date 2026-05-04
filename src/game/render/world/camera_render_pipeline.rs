use crate::{
    engine::renderer::{RenderContext, RenderTarget},
    game::render::{
        geometry_buffer::GeometryBuffer,
        world::{
            render_bindings::RenderBindings, render_layouts::RenderLayout,
            render_pipeline::RenderPipeline, world_render_snapshot::WorldRenderSnapshot,
        },
    },
};

pub struct CameraEnvironmentLayout;

impl RenderLayout for CameraEnvironmentLayout {
    fn label() -> &'static str {
        "camera_bind_group_layout"
    }

    fn entries() -> &'static [wgpu::BindGroupLayoutEntry] {
        const ENTRIES: &[wgpu::BindGroupLayoutEntry] = &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }];

        ENTRIES
    }
}

pub struct CameraRenderPipeline;

impl RenderPipeline for CameraRenderPipeline {
    fn prepare(
        &mut self,
        context: &crate::engine::renderer::Gpu,
        bindings: &mut RenderBindings,
        snapshot: &WorldRenderSnapshot,
    ) {
        let data = gpu::CameraEnvironment {
            proj_view: snapshot.camera.proj_view.to_cols_array_2d(),
            frustum: snapshot
                .camera
                .frustum
                .planes
                .map(|plane| plane.normal.extend(plane.distance).to_array()),
            position: snapshot
                .camera
                .position
                .extend(snapshot.camera._near)
                .to_array(),
            forward: snapshot
                .camera
                .forward
                .extend(snapshot.camera.far)
                .to_array(),
            sun_dir: snapshot.environment.sun_dir.extend(0.0).to_array(),
            sun_color: snapshot.environment.sun_color.extend(1.0).to_array(),
            ambient_color: snapshot.environment.ambient_color.extend(1.0).to_array(),
            fog_color: snapshot.environment.fog_color.extend(1.0).to_array(),
            fog_distance: snapshot.environment.fog_distance,
            fog_near_fraction: snapshot.environment.fog_near_fraction,
            sim_time: snapshot.environment.sim_time,
            _pad: Default::default(),
        };

        bindings
            .camera_env_buffer
            .advance()
            .write(context, bytemuck::bytes_of(&data));
    }

    fn queue(
        &self,
        _bindings: &RenderBindings,
        _render_context: &mut RenderContext,
        _render_target: &RenderTarget,
        _geometry_buffer: &GeometryBuffer,
        _snapshot: &WorldRenderSnapshot,
    ) {
    }
}

pub mod gpu {
    use bytemuck::NoUninit;

    #[derive(Clone, Copy, Debug, Default, NoUninit)]
    #[repr(C)]
    pub struct CameraEnvironment {
        pub proj_view: [[f32; 4]; 4],
        pub frustum: [[f32; 4]; 6],
        pub position: [f32; 4], // x, y, z, near
        pub forward: [f32; 4],  // x, y, z, far

        pub sun_dir: [f32; 4],       // x, y, z, 0
        pub sun_color: [f32; 4],     // r, g, b, 1
        pub ambient_color: [f32; 4], // r, g, b, 1
        pub fog_color: [f32; 4],     // r, g, b, 1
        pub fog_distance: f32,
        pub fog_near_fraction: f32,
        pub sim_time: f32,
        pub _pad: [u32; 5],
    }
}
