use glam::{Mat4, UVec2, Vec2, Vec3, Vec4};

use crate::{
    engine::{
        gizmos::GizmoVertex,
        input::InputState,
        prelude::{Frame, Renderer},
    },
    game::{
        config::Campaign,
        models::models,
        scenes::world::{
            animation::generate_pose,
            render::{RenderStore, RenderWorld},
            sim_world::SimWorld,
            systems::top_down_camera_controller::TopDownCameraController,
        },
        skeleton::Skeleton,
    },
};

pub use cull_system::DebugQuadTreeOptions;
pub use objects_system::RenderWrapper;

mod camera_system;
mod clear_render_targets;
mod cull_system;
mod day_night_cycle_system;
mod free_camera_controller;
mod gizmo_system;
mod objects_system;
mod terrain_system;
mod top_down_camera_controller;
mod ui_system;
mod world_interaction;

pub struct Time {
    pub delta_time: f32,
}

/// Shared resources between rendering in the systems and the [RenderWorld].
pub struct Systems {
    camera_system: camera_system::CameraSystem<TopDownCameraController>,
    pub culling: cull_system::CullSystem,
    pub terrain_system: terrain_system::TerrainSystem,
    pub objects_system: objects_system::ObjectsSystem,
    world_interaction_system: world_interaction::WorldInteractionSystem,
    gizmo_system: gizmo_system::GizmoSystem,
    ui_system: ui_system::UiSystem,
}

impl Systems {
    pub fn new(
        renderer: &Renderer,
        render_store: &RenderStore,
        sim_world: &SimWorld,
        campaign: &Campaign,
    ) -> Self {
        Self {
            camera_system: camera_system::CameraSystem::new({
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
            }),
            culling: cull_system::CullSystem::default(),
            terrain_system: terrain_system::TerrainSystem::new(renderer, render_store, sim_world),
            objects_system: objects_system::ObjectsSystem::new(renderer, render_store),
            world_interaction_system: world_interaction::WorldInteractionSystem::default(),
            gizmo_system: gizmo_system::GizmoSystem::new(renderer, render_store),
            ui_system: ui_system::UiSystem::new(renderer, render_store),
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
        self.world_interaction_system
            .input(sim_world, input_state, viewport_size);
    }

    pub fn update(&mut self, sim_world: &mut SimWorld, time: &Time) {
        self.culling.calculate_visible_chunks(sim_world);
        day_night_cycle_system::increment_time_of_day(sim_world, time);
        self.objects_system.render_gizmos(sim_world);

        self.world_interaction_system.update(sim_world);

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
        sim_world: &mut SimWorld,
        render_store: &mut RenderStore,
        render_world: &mut RenderWorld,
        viewport_size: UVec2,
    ) {
        self.camera_system.extract(sim_world, render_world);
        self.terrain_system.extract(sim_world, render_world);
        self.gizmo_system.extract(sim_world, render_world);

        // Make sure all models are prepared to be rendered.
        sim_world.objects.prepare_models(render_store);

        self.objects_system.extract(sim_world, render_store);

        self.ui_system
            .extract(sim_world, render_store, render_world, viewport_size);
    }

    pub fn prepare(
        &mut self,
        render_store: &mut RenderStore,
        render_world: &mut RenderWorld,
        renderer: &Renderer,
    ) {
        // Make sure the geometry buffer is the correct size.
        if renderer.surface.size() != render_store.geometry_buffer.size {
            render_store
                .geometry_buffer
                .resize(&renderer.device, renderer.surface.size());
        }

        self.camera_system.prepare(render_world, renderer);
        self.terrain_system.prepare(render_world, renderer);
        self.objects_system.prepare(render_world, renderer);
        self.gizmo_system.prepare(render_world, renderer);

        self.ui_system.prepare(render_world, renderer);
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
        self.terrain_system
            .queue(render_world, frame, &render_store.geometry_buffer);
        self.objects_system.queue(
            render_store,
            render_world,
            frame,
            &render_store.geometry_buffer,
        );

        render_store
            .compositor
            .render(frame, &render_store.geometry_buffer);

        self.gizmo_system.queue(render_world, frame);

        self.ui_system.queue(render_world, frame);
    }
}
