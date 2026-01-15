use bevy_ecs::prelude::*;
use glam::{Mat4, Vec3};

use crate::{
    engine::renderer::Renderer,
    game::{
        math::Frustum,
        scenes::world::{
            render::RenderWorld,
            sim_world::{
                Camera, ComputedCamera, DayNightCycle, SimWorldState,
                ecs::{ActiveCamera, Snapshots},
            },
            systems::Time,
        },
    },
};

/// A snapshot of the camera and the general environment to render.
#[derive(Default, Resource)]
pub struct CameraEnvSnapshot {
    pub proj_view: Mat4,

    pub frustum: Frustum,
    pub camera_position: Vec3,
    pub camera_forward: Vec3,

    pub sun_dir: Vec3,
    pub sun_color: Vec3,
    pub ambient_color: Vec3,

    pub fog_color: Vec3,
    pub fog_distance: f32,
    pub fog_near_fraction: f32,

    pub sim_time: f32,
}

pub fn extract_camera_env_snapshot(
    mut snapshots: ResMut<Snapshots>,
    computed_camera: Single<&ComputedCamera, With<ActiveCamera>>,
    day_night_cycle: Res<DayNightCycle>,
    state: Res<SimWorldState>,
    time: Res<Time>,
) {
    let tod = state.time_of_day;

    let ambient_color = Vec3::splat(0.3);

    snapshots.camera_env_snapshot = CameraEnvSnapshot {
        proj_view: computed_camera.view_proj.mat,
        frustum: computed_camera.frustum.clone(),
        camera_position: computed_camera.position,
        camera_forward: computed_camera.forward,
        sun_dir: day_night_cycle.sun_dir.sample_sub_frame(tod, true),
        sun_color: day_night_cycle.sun_color.sample_sub_frame(tod, true),
        ambient_color,
        fog_color: day_night_cycle.fog_color.sample_sub_frame(tod, true),
        fog_distance: day_night_cycle.fog_distance.sample_sub_frame(tod, true),
        fog_near_fraction: day_night_cycle
            .fog_near_fraction
            .sample_sub_frame(tod, true),
        sim_time: time.sim_time,
    };
}

pub fn compute_cameras(mut cameras: Query<(&Camera, &mut ComputedCamera)>) {
    for (camera, mut computed_camera) in cameras.iter_mut() {
        *computed_camera = camera.compute();
    }
}

pub fn prepare(renderer: &Renderer, render_world: &RenderWorld, snapshots: &Snapshots) {
    let data = gpu::CameraEnvironment::from_snapshot(&snapshots.camera_env_snapshot);

    renderer.queue.write_buffer(
        &render_world.camera_env_buffer,
        0,
        bytemuck::bytes_of(&data),
    );
}

pub mod gpu {
    use bytemuck::NoUninit;

    use crate::game::scenes::world::systems::camera_system::CameraEnvSnapshot;

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

    impl CameraEnvironment {
        pub fn from_snapshot(snapshot: &CameraEnvSnapshot) -> Self {
            Self {
                proj_view: snapshot.proj_view.to_cols_array_2d(),
                frustum: snapshot
                    .frustum
                    .planes
                    .map(|plane| plane.normal.extend(plane.distance).to_array()),
                position: snapshot.camera_position.extend(1.0).to_array(),
                forward: snapshot.camera_forward.extend(0.0).to_array(),
                sun_dir: snapshot.sun_dir.extend(0.0).to_array(),
                sun_color: snapshot.sun_color.extend(1.0).to_array(),
                ambient_color: snapshot.ambient_color.extend(1.0).to_array(),
                fog_color: snapshot.fog_color.extend(1.0).to_array(),
                fog_distance: snapshot.fog_distance,
                fog_near_fraction: snapshot.fog_near_fraction,
                sim_time: snapshot.sim_time,
                _pad: Default::default(),
            }
        }
    }
}
