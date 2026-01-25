use bevy_ecs::prelude::*;

use crate::{
    engine::renderer::Renderer,
    game::scenes::world::{
        extract::RenderSnapshot,
        render::RenderWorld,
        sim_world::{Camera, ComputedCamera},
    },
};

pub fn compute_cameras(mut cameras: Query<(&Camera, &mut ComputedCamera)>) {
    for (camera, mut computed_camera) in cameras.iter_mut() {
        *computed_camera = camera.compute();
    }
}

pub fn prepare(renderer: &Renderer, render_world: &RenderWorld, snapshot: &RenderSnapshot) {
    let data = gpu::CameraEnvironment {
        proj_view: snapshot.camera.proj_view.to_cols_array_2d(),
        frustum: snapshot
            .camera
            .frustum
            .planes
            .map(|plane| plane.normal.extend(plane.distance).to_array()),
        position: snapshot.camera.position.extend(1.0).to_array(),
        forward: snapshot.camera.forward.extend(0.0).to_array(),
        sun_dir: snapshot.environment.sun_dir.extend(0.0).to_array(),
        sun_color: snapshot.environment.sun_color.extend(1.0).to_array(),
        ambient_color: snapshot.environment.ambient_color.extend(1.0).to_array(),
        fog_color: snapshot.environment.fog_color.extend(1.0).to_array(),
        fog_distance: snapshot.environment.fog_distance,
        fog_near_fraction: snapshot.environment.fog_near_fraction,
        sim_time: snapshot.environment.sim_time,
        _pad: Default::default(),
    };

    renderer.queue.write_buffer(
        &render_world.camera_env_buffer,
        0,
        bytemuck::bytes_of(&data),
    );
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
