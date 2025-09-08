use ahash::HashSet;
use glam::{Mat3, Quat, Vec2, Vec3};

use crate::{
    engine::{prelude::Transform, storage::Handle},
    game::{
        animations::{Sequencer, sequences},
        config::ObjectType,
        height_map::HeightMap,
        model::Model,
        renderer::{ModelRenderer, RenderInstance},
    },
};

/// Represents an object inside the game world.
pub struct Object {
    pub title: String,
    pub object_type: ObjectType,
    pub transform: Transform,
    pub detail: ObjectDetail,

    /// Whether to draw the bones of the skeleton.
    pub draw_debug_bones: bool,
    /// Whether to draw the bounding sphere for each mesh.
    pub draw_bounding_spheres: bool,
    /// A list of node indices to draw in debug mode.
    pub selected_bones: HashSet<usize>,
}

impl Object {
    pub fn update(
        &mut self,
        delta_time: f32,
        height_map: &HeightMap,
        model_renderer: &mut ModelRenderer,
    ) {
        match self.detail {
            ObjectDetail::Scenery {
                render_instance, ..
            }
            | ObjectDetail::SceneryLit {
                render_instance, ..
            } => {
                model_renderer.update_instance(render_instance, |updater| {
                    updater.set_transform(self.transform.to_mat4());
                });
            }

            ObjectDetail::Bipedal {
                ref mut order,
                ref mut sequencer,
                ..
            } => {
                sequencer.update(delta_time);

                match *order {
                    BipedalOrder::Stand => {}
                    BipedalOrder::MoveTo {
                        target_location,
                        speed,
                    } => {
                        let current_xy = self.transform.translation.truncate();
                        let (current_pos, current_normal) =
                            height_map.world_position_and_normal(current_xy);

                        // Snap to the ground.
                        self.transform.translation.z = current_pos.z;

                        // Create a vector to the target.
                        let to_target_xy = target_location - current_xy;
                        let distance_to_target = to_target_xy.length();

                        // Arrived already?
                        if distance_to_target <= speed * delta_time {
                            let (target_pos, _) =
                                height_map.world_position_and_normal(target_location);

                            self.transform.translation = target_pos;

                            let forward = to_target_xy.extend(0.0).normalize();
                            let left = forward.cross(Vec3::Z).normalize_or_zero();
                            let basis = Mat3::from_cols(left, forward, Vec3::Z);
                            self.transform.rotation = Quat::from_mat3(&basis);

                            // Issue a *stand* order.
                            *order = BipedalOrder::Stand;
                            if let Some(stand_sequence) = sequences().get_by_name("MSEQ_STAND") {
                                sequencer.play_sequence(stand_sequence);
                            } else {
                                tracing::warn!("Could not play sequence MSEQ_STAND");
                            }
                        } else {
                            // Desired planar direction, then slide along the terrain tangent.
                            let desired_dir_world =
                                Vec3::new(to_target_xy.x, to_target_xy.y, 0.0).normalize();

                            // Project onto the ground plane.
                            let tangent_dir = (desired_dir_world
                                - current_normal * desired_dir_world.dot(current_normal))
                            .normalize_or_zero();

                            // Constant-speed step, clamped to avoid overshooting.
                            let step_xy = (speed * delta_time).min(distance_to_target);
                            let new_xy = current_xy + tangent_dir.truncate() * step_xy;

                            // Stick to the ground at the new (x, y) position.
                            let (new_pos, _) = height_map.world_position_and_normal(new_xy);

                            self.transform.translation = new_pos;

                            let forward = to_target_xy.extend(0.0).normalize();
                            let left = forward.cross(Vec3::Z).normalize_or_zero();
                            let basis = Mat3::from_cols(left, forward, Vec3::Z);
                            self.transform.rotation = Quat::from_mat3(&basis);
                        }
                    }
                }
            }
        }

        self.update_model_renderer(model_renderer);
    }

    pub fn update_model_renderer(&self, model_renderer: &mut ModelRenderer) {
        match self.detail {
            ObjectDetail::Scenery {
                render_instance, ..
            }
            | ObjectDetail::SceneryLit {
                render_instance, ..
            } => {
                model_renderer.update_instance(render_instance, |updater| {
                    updater.set_transform(self.transform.to_mat4());
                });
            }
            ObjectDetail::Bipedal {
                body_model,
                body_render_instance,
                head_render_instance,
                ref sequencer,
                ..
            } => {
                let mut animation = None;
                let mut time = 0.0;
                if let Some(animation_state) = sequencer.get_animation_state() {
                    animation = Some(
                        model_renderer
                            .get_or_insert_animation(body_model, animation_state.animation),
                    );
                    time = animation_state.frame;
                }

                model_renderer.update_instance(body_render_instance, |updater| {
                    updater.set_transform(self.transform.to_mat4());
                    if let Some(animation) = animation {
                        updater.set_animation(animation, time);
                    }
                });
                model_renderer.update_instance(head_render_instance, |updater| {
                    updater.set_transform(self.transform.to_mat4());
                    if let Some(animation) = animation {
                        updater.set_animation(animation, time);
                    }
                });
            }
        }
    }
}

pub enum ObjectDetail {
    Scenery {
        model: Handle<Model>,
        render_instance: Handle<RenderInstance>,
    },
    SceneryLit {
        model: Handle<Model>,
        render_instance: Handle<RenderInstance>,
    },
    Bipedal {
        body_model: Handle<Model>,
        body_render_instance: Handle<RenderInstance>,
        head_render_instance: Handle<RenderInstance>,
        order: BipedalOrder,
        sequencer: Sequencer,
    },
}

pub enum BipedalOrder {
    Stand,
    MoveTo { target_location: Vec2, speed: f32 },
}

fn project_onto_plane(v: Vec3, n: Vec3) -> Vec3 {
    v - n * v.dot(n)
}
