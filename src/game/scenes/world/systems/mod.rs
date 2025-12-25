use glam::{Mat4, UVec2, Vec2, Vec3, Vec4};

use crate::{
    engine::{
        gizmos::GizmoVertex,
        input::InputState,
        renderer::{Frame, Renderer},
    },
    game::{
        config::Campaign,
        models::models,
        scenes::world::{
            animation::generate_pose,
            render::{RenderStore, RenderWorld, WorldRenderer},
            sim_world::SimWorld,
            systems::{
                free_camera_controller::FreeCameraController,
                top_down_camera_controller::TopDownCameraController,
            },
        },
        skeleton::Skeleton,
    },
};

mod camera_system;
mod clear_render_targets;
mod cull_system;
mod day_night_cycle_system;
mod free_camera_controller;
mod gizmo_system;
mod top_down_camera_controller;
mod world_interaction;

pub struct Time {
    pub delta_time: f32,
}

/// Shared resources between rendering in the systems and the [RenderWorld].
pub struct Systems {
    pub camera_system: camera_system::CameraSystem,
    pub culling: cull_system::CullSystem,

    pub world_renderer: WorldRenderer,

    world_interaction_system: world_interaction::WorldInteractionSystem,
    gizmo_system: gizmo_system::GizmoSystem,
}

impl Systems {
    pub fn new(
        renderer: &Renderer,
        surface_format: wgpu::TextureFormat,
        render_store: &RenderStore,
        sim_world: &SimWorld,
        campaign: &Campaign,
    ) -> Self {
        Self {
            camera_system: camera_system::CameraSystem::new(
                {
                    let camera_from = campaign.view_initial.from.extend(2500.0);
                    let camera_to = campaign.view_initial.to.extend(0.0);

                    let dir = (camera_to - camera_from).normalize();

                    let flat = Vec2::new(dir.x, dir.y);
                    let yaw = (-dir.x).atan2(dir.y).to_degrees();
                    let pitch = dir.z.atan2(flat.length()).to_degrees();

                    TopDownCameraController::new(
                        camera_from,
                        yaw.to_degrees(),
                        pitch.to_degrees(),
                        4_000.0,
                        100.0,
                    )
                },
                FreeCameraController::new(1000.0, 0.2),
            ),
            culling: cull_system::CullSystem::default(),
            world_renderer: WorldRenderer::new(renderer, surface_format, render_store, sim_world),
            world_interaction_system: world_interaction::WorldInteractionSystem::default(),
            gizmo_system: gizmo_system::GizmoSystem::new(renderer, surface_format, render_store),
        }
    }

    pub fn input(
        &mut self,
        sim_world: &mut SimWorld,
        time: &Time,
        input_state: &InputState,
        viewport_size: UVec2,
    ) {
        // TODO: Not nice that we have to pass in a `viewport_size` here, but don't know where else
        //       to put it for now.

        // TODO: This should really be part of a system somewhere.
        sim_world.ui.ui_rects.clear();

        self.camera_system.input(sim_world, time, input_state);

        // TODO: This should be the first step in the update system, but that
        //       would mean all systems should record input state and then
        //       process it in `update` as well, which is not done right now.
        self.camera_system.compute_cameras(sim_world);

        self.world_interaction_system
            .input(sim_world, input_state, viewport_size);
    }

    pub fn update(&mut self, sim_world: &mut SimWorld, time: &Time) {
        self.culling.calculate_visible_chunks(sim_world);
        day_night_cycle_system::increment_time_of_day(sim_world, time);

        self.world_interaction_system.update(sim_world);

        {
            // Test BVH.
            sim_world
                .objects
                .static_bvh
                .test(&mut sim_world.gizmo_vertices);
        }

        #[allow(clippy::collapsible_if)]
        if false {
            if let Some(model) = models().get(sim_world.test_model) {
                let pose = generate_pose(
                    &model.skeleton,
                    &sim_world.test_motion,
                    sim_world.timer,
                    true,
                );

                fn draw_bone(
                    skeleton: &Skeleton,
                    pose: &[Mat4],
                    bone_index: u32,
                    origin: &Mat4,
                    gizmo_vertices: &mut Vec<GizmoVertex>,
                    color: Option<Vec4>,
                    level: usize,
                ) {
                    static COLORS: &[Vec4] = &[
                        Vec4::new(1.0, 0.0, 0.0, 1.0),
                        Vec4::new(0.0, 1.0, 0.0, 1.0),
                        Vec4::new(0.0, 0.0, 1.0, 1.0),
                        Vec4::new(1.0, 1.0, 0.0, 1.0),
                        Vec4::new(1.0, 0.0, 1.0, 1.0),
                        Vec4::new(1.0, 1.0, 1.0, 1.0),
                    ];

                    let start = (origin * pose[bone_index as usize]).w_axis.truncate();

                    for (index, _) in skeleton
                        .bones
                        .iter()
                        .enumerate()
                        .filter(|(_, b)| b.parent == bone_index)
                    {
                        let end = (origin * pose[index]).w_axis.truncate();

                        let this_color = color.unwrap_or(COLORS[level.min(COLORS.len() - 1)]);

                        gizmo_vertices.extend_from_slice(&[
                            GizmoVertex::new(start, this_color),
                            GizmoVertex::new(end, this_color),
                        ]);

                        draw_bone(
                            skeleton,
                            pose,
                            index as u32,
                            origin,
                            gizmo_vertices,
                            color,
                            level + 1,
                        );
                    }
                }

                draw_bone(
                    &model.skeleton,
                    &pose.bones,
                    0,
                    &Mat4::IDENTITY,
                    &mut sim_world.gizmo_vertices,
                    Some(Vec4::new(1.0, 1.0, 1.0, 1.0)),
                    0,
                );

                let rest_pose: Vec<Mat4> = (0..model.skeleton.bones.len() as u32)
                    .map(|b| model.skeleton.local_transform(b))
                    .collect();

                draw_bone(
                    &model.skeleton,
                    &rest_pose,
                    0,
                    &Mat4::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                    &mut sim_world.gizmo_vertices,
                    Some(Vec4::new(0.0, 0.0, 1.0, 1.0)),
                    0,
                );
            }
        }
    }

    pub fn extract(
        &mut self,
        renderer: &Renderer,
        sim_world: &mut SimWorld,
        render_store: &mut RenderStore,
        render_world: &mut RenderWorld,
        viewport_size: UVec2,
    ) {
        self.camera_system.extract(sim_world, render_world);

        // Make sure all models are prepared to be rendered.
        sim_world.objects.prepare_models(renderer, render_store);

        self.world_renderer
            .extract(sim_world, render_store, render_world, viewport_size);

        self.gizmo_system.extract(sim_world, render_world);
    }

    pub fn prepare(
        &mut self,
        render_store: &mut RenderStore,
        render_world: &mut RenderWorld,
        renderer: &Renderer,
        surface_size: UVec2,
    ) {
        // Make sure the geometry buffer is the correct size.
        if surface_size != render_store.geometry_buffer.size {
            render_store
                .geometry_buffer
                .resize(&renderer.device, surface_size);
        }

        self.camera_system.prepare(render_world, renderer);
        self.world_renderer.prepare(renderer, render_world);
        self.gizmo_system.prepare(render_world, renderer);
    }

    pub fn queue(
        &mut self,
        render_store: &RenderStore,
        render_world: &RenderWorld,
        frame: &mut Frame,
    ) {
        clear_render_targets::clear_render_targets(
            render_world,
            frame,
            &render_store.geometry_buffer,
        );
        self.world_renderer.queue(
            render_store,
            render_world,
            frame,
            &render_store.geometry_buffer,
        );

        render_store
            .compositor
            .render(frame, &render_store.geometry_buffer);

        self.gizmo_system.queue(render_world, frame);
    }
}
