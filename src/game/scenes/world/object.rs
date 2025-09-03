use ahash::HashSet;
use glam::{Quat, Vec3};

use crate::{
    engine::{prelude::Transform, storage::Handle},
    game::{
        animations::{Sequencer, sequences},
        config::ObjectType,
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
    pub fn update(&mut self, delta_time: f32, model_renderer: &mut ModelRenderer) {
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
                ref mut sequencer,
                ref mut order,
                body_render_instance,
                head_render_instance,
            } => {
                match *order {
                    BipedalOrder::Stand => {}
                    BipedalOrder::MoveTo {
                        target_location,
                        speed,
                    } => {
                        // Calculate the direction where we should move to.
                        let diff = target_location - self.transform.translation;
                        let distance = diff.length();
                        if distance < 10.0 {
                            *order = BipedalOrder::Stand;
                            if let Some(stand_sequence) = sequences().get_by_name("MSEQ_STAND") {
                                sequencer.play_sequence(stand_sequence);
                            } else {
                                sequencer.stop();
                            }
                        } else {
                            let direction = diff.normalize();
                            self.transform.translation += direction * speed;
                            self.transform.rotation = Quat::from_rotation_arc(Vec3::Y, direction);
                        }
                    }
                }

                sequencer.update(delta_time);

                if let Some(animation_state) = sequencer.get_animation_state() {
                    let render_animation =
                        model_renderer.add_animation(body_model, animation_state.animation);

                    model_renderer.update_instance(body_render_instance, |updater| {
                        updater.set_animation(render_animation, animation_state.time);
                        updater.set_transform(self.transform.to_mat4());
                    });
                    model_renderer.update_instance(head_render_instance, |updater| {
                        updater.set_animation(render_animation, animation_state.time);
                        updater.set_transform(self.transform.to_mat4());
                    });
                } else {
                    model_renderer.update_instance(body_render_instance, |updater| {
                        updater.set_transform(self.transform.to_mat4());
                    });
                    model_renderer.update_instance(head_render_instance, |updater| {
                        updater.set_transform(self.transform.to_mat4());
                    });
                }
            }
        }
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
            ObjectDetail::Bipedal { .. } => {}
        }
    }

    pub fn interact_with_self(&mut self) {}

    /// Returns true if the object interacted with the other object. Returning false, will let the
    /// other object be selected.
    pub fn interact_with(&mut self, _object: &mut Object) -> bool {
        false
    }

    pub fn interact_with_terrain(&mut self, position: Vec3) {
        if let ObjectDetail::Bipedal {
            ref mut order,
            ref mut sequencer,
            ..
        } = self.detail
        {
            tracing::info!("{} -> terrain clicked at {:?}", self.title, position);

            let already_walking = matches!(order, BipedalOrder::MoveTo { .. });

            *order = BipedalOrder::MoveTo {
                target_location: position,
                speed: 1.6,
            };

            if !already_walking {
                if let Some(walk_sequence) = sequences().get_by_name("MSEQ_WALK") {
                    sequencer.play_sequence(walk_sequence);
                }
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
    MoveTo { target_location: Vec3, speed: f32 },
}
