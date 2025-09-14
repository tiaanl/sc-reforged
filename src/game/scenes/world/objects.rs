use std::path::PathBuf;

use crate::{
    engine::{gizmos::GizmoVertex, prelude::*},
    game::{
        animations::Sequencer,
        config::ObjectType,
        geometry_buffers::GeometryBuffers,
        height_map::HeightMap,
        image::BlendMode,
        math::Frustum,
        model::Model,
        models::models,
        renderer::ModelRenderer,
        scenes::world::{
            actions::PlayerAction,
            object::{Bipedal, BipedalOrder, Object, ObjectLike, Scenery, SceneryLit},
        },
        shadows::ShadowCascades,
    },
};

pub struct Objects {
    objects: Vec<Box<dyn ObjectLike>>,
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

                self.objects.push(Box::new(Bipedal {
                    object: Object {
                        title: title.to_string(),
                        object_type,
                        transform,
                        draw_debug_bones: false,
                        draw_bounding_spheres: false,
                    },
                    body_model,
                    body_render_instance,
                    head_model,
                    head_render_instance,
                    order: BipedalOrder::Stand,
                    sequencer: Sequencer::default(),

                    render_skeleton: false,
                }));
            }

            ObjectType::SceneryLit => {
                let model_handle = models().load_object_model(model_name)?;
                let model = models().get_mut(model_handle).unwrap();

                // Create a new model with only the light cone.
                let light_cone_model = match model.node_index_by_name("lightfcone") {
                    Some(light_cone_index) => {
                        let mut new_model = Model::from_skeleton(model.skeleton.clone());
                        // Clone the light cone from the original mesh to the new one. We can use the
                        // same node index, because we cloned the skeleton.
                        new_model.clone_meshes(model, light_cone_index, light_cone_index);
                        for mesh in new_model.meshes.iter_mut() {
                            mesh.blend_mode = BlendMode::Additive;
                        }

                        // Remove the light cone from the original mesh.
                        model.clear_meshes(light_cone_index);
                        Some(models().add(format!("{model_name}_light_cone"), new_model))
                    }
                    None => {
                        tracing::warn!("lightcone node not found on SceneryLit");
                        None
                    }
                };

                // Create the `render_instance` after possibly modifying the original model.
                let render_instance = self.model_renderer.add_render_instance(
                    model_handle,
                    transform.to_mat4(),
                    new_object_id,
                )?;

                self.objects.push(Box::new(SceneryLit {
                    scenery: Scenery {
                        object: Object {
                            title: title.to_string(),
                            object_type,
                            transform,
                            draw_debug_bones: false,
                            draw_bounding_spheres: false,
                        },
                        model: model_handle,
                        render_instance,
                    },
                    on: false,
                    light_cone_model,
                    light_cone_render_instance: None,
                }));
            }

            _ => {
                let model = models().load_object_model(model_name)?;

                let render_instance = self.model_renderer.add_render_instance(
                    model,
                    transform.to_mat4(),
                    new_object_id,
                )?;

                self.objects.push(Box::new(Scenery {
                    object: Object {
                        title: title.to_string(),
                        object_type,
                        transform,
                        draw_debug_bones: false,
                        draw_bounding_spheres: false,
                    },
                    model,
                    render_instance,
                }));
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
                if let Some(_object) = self.objects.get(id as usize) {
                    // if object.is_player_controlled() {
                    // Set a new selected object.
                    self.selected_object = Some(id);
                    // }
                }
            }
            PlayerAction::TerrainClicked { .. } => {
                // If the player clicked on the terrain and has a biped selected, issue an order
                // for the biped to move there.
                /*
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
                */
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
            object.render_gizmos(vertices);
            /*
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
            */
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

                        object.debug_panel(ui);
                    });
            }
        }
    }
}
