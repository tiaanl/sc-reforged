use ahash::{HashMap, HashSet};

use crate::{
    engine::{
        gizmos::{GizmoVertex, GizmosRenderer},
        prelude::*,
        storage::Handle,
    },
    game::{
        animations::{Animation, Sequencer, animations, sequences},
        camera::Frustum,
        config::ObjectType,
        geometry_buffers::{GeometryBuffers, GeometryData, RenderTarget},
        model::Model,
        models::models,
        renderer::{ModelRenderer, RenderAnimation, RenderInstance},
        scenes::world::actions::PlayerAction,
        skeleton::Skeleton,
    },
};

impl ObjectType {}

/// Represents an object inside the game world.a
pub struct Object {
    pub title: String,
    pub object_type: ObjectType,
    pub transform: Transform,
    pub model_handle: Handle<Model>,
    pub render_instance: Handle<RenderInstance>,

    pub sequencer: Sequencer,

    /// Whether to draw the bones of the skeleton.
    pub draw_debug_bones: bool,
    /// Whether to draw the bounding sphere for each mesh.
    pub draw_bounding_spheres: bool,
    /// A list of node indices to draw in debug mode.
    pub selected_bones: HashSet<usize>,
}

impl Object {
    pub fn update(&mut self, delta_time: f32) {
        self.sequencer.update(delta_time);
    }

    pub fn clear_animation(&mut self) {
        self.sequencer.stop();
    }

    pub fn handle_player_action(&mut self, player_action: &PlayerAction) {
        match player_action {
            PlayerAction::Object { position, id } => {
                tracing::info!("{} -> object clicked {} at {:?}", self.title, id, position);
            }
            PlayerAction::Terrain { position } => {
                tracing::info!("{} -> terrain clicked at {:?}", self.title, position);
            }
        }
    }
}

pub struct Objects {
    model_renderer: ModelRenderer,
    render_animation_lookup: HashMap<Handle<Animation>, Handle<RenderAnimation>>,

    pub objects: Vec<Object>,

    /// The entity index that the mouse is currently over.
    selected_object: Option<u32>,
}

impl Objects {
    pub fn new(
        shaders: &mut Shaders,
        shadow_render_target: &RenderTarget,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        environment_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self, AssetError> {
        let model_renderer = ModelRenderer::new(
            shaders,
            shadow_render_target,
            camera_bind_group_layout,
            environment_bind_group_layout,
        );

        // Add all the sequence animations to the renderer.
        // for (name, sequence) in sequences().lookup() {
        //     let sequence = sequences().get(*sequence).unwrap();

        //     for clip in sequence.clips.iter() {
        //         model_renderer.add_animation(
        //     }
        // }

        Ok(Self {
            model_renderer,
            render_animation_lookup: HashMap::default(),

            objects: vec![],
            selected_object: None,
        })
    }

    pub fn spawn(
        &mut self,
        transform: Transform,
        model_name: &str,
        title: &str,
        object_type: ObjectType,
    ) -> Result<(), AssetError> {
        let model_handle = if object_type.is_bipedal() {
            models().load_bipedal_model(model_name)
        } else {
            models().load_object_model(model_name)
        }?;

        let model_instance_handle = self.model_renderer.add_render_instance(
            model_handle,
            transform.to_mat4(),
            self.objects.len() as u32,
        )?;

        self.objects.push(Object {
            title: title.to_string(),
            object_type,
            transform,
            model_handle,
            render_instance: model_instance_handle,

            sequencer: Sequencer::default(),

            draw_debug_bones: false,
            draw_bounding_spheres: false,
            selected_bones: HashSet::default(),
        });

        Ok(())
    }

    pub fn update(
        &mut self,
        delta_time: f32,
        input: &InputState,
        geometry_data: Option<&GeometryData>,
    ) {
        let delta_time = delta_time / 100.0;

        // Update sequencers.
        self.objects
            .iter_mut()
            .filter(|object| object.sequencer.is_playing())
            .for_each(|object| {
                object.update(delta_time);

                // if let Some(animation_state) = object.sequencer.get_animation_state() {
                //     self.model_renderer
                //         .update_instance(object.render_instance, |updater| {
                //             updater.set_animation(animation_state.animation, animation_state.time);
                //         });
                // }
            });

        if input.mouse_just_pressed(MouseButton::Left) {
            if let Some(geometry_data) = geometry_data {
                if geometry_data.id < 1 << 16 {
                    self.selected_object = Some(geometry_data.id);
                    return;
                }
            }

            self.selected_object = None;
        }
    }

    pub fn handle_player_action(&mut self, player_action: &PlayerAction) {
        if let Some(object) = self.selected_object {
            let object = match self.objects.get_mut(object as usize) {
                Some(object) => object,
                None => {
                    // The ID to an object is not found, so it must be a stale ID, just clear it.
                    todo!("clear the id");
                }
            };

            object.handle_player_action(player_action);
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
            let Some(model) = models().get(object.model_handle) else {
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

        if let Some(animation_state) = object.sequencer.get_animation_state() {
            if let Some(animation) = animations().get(animation_state.animation) {
                let skeleton =
                    animation.sample_pose(animation_state.time, 30.0, &model.skeleton, true);
                do_node(&skeleton, object.transform.to_mat4(), 0, vertices, 0);
            }
        } else {
            do_node(&model.skeleton, object.transform.to_mat4(), 0, vertices, 0);
        }
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

                        egui::ComboBox::from_label("Sequences").show_ui(ui, |ui| {
                            use crate::game::animations::sequences;

                            for (name, sequence) in sequences().lookup() {
                                if ui.button(name).clicked() {
                                    // Safety: Sequence must be there, because we're iterating
                                    // a known list of sequences.
                                    object.sequencer.play(*sequence);
                                }
                            }
                        });

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

                            self.model_renderer.update_instance(
                                object.render_instance,
                                |updater| {
                                    updater.set_transform(object.transform.to_mat4());
                                },
                            );
                        }

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

                        ui.heading("Debug");
                        ui.checkbox(&mut object.draw_debug_bones, "Draw debug bones");
                        ui.checkbox(&mut object.draw_bounding_spheres, "Draw bounding spheres");
                    });
            }
        }
    }
}
