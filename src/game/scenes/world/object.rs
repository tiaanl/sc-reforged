use glam::{Mat3, Mat4, Quat, Vec2, Vec3, Vec4};

use crate::{
    engine::{
        gizmos::{GizmoVertex, GizmosRenderer},
        prelude::Transform,
        storage::Handle,
    },
    game::{
        animations::{Sequencer, sequences},
        config::ObjectType,
        height_map::HeightMap,
        math::BoundingSphere,
        model::Model,
        models::models,
        renderer::{ModelRenderer, RenderInstance},
        skeleton::Skeleton,
    },
};

pub trait ObjectLike {
    /// Return true if the player can select this object and control it.
    fn _is_player_controlled(&self) -> bool {
        false
    }

    fn update(
        &mut self,
        delta_time: f32,
        height_map: &HeightMap,
        model_renderer: &mut ModelRenderer,
    );

    fn render_gizmos(&self, vertices: &mut Vec<GizmoVertex>);

    fn debug_panel(&mut self, ui: &mut egui::Ui);
}

/// Represents an object inside the game world.
pub struct Object {
    pub title: String,
    pub object_type: ObjectType,
    pub transform: Transform,

    /// Whether to draw the bones of the skeleton.
    pub draw_debug_bones: bool,
    /// Whether to draw the bounding sphere for each mesh.
    pub draw_bounding_spheres: bool,
}

impl Object {
    fn render_bounding_sphere(
        &self,
        bounding_sphere: &BoundingSphere,
        vertices: &mut Vec<GizmoVertex>,
    ) {
        let world_position =
            self.transform.to_mat4() * Mat4::from_translation(bounding_sphere.center);

        vertices.extend(GizmosRenderer::create_iso_sphere(
            world_position,
            bounding_sphere.radius,
            32,
        ));
    }
}

impl ObjectLike for Object {
    fn update(
        &mut self,
        _delta_time: f32,
        _height_map: &HeightMap,
        _model_renderer: &mut ModelRenderer,
    ) {
    }

    fn render_gizmos(&self, _vertices: &mut Vec<GizmoVertex>) {}

    fn debug_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading(format!("{} ({:?})", self.title, self.object_type));

        let mut euler_rot = Vec3::from(self.transform.rotation.to_euler(glam::EulerRot::default()));

        fn drag_vec3(
            ui: &mut egui::Ui,
            label: &str,
            value: &mut Vec3,
            step: f32,
        ) -> egui::Response {
            use egui::Widget;
            use egui::widgets::DragValue;

            ui.label(label);

            let x = DragValue::new(&mut value.x).speed(step).ui(ui);
            let y = DragValue::new(&mut value.y).speed(step).ui(ui);
            let z = DragValue::new(&mut value.z).speed(step).ui(ui);

            x | y | z
        }

        egui::Grid::new("object_detail")
            .num_columns(4)
            .show(ui, |ui| {
                let a = drag_vec3(ui, "Position", &mut self.transform.translation, 1.0);

                ui.end_row();

                let b = drag_vec3(ui, "Rotation", &mut euler_rot, 0.01);

                if (a | b).changed() {
                    self.transform.rotation = Quat::from_euler(
                        glam::EulerRot::default(),
                        euler_rot.x,
                        euler_rot.y,
                        euler_rot.z,
                    );

                    // self.update_model_renderer(model_renderer);
                }
            });

        ui.heading("Debug");
        ui.checkbox(&mut self.draw_debug_bones, "Draw debug bones");
        ui.checkbox(&mut self.draw_bounding_spheres, "Draw bounding spheres");
    }
}

pub struct Scenery {
    pub object: Object,
    pub model: Handle<Model>,
    pub render_instance: Handle<RenderInstance>,
}

impl ObjectLike for Scenery {
    fn update(
        &mut self,
        delta_time: f32,
        height_map: &HeightMap,
        model_renderer: &mut ModelRenderer,
    ) {
        self.object.update(delta_time, height_map, model_renderer);

        model_renderer.update_instance(self.render_instance, |updater| {
            updater.set_transform(self.object.transform.to_mat4());
        });
    }

    fn render_gizmos(&self, vertices: &mut Vec<GizmoVertex>) {
        if self.object.draw_bounding_spheres {
            let model = models().get(self.model).expect("Missing model!");
            self.object
                .render_bounding_sphere(&model.bounding_sphere, vertices);
        }
    }

    fn debug_panel(&mut self, ui: &mut egui::Ui) {
        self.object.debug_panel(ui);
    }
}

pub struct SceneryLit {
    pub scenery: Scenery,

    /// Set to true if the light is on.
    pub on: bool,

    pub light_cone_model: Option<Handle<Model>>,
    pub light_cone_render_instance: Option<Handle<RenderInstance>>,
}

impl ObjectLike for SceneryLit {
    fn update(
        &mut self,
        _delta_time: f32,
        _height_map: &HeightMap,
        model_renderer: &mut ModelRenderer,
    ) {
        if let Some(light_cone_model) = self.light_cone_model {
            if self.on && self.light_cone_render_instance.is_none() {
                self.light_cone_render_instance = match model_renderer.add_render_instance(
                    light_cone_model,
                    self.scenery.object.transform.to_mat4(),
                    0,
                ) {
                    Ok(render_instance) => Some(render_instance),
                    Err(err) => {
                        tracing::warn!("Could not add render instance! {err}");
                        None
                    }
                };
            }
        }

        if !self.on && self.light_cone_render_instance.is_some() {
            model_renderer.remove_model_instance(self.light_cone_render_instance.unwrap());
            self.light_cone_render_instance = None;
        }

        model_renderer.update_instance(self.scenery.render_instance, |updater| {
            updater.set_transform(self.scenery.object.transform.to_mat4());
        });

        if let Some(render_instance) = self.light_cone_render_instance {
            model_renderer.update_instance(render_instance, |updater| {
                updater.set_transform(self.scenery.object.transform.to_mat4());
            });
        }
    }

    fn render_gizmos(&self, vertices: &mut Vec<GizmoVertex>) {
        self.scenery.render_gizmos(vertices);
    }

    fn debug_panel(&mut self, ui: &mut egui::Ui) {
        self.scenery.debug_panel(ui);
        ui.checkbox(&mut self.on, "On");
    }
}

pub struct Bipedal {
    pub object: Object,
    pub body_model: Handle<Model>,
    pub body_render_instance: Handle<RenderInstance>,
    pub head_model: Handle<Model>,
    pub head_render_instance: Handle<RenderInstance>,
    pub order: BipedalOrder,
    pub sequencer: Sequencer,

    /// Whether to render the gizmo skeleton.
    pub render_skeleton: bool,
}

impl Bipedal {
    fn render_skeleton_gizmo(&self, model: &Model, vertices: &mut Vec<GizmoVertex>) {
        fn do_node(
            skeleton: &Skeleton,
            transform: Mat4,
            bone_index: u32,
            vertices: &mut Vec<GizmoVertex>,
            depth: usize,
        ) {
            const COLORS: &[Vec4] = &[
                Vec4::new(1.0, 0.0, 0.0, 1.0),
                Vec4::new(0.0, 1.0, 0.0, 1.0),
                Vec4::new(0.0, 0.0, 1.0, 1.0),
                Vec4::new(1.0, 1.0, 0.0, 1.0),
                Vec4::new(1.0, 0.0, 1.0, 1.0),
                Vec4::new(1.0, 1.0, 1.0, 1.0),
            ];

            let color = COLORS[depth % COLORS.len()];

            for (child_index, child_node) in skeleton
                .bones
                .iter()
                .enumerate()
                .filter(|(_, bone)| bone.parent == bone_index)
            {
                let start_position = transform.transform_point3(Vec3::ZERO);

                let end_transform = transform * child_node.transform.to_mat4();
                let end_position = end_transform.transform_point3(Vec3::ZERO);

                vertices.push(GizmoVertex::new(start_position, color));
                vertices.push(GizmoVertex::new(end_position, color));

                // if depth == 0 {
                //     continue;
                // }

                do_node(
                    skeleton,
                    end_transform,
                    child_index as u32,
                    vertices,
                    depth + 1,
                );
            }
        }

        // if let Some(animation_state) = object.sequencer.get_animation_state() {
        //     if let Some(animation) = animations().get(animation_state.animation) {
        //         let skeleton =
        //             animation.sample_pose(animation_state.time, 30.0, &model.skeleton, true);
        //         do_node(&skeleton, object.transform.to_mat4(), 0, vertices, 0);
        //     }
        // } else {
        do_node(
            &model.skeleton,
            self.object.transform.to_mat4(),
            0,
            vertices,
            0,
        );
        // }
    }
}

impl ObjectLike for Bipedal {
    fn _is_player_controlled(&self) -> bool {
        true
    }

    fn update(
        &mut self,
        delta_time: f32,
        height_map: &HeightMap,
        model_renderer: &mut ModelRenderer,
    ) {
        self.sequencer.update(delta_time);

        match self.order {
            BipedalOrder::Stand => {}
            BipedalOrder::_MoveTo {
                target_location,
                speed,
            } => {
                let current_xy = self.object.transform.translation.truncate();
                let (current_pos, current_normal) =
                    height_map.world_position_and_normal(current_xy);

                // Snap to the ground.
                self.object.transform.translation.z = current_pos.z;

                // Create a vector to the target.
                let to_target_xy = target_location - current_xy;
                let distance_to_target = to_target_xy.length();

                // Arrived already?
                if distance_to_target <= speed * delta_time {
                    let (target_pos, _) = height_map.world_position_and_normal(target_location);

                    self.object.transform.translation = target_pos;

                    let forward = to_target_xy.extend(0.0).normalize();
                    let left = forward.cross(Vec3::Z).normalize_or_zero();
                    let basis = Mat3::from_cols(left, forward, Vec3::Z);
                    self.object.transform.rotation = Quat::from_mat3(&basis);

                    // Issue a *stand* order.
                    self.order = BipedalOrder::Stand;
                    if let Some(stand_sequence) = sequences().get_by_name("MSEQ_STAND") {
                        self.sequencer.play_sequence(stand_sequence);
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

                    self.object.transform.translation = new_pos;

                    let forward = to_target_xy.extend(0.0).normalize();
                    let left = forward.cross(Vec3::Z).normalize_or_zero();
                    let basis = Mat3::from_cols(left, forward, Vec3::Z);
                    self.object.transform.rotation = Quat::from_mat3(&basis);
                }
            }
        }

        let mut animation = None;
        let mut time = 0.0;
        if let Some(animation_state) = self.sequencer.get_animation_state() {
            animation = Some(
                model_renderer.get_or_insert_animation(self.body_model, animation_state.animation),
            );
            time = animation_state.frame;
        }

        model_renderer.update_instance(self.body_render_instance, |updater| {
            updater.set_transform(self.object.transform.to_mat4());
            if let Some(animation) = animation {
                updater.set_animation(animation, time);
            }
        });
        model_renderer.update_instance(self.head_render_instance, |updater| {
            updater.set_transform(self.object.transform.to_mat4());
            if let Some(animation) = animation {
                updater.set_animation(animation, time);
            }
        });
    }

    fn render_gizmos(&self, vertices: &mut Vec<GizmoVertex>) {
        let body_model = models().get(self.body_model).expect("Missing body model!");
        let head_model = models().get(self.head_model).expect("Missing body model!");

        if self.object.draw_bounding_spheres {
            self.object
                .render_bounding_sphere(&body_model.bounding_sphere, vertices);

            self.object
                .render_bounding_sphere(&head_model.bounding_sphere, vertices);
        }

        if self.render_skeleton {
            self.render_skeleton_gizmo(body_model, vertices);
        }
    }

    fn debug_panel(&mut self, ui: &mut egui::Ui) {
        self.object.debug_panel(ui);

        // Show a few common sequences.
        const COMMON: &[&str] = &[
            "MSEQ_STAND",
            "MSEQ_PRONE",
            "MSEQ_CROUCH",
            "MSEQ_ON_BACK",
            "MSEQ_SIT",
        ];
        for name in COMMON {
            if let Some(seq) = sequences().get_by_name(name) {
                if ui.button(*name).clicked() {
                    self.sequencer.play_sequence(seq);
                }
            } else {
                println!("not found {name}");
            }
        }

        egui::ComboBox::from_label("Sequencer").show_ui(ui, |ui| {
            use crate::game::animations::sequences;

            let mut sequences = sequences()
                .lookup()
                .map(|(name, seq)| (name.clone(), *seq))
                .collect::<Vec<_>>();
            sequences.sort_by(|(left, _), (right, _)| left.cmp(right));

            for (name, sequence) in sequences.iter() {
                if ui.button(name).clicked() {
                    // Safety: Sequence must be there, because we're iterating
                    // a known list of sequences.
                    self.sequencer.play_sequence(*sequence);
                }
            }
        });
        self.sequencer.debug_panel(ui);

        if let Some(animation_state) = self.sequencer.get_animation_state() {
            ui.label(format!("Animation: {}", animation_state.animation));
            ui.label(format!("Time: {}", animation_state.frame));
        }
    }
}

pub enum BipedalOrder {
    Stand,
    _MoveTo { target_location: Vec2, speed: f32 },
}
