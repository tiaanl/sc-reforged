use std::path::PathBuf;

use crate::{
    engine::{
        gizmos::{GizmoVertex, GizmosRenderer},
        prelude::*,
    },
    game::{
        animations::{Sequencer, sequences},
        config::ObjectType,
        geometry_buffers::GeometryBuffers,
        height_map::HeightMap,
        math::Frustum,
        model::Model,
        models::models,
        renderer::ModelRenderer,
        scenes::world::{
            actions::PlayerAction,
            object::{BipedalOrder, Object, ObjectDetail},
        },
        shadows::ShadowCascades,
        skeleton::Skeleton,
    },
};

pub struct Objects {
    objects: Vec<Object>,
    model_renderer: ModelRenderer,

    selected_object: Option<u32>,
}

impl Objects {
    pub fn new(
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        environment_bind_group_layout: &wgpu::BindGroupLayout,
        shadow_cascades: &ShadowCascades,
    ) -> Result<Self, AssetError> {
        let objects = vec![];
        let model_renderer = ModelRenderer::new(
            camera_bind_group_layout,
            environment_bind_group_layout,
            shadow_cascades,
        );

        Ok(Self {
            objects,
            model_renderer,

            selected_object: None,
        })
    }

    pub fn spawn(
        &mut self,
        transform: Transform,
        object_type: ObjectType,
        model_name: &str,
        title: &str,
    ) -> Result<(), AssetError> {
        let new_object_id = self.objects.len() as u32;

        match object_type {
            ObjectType::Bipedal => {
                let body_model = models().load_model(
                    model_name,
                    PathBuf::from("models")
                        .join("people")
                        .join("bodies")
                        .join(model_name)
                        .join(model_name)
                        .with_extension("smf"),
                )?;

                let head_model_name = "head_john";
                let head_model = models().load_model(
                    head_model_name,
                    PathBuf::from("models")
                        .join("people")
                        .join("heads")
                        .join(head_model_name)
                        .join(head_model_name)
                        .with_extension("smf"),
                )?;

                let body_render_instance = self.model_renderer.add_render_instance(
                    body_model,
                    transform.to_mat4(),
                    new_object_id,
                )?;

                let head_render_instance = self.model_renderer.add_render_instance(
                    head_model,
                    transform.to_mat4(),
                    new_object_id,
                )?;

                self.objects.push(Object {
                    title: title.to_string(),
                    object_type,
                    transform,
                    detail: ObjectDetail::Bipedal {
                        body_model,
                        body_render_instance,
                        head_render_instance,
                        order: BipedalOrder::Stand,
                        sequencer: Sequencer::default(),
                    },
                    draw_debug_bones: false,
                    draw_bounding_spheres: false,
                    selected_bones: Default::default(),
                })
            }
            _ => {
                let model = models().load_object_model(model_name)?;

                let render_instance = self.model_renderer.add_render_instance(
                    model,
                    transform.to_mat4(),
                    new_object_id,
                )?;

                self.objects.push(Object {
                    title: title.to_string(),
                    object_type,
                    transform,
                    detail: ObjectDetail::Scenery {
                        model,
                        render_instance,
                    },
                    draw_debug_bones: false,
                    draw_bounding_spheres: false,
                    selected_bones: Default::default(),
                })
            }
        }

        Ok(())
    }

    pub fn update(&mut self, delta_time: f32, height_map: &HeightMap) {
        for object in self.objects.iter_mut() {
            object.update(delta_time, height_map, &mut self.model_renderer);
        }
    }

    pub fn handle_player_action(&mut self, player_action: &PlayerAction) {
        match *player_action {
            PlayerAction::ClearSelection => {
                // Deselect the selected object, if any.
                self.selected_object = None;
            }
            PlayerAction::ObjectClicked { id, .. } => {
                // Set a new selected object.
                self.selected_object = Some(id);
            }
            PlayerAction::TerrainClicked { position } => {
                // If the player clicked on the terrain and has a biped selected, issue an order
                // for the biped to move there.
                if let Some(selected_id) = self.selected_object {
                    if let Some(object) = self.objects.get_mut(selected_id as usize) {
                        if let ObjectDetail::Bipedal {
                            ref mut order,
                            ref mut sequencer,
                            ..
                        } = object.detail
                        {
                            *order = BipedalOrder::MoveTo {
                                target_location: position.truncate(),
                                speed: 200.0,
                            };

                            if let Some(walk_sequence) = sequences().get_by_name("MSEQ_WALK") {
                                sequencer.play_sequence(walk_sequence);
                            } else {
                                tracing::warn!("Could not play MSEQ_WALK");
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn render_shadow_casters(&mut self, frame: &mut Frame, shadow_cascades: &ShadowCascades) {
        self.model_renderer
            .render_shadow_casters(frame, shadow_cascades);
    }

    pub fn render_objects(
        &mut self,
        frame: &mut Frame,
        frustum: &Frustum,
        geometry_buffers: &GeometryBuffers,
        camera_bind_group: &wgpu::BindGroup,
        environment_bind_group: &wgpu::BindGroup,
        shadow_cascades: &ShadowCascades,
    ) {
        let _z = tracy_client::span!("render objects");
        self.model_renderer.render(
            frame,
            frustum,
            geometry_buffers,
            camera_bind_group,
            environment_bind_group,
            shadow_cascades,
        );
    }

    pub fn render_gizmos(&self, vertices: &mut Vec<GizmoVertex>) {
        for object in self.objects.iter() {
            let model = match object.detail {
                ObjectDetail::Scenery { model, .. } => model,
                ObjectDetail::SceneryLit { model, .. } => model,
                ObjectDetail::Bipedal { body_model, .. } => body_model,
            };

            let Some(model) = models().get(model) else {
                continue;
            };

            if object.draw_debug_bones {
                Self::render_skeleton(object, model, vertices);
                Self::render_selected_bones(object, model, vertices);
            }

            if object.draw_bounding_spheres {
                Self::render_bounding_sphere(object, model, vertices);
            }
        }
    }

    fn render_skeleton(object: &Object, model: &Model, vertices: &mut Vec<GizmoVertex>) {
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
        do_node(&model.skeleton, object.transform.to_mat4(), 0, vertices, 0);
        // }
    }

    fn render_selected_bones(object: &Object, model: &Model, vertices: &mut Vec<GizmoVertex>) {
        for bone_index in object.selected_bones.iter() {
            let transform =
                object.transform.to_mat4() * model.skeleton.local_transform(*bone_index as u32);

            vertices.extend(GizmosRenderer::create_iso_sphere(transform, 10.0, 8));
        }
    }

    fn render_bounding_sphere(object: &Object, model: &Model, vertices: &mut Vec<GizmoVertex>) {
        let world_position =
            object.transform.to_mat4() * Mat4::from_translation(model.bounding_sphere.center);

        vertices.extend(GizmosRenderer::create_iso_sphere(
            world_position,
            model.bounding_sphere.radius,
            32,
        ));
    }

    #[cfg(feature = "egui")]
    pub fn debug_panel(&mut self, egui: &egui::Context) {
        if let Some(selected_object) = self.selected_object {
            if let Some(object) = self.objects.get_mut(selected_object as usize) {
                egui::Window::new("Object")
                    .collapsible(false)
                    .resizable(false)
                    .show(egui, |ui| {
                        ui.set_width(300.0);

                        ui.heading(format!("{} ({:?})", object.title, object.object_type));

                        match object.detail {
                            ObjectDetail::Scenery { .. } | ObjectDetail::SceneryLit { .. } => {}
                            ObjectDetail::Bipedal {
                                ref mut sequencer, ..
                            } => {
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
                                            sequencer.play_sequence(seq);
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
                                            sequencer.play_sequence(*sequence);
                                        }
                                    }
                                });
                                sequencer.debug_panel(ui);

                                if let Some(animation_state) = sequencer.get_animation_state() {
                                    ui.label(format!("Animation: {}", animation_state.animation));
                                    ui.label(format!("Time: {}", animation_state.frame));
                                }
                            }
                        }

                        let mut euler_rot = Vec3::from(
                            object
                                .transform
                                .rotation
                                .to_euler(glam::EulerRot::default()),
                        );

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
                                let a = drag_vec3(
                                    ui,
                                    "Position",
                                    &mut object.transform.translation,
                                    1.0,
                                );

                                ui.end_row();

                                let b = drag_vec3(ui, "Rotation", &mut euler_rot, 0.01);

                                if (a | b).changed() {
                                    object.transform.rotation = Quat::from_euler(
                                        glam::EulerRot::default(),
                                        euler_rot.x,
                                        euler_rot.y,
                                        euler_rot.z,
                                    );

                                    object.update_model_renderer(&mut self.model_renderer);
                                }
                            });

                        ui.heading("Debug");
                        ui.checkbox(&mut object.draw_debug_bones, "Draw debug bones");
                        ui.checkbox(&mut object.draw_bounding_spheres, "Draw bounding spheres");
                    });
            }
        }
    }
}
