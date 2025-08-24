use std::path::PathBuf;

use ahash::HashSet;

use crate::{
    engine::{
        gizmos::{GizmoVertex, GizmosRenderer},
        prelude::*,
    },
    game::{
        animations::Sequencer,
        config::ObjectType,
        geometry_buffers::{GeometryBuffers, RenderTarget},
        math::Frustum,
        model::Model,
        models::models,
        renderer::ModelRenderer,
        scenes::world::{
            actions::PlayerAction,
            object::{BipedalOrder, Object, ObjectDetail},
        },
        skeleton::Skeleton,
    },
};

pub struct Objects {
    model_renderer: ModelRenderer,

    pub objects: Vec<Object>,

    /// The entity index that the mouse is currently over.
    selected_object: Option<u32>,
}

impl Objects {
    pub fn new(
        shadow_render_target: &RenderTarget,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        environment_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self, AssetError> {
        let model_renderer = ModelRenderer::new(
            shadow_render_target,
            camera_bind_group_layout,
            environment_bind_group_layout,
        );

        Ok(Self {
            model_renderer,

            objects: vec![],
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
        let new_entity_id = self.objects.len() as u32;

        let detail = match object_type {
            ObjectType::Ape | ObjectType::Bipedal => {
                let body_model = models().load_bipedal_model(model_name)?;
                let body_render_instance = self.model_renderer.add_render_instance(
                    body_model,
                    transform.to_mat4(),
                    self.objects.len() as u32,
                )?;

                let head_name = "head_john";

                let head_model = models().load_model(
                    head_name,
                    PathBuf::from("models")
                        .join("people")
                        .join("heads")
                        .join(head_name)
                        .join(head_name)
                        .with_extension("smf"),
                )?;
                let head_render_instance = self.model_renderer.add_render_instance(
                    head_model,
                    transform.to_mat4(),
                    new_entity_id,
                )?;

                ObjectDetail::Bipedal {
                    body_model,
                    body_render_instance,
                    head_render_instance,
                    order: BipedalOrder::Stand,
                    sequencer: Sequencer::default(),
                }
            }

            _ => {
                let model = models().load_object_model(model_name)?;
                let render_instance = self.model_renderer.add_render_instance(
                    model,
                    transform.to_mat4(),
                    new_entity_id,
                )?;
                ObjectDetail::Scenery {
                    model,
                    render_instance,
                }
            }
        };

        self.objects.push(Object {
            title: title.to_string(),
            object_type,
            transform,
            detail,

            draw_debug_bones: false,
            draw_bounding_spheres: false,
            selected_bones: HashSet::default(),
        });

        Ok(())
    }

    pub fn update(&mut self, delta_time: f32) {
        // Update sequencers.
        self.objects.iter_mut().for_each(|object| {
            object.update(delta_time, &mut self.model_renderer);
        });
    }

    pub fn handle_player_action(&mut self, player_action: &PlayerAction) {
        if let Some(selected_object) = self.selected_object {
            match *player_action {
                PlayerAction::Object { id, .. } => {
                    if selected_object == id {
                        let object = self.objects.get_mut(selected_object as usize).unwrap();
                        object.interact_with_self();
                    } else {
                        let (left, right, sel_is_left) = if selected_object < id {
                            let (l, r) = self.objects.split_at_mut(id as usize);
                            (l, r, true)
                        } else {
                            let (l, r) = self.objects.split_at_mut(selected_object as usize);
                            (l, r, false)
                        };

                        let (selected, clicked): (&mut Object, &mut Object) = if sel_is_left {
                            (&mut left[selected_object as usize], &mut right[0])
                        } else {
                            (&mut right[0], &mut left[id as usize])
                        };

                        if !selected.interact_with(clicked) {
                            self.selected_object = Some(id);
                        }
                    }
                }
                PlayerAction::Terrain { position } => {
                    let object = self.objects.get_mut(selected_object as usize).unwrap();
                    object.interact_with_terrain(position);
                }
            }
        } else {
            match *player_action {
                PlayerAction::Object { id, .. } => self.selected_object = Some(id),
                PlayerAction::Terrain { .. } => {
                    // With nothing selected, clickin on the terrain does nothing.
                }
            }
        }
    }

    pub fn render_shadow_casters(
        &mut self,
        frame: &mut Frame,
        shadow_render_target: &RenderTarget,
        frustum: &Frustum,
        environment_bind_group: &wgpu::BindGroup,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        self.model_renderer.render_shadow_casters(
            frame,
            shadow_render_target,
            frustum,
            environment_bind_group,
            camera_bind_group,
        );
    }

    pub fn render_objects(
        &mut self,
        frame: &mut Frame,
        frustum: &Frustum,
        geometry_buffers: &GeometryBuffers,
        camera_bind_group: &wgpu::BindGroup,
        environment_bind_group: &wgpu::BindGroup,
    ) {
        let _z = tracy_client::span!("render objects");
        self.model_renderer.render(
            frame,
            frustum,
            geometry_buffers,
            camera_bind_group,
            environment_bind_group,
        );
    }

    pub fn render_gizmos(&self, vertices: &mut Vec<GizmoVertex>) {
        for object in self.objects.iter() {
            let model = match object.detail {
                ObjectDetail::Scenery { model, .. } => model,
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

                        ui.heading("Transform");

                        let mut euler_rot = Vec3::from(
                            object
                                .transform
                                .rotation
                                .to_euler(glam::EulerRot::default()),
                        );

                        match object.detail {
                            ObjectDetail::Scenery { .. } => {}
                            ObjectDetail::Bipedal {
                                ref mut sequencer, ..
                            } => {
                                egui::ComboBox::from_label("Sequencer").show_ui(ui, |ui| {
                                    use crate::game::animations::sequences;

                                    for (name, sequence) in sequences().lookup() {
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
                                    ui.label(format!("Time: {}", animation_state.time));
                                }
                            }
                        }

                        fn drag_vec3(
                            ui: &mut egui::Ui,
                            label: &str,
                            value: &mut glam::Vec3,
                            speed: f32,
                        ) -> egui::Response {
                            ui.horizontal(|ui| {
                                use egui::DragValue;

                                let box_height = ui.text_style_height(&egui::TextStyle::Body);

                                ui.add_sized([60.0, box_height], egui::Label::new(label));

                                let x = ui.add_sized(
                                    [60.0, box_height],
                                    DragValue::new(&mut value.x).speed(speed),
                                );
                                let y = ui.add_sized(
                                    [60.0, box_height],
                                    DragValue::new(&mut value.y).speed(speed),
                                );
                                let z = ui.add_sized(
                                    [60.0, box_height],
                                    DragValue::new(&mut value.z).speed(speed),
                                );

                                x | y | z
                            })
                            .inner
                        }

                        let a = drag_vec3(ui, "Position", &mut object.transform.translation, 1.0);
                        let b = drag_vec3(ui, "Rotation", &mut euler_rot, 0.01);

                        if (a | b).changed() {
                            object.transform.rotation = Quat::from_euler(
                                glam::EulerRot::default(),
                                euler_rot.x,
                                euler_rot.y,
                                euler_rot.z,
                            );

                            // self.model_renderer.set_instance_transform(object.model_instance_handle, transform);

                            object.update_model_renderer(&mut self.model_renderer);
                        }

                        /*
                        if let Some(model) = models().get(object.model_handle) {
                            ui.heading("Skeleton");
                            fn do_node(
                                ui: &mut egui::Ui,
                                skeleton: &Skeleton,
                                parent_index: u32,
                                selected_nodes: &mut HashSet<usize>,
                            ) {
                                skeleton
                                    .bones
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, node)| node.parent == parent_index)
                                    .for_each(|(node_index, node)| {
                                        egui::CollapsingHeader::new(&node.name).show(ui, |ui| {
                                            let mut node_checked =
                                                selected_nodes.contains(&node_index);
                                            if ui.checkbox(&mut node_checked, "Highlight").changed()
                                            {
                                                if node_checked {
                                                    selected_nodes.insert(node_index);
                                                } else {
                                                    selected_nodes.remove(&node_index);
                                                }
                                            }
                                            do_node(
                                                ui,
                                                skeleton,
                                                node_index as u32,
                                                selected_nodes,
                                            );
                                        });
                                    });
                            }

                            do_node(ui, &model.skeleton, 0xFFFF_FFFF, &mut object.selected_bones);
                        }
                        */

                        ui.heading("Debug");
                        ui.checkbox(&mut object.draw_debug_bones, "Draw debug bones");
                        ui.checkbox(&mut object.draw_bounding_spheres, "Draw bounding spheres");
                    });
            }
        }
    }
}
