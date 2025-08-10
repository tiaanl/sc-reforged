use std::path::PathBuf;

use ahash::HashSet;

use crate::{
    engine::{
        gizmos::{GizmoVertex, GizmosRenderer},
        prelude::*,
        storage::Handle,
    },
    game::{
        animations::{Animation, animations},
        config::ObjectType,
        geometry_buffers::{GeometryBuffers, GeometryData},
        model::{Model, Node},
        models::models,
        renderer::{ModelRenderer, RenderAnimation, RenderInstance},
    },
};

/// Represents an object inside the game world.a
#[derive(Debug)]
pub struct Object {
    pub title: String,
    pub object_type: ObjectType,
    pub transform: Transform,
    pub model_handle: Handle<Model>,
    pub render_instance: Handle<RenderInstance>,

    pub animation: Option<Handle<Animation>>,
    pub animation_time: f32,

    /// Whether to draw the bones of the skeleton.
    pub draw_debug_bones: bool,
    /// Whether to draw the bounding sphere for each mesh.
    pub draw_bounding_spheres: bool,
    /// A list of node indices to draw in debug mode.
    pub selected_nodes: HashSet<usize>,
}

impl Object {
    pub fn update(&mut self, delta_time: f32) {
        // if self.animation.is_some() {
        self.animation_time += delta_time;
        // }
    }

    pub fn set_animation(&mut self, animation: Handle<Animation>) {
        self.animation = Some(animation);
        self.animation_time = 0.0;
    }

    pub fn clear_animation(&mut self) {
        self.animation = None;
        self.animation_time = 0.0;
    }
}

pub struct Objects {
    model_renderer: ModelRenderer,

    pub objects: Vec<Object>,

    /// The entity index that the mouse is currently over.
    selected_object: Option<u32>,

    walking_animation: Handle<RenderAnimation>,
    running_animation: Handle<RenderAnimation>,
    crouching_animation: Handle<RenderAnimation>,
    crawling_animation: Handle<RenderAnimation>,
}

impl Objects {
    pub fn new(
        shaders: &mut Shaders,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        environment_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let mut model_renderer = ModelRenderer::new(
            shaders,
            camera_bind_group_layout,
            environment_bind_group_layout,
        );

        let man_skel = models()
            .load_bipedal_model("man_skel")
            .expect("Could not load default skeleton");

        let walking_animation = {
            let anim = animations()
                .load(PathBuf::from("motions").join("bipedal_walk.bmf"))
                .expect("Could not load walking animation");

            model_renderer.add_animation(man_skel, anim)
        };

        let running_animation = {
            let anim = animations()
                .load(PathBuf::from("motions").join("bipedal_stand_run.bmf"))
                .expect("Could not load walking animation");

            model_renderer.add_animation(man_skel, anim)
        };

        let crouching_animation = {
            let anim = animations()
                .load(PathBuf::from("motions").join("bipedal_crouchwalk_cycle.bmf"))
                .expect("Could not load walking animation");

            model_renderer.add_animation(man_skel, anim)
        };

        let crawling_animation = {
            let anim = animations()
                .load(PathBuf::from("motions").join("bipedal_prone_low_crawl.bmf"))
                .expect("Could not load walking animation");

            model_renderer.add_animation(man_skel, anim)
        };

        Self {
            model_renderer,
            objects: vec![],
            selected_object: None,
            walking_animation,
            running_animation,
            crouching_animation,
            crawling_animation,
        }
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

            animation: None,
            animation_time: 0.0,

            draw_debug_bones: false,
            draw_bounding_spheres: false,
            selected_nodes: HashSet::default(),
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

        self.objects.iter_mut().for_each(|object| {
            object.update(delta_time);

            self.model_renderer
                .update_instance(object.render_instance, |updater| {
                    updater.set_animation_time(object.animation_time);
                });
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

    pub fn render_objects(
        &mut self,
        frame: &mut Frame,
        geometry_buffers: &GeometryBuffers,
        camera_bind_group: &wgpu::BindGroup,
        environment_bind_group: &wgpu::BindGroup,
    ) {
        self.model_renderer.render(
            frame,
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
                Self::render_selected_nodes(object, model, vertices);
            }

            if object.draw_bounding_spheres {
                Self::render_bounding_spheres(object, model, vertices);
            }
        }
    }

    fn render_skeleton(object: &Object, model: &Model, vertices: &mut Vec<GizmoVertex>) {
        fn do_node(
            nodes: &[Node],
            transform: Mat4,
            node_index: u32,
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

            for (child_index, child_node) in nodes
                .iter()
                .enumerate()
                .filter(|(_, node)| node.parent == node_index)
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
                    nodes,
                    end_transform,
                    child_index as u32,
                    vertices,
                    depth + 1,
                );
            }
        }

        if let Some(animation) = object.animation.and_then(|handle| animations().get(handle)) {
            let pose = animation.sample_pose(object.animation_time, &model.nodes, true);
            let nodes = pose
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let mut node = model.nodes[i].clone();
                    node.transform = t.clone();
                    node
                })
                .collect::<Vec<_>>();

            do_node(&nodes, object.transform.to_mat4(), 0, vertices, 0);
        } else {
            do_node(&model.nodes, object.transform.to_mat4(), 0, vertices, 0);
        }
    }

    fn render_selected_nodes(object: &Object, model: &Model, vertices: &mut Vec<GizmoVertex>) {
        for node_index in object.selected_nodes.iter() {
            let transform = object.transform.to_mat4() * model.local_transform(*node_index as u32);

            vertices.extend(GizmosRenderer::create_iso_sphere(transform, 10.0, 8));
        }
    }

    fn render_bounding_spheres(object: &Object, model: &Model, vertices: &mut Vec<GizmoVertex>) {
        for mesh in model.meshes.iter() {
            let transform = object.transform.to_mat4()
                * model.local_transform(mesh.node_index)
                * Mat4::from_translation(mesh.bounding_sphere.center);

            vertices.extend(GizmosRenderer::create_iso_sphere(
                transform,
                mesh.bounding_sphere.radius,
                32,
            ));
        }
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

                        if ui.button("Walking").clicked() {
                            self.model_renderer.update_instance(
                                object.render_instance,
                                |updater| {
                                    updater.set_animation(self.walking_animation);
                                },
                            );
                        }

                        if ui.button("Running").clicked() {
                            self.model_renderer.update_instance(
                                object.render_instance,
                                |updater| {
                                    updater.set_animation(self.running_animation);
                                },
                            );
                        }

                        if ui.button("Crouching").clicked() {
                            self.model_renderer.update_instance(
                                object.render_instance,
                                |updater| {
                                    updater.set_animation(self.crouching_animation);
                                },
                            );
                        }

                        if ui.button("Crawling").clicked() {
                            self.model_renderer.update_instance(
                                object.render_instance,
                                |updater| {
                                    updater.set_animation(self.crawling_animation);
                                },
                            );
                        }

                        if ui.button("Clear animation").clicked() {
                            object.clear_animation();
                            self.model_renderer
                                .update_instance(object.render_instance, |updater| {
                                    updater.clear_animation()
                                });
                        }

                        if let Some(model) = models().get(object.model_handle) {
                            use crate::game::model::Node;

                            ui.heading("Skeleton");
                            fn do_node(
                                ui: &mut egui::Ui,
                                nodes: &[Node],
                                parent_index: u32,
                                selected_nodes: &mut HashSet<usize>,
                            ) {
                                nodes
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
                                            do_node(ui, nodes, node_index as u32, selected_nodes);
                                        });
                                    });
                            }

                            do_node(ui, &model.nodes, 0xFFFF_FFFF, &mut object.selected_nodes);
                        }

                        ui.heading("Debug");
                        ui.checkbox(&mut object.draw_debug_bones, "Draw debug bones");
                        ui.checkbox(&mut object.draw_bounding_spheres, "Draw bounding spheres");
                    });
            }
        }
    }
}
