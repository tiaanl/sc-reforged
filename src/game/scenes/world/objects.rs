use slab::Slab;

use crate::{
    engine::{
        gizmos::{GizmoVertex, GizmosRenderer},
        prelude::*,
    },
    game::{
        camera::Camera,
        config::ObjectType,
        data_dir::data_dir,
        geometry_buffers::{GeometryBuffers, GeometryData},
        model::Model,
        model_renderer::{ModelInstanceHandle, ModelRenderer},
    },
};

/// Represents an object inside the game world.a
#[derive(Debug)]
pub struct Object {
    pub object_type: ObjectType,
    pub transform: Mat4,
    pub model_instance_handle: ModelInstanceHandle,
    /// Slab id for the [Model] stored in [Objects::models].
    pub model_index: usize,
    pub visible: bool,

    /// Whether to draw the bones of the skeleton.
    pub draw_debug_bones: bool,
}

pub struct Objects {
    model_renderer: ModelRenderer,

    /// Keep a local cache of models we loaded and added to the [ModelRenderer].
    models: Slab<Model>,

    pub objects: Vec<Object>,

    /// The entity index that the mouse is currently over.
    selected_object: Option<u32>,
}

impl Objects {
    pub fn new(
        renderer: &Renderer,
        shaders: &mut Shaders,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let model_renderer = ModelRenderer::new(renderer, shaders, camera_bind_group_layout);

        Self {
            // models,
            model_renderer,
            models: Slab::default(),
            objects: vec![],
            selected_object: None,
        }
    }

    pub fn spawn(
        &mut self,
        renderer: &Renderer,
        translation: Vec3,
        rotation: Vec3,
        model_name: &str,
        object_type: ObjectType,
    ) -> Result<(), AssetError> {
        let model_handle =
            self.model_renderer
                .add_model(renderer, model_name, object_type.is_bipedal())?;

        let model = if object_type.is_bipedal() {
            data_dir().load_bipedal_model(model_name)
        } else {
            data_dir().load_object_model(model_name)
        }?;
        let model_index = self.models.insert(model);

        // Because we're using a left handed coordinate system, the z rotations have
        // to be reversed.  (Why though!???)
        let rotation = Vec3::new(rotation.x, rotation.y, -rotation.z);

        let transform = Mat4::from_rotation_translation(
            Quat::from_euler(glam::EulerRot::XYZ, rotation.x, rotation.y, rotation.z),
            translation,
        );

        let model_instance_handle = self.model_renderer.add_model_instance(
            renderer,
            model_handle,
            transform,
            self.objects.len() as u32,
        );

        self.objects.push(Object {
            object_type,
            transform,
            model_instance_handle,
            model_index,
            visible: true,

            draw_debug_bones: false,
        });

        Ok(())
    }

    pub fn get(&self, object_index: usize) -> Option<&Object> {
        self.objects.get(object_index)
    }

    pub fn get_mut(&mut self, object_index: usize) -> Option<&mut Object> {
        self.objects.get_mut(object_index)
    }

    pub fn update(
        &mut self,
        _camera: &Camera,
        input: &InputState,
        geometry_data: Option<&GeometryData>,
    ) {
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
    ) {
        self.model_renderer
            .render(frame, geometry_buffers, camera_bind_group);
    }

    pub fn render_gizmos(&self, vertices: &mut Vec<GizmoVertex>) {
        for object in self.objects.iter() {
            if false {
                let Some(model) = self.model_renderer.get_model(object.model_instance_handle)
                else {
                    continue;
                };

                // scale.x == object max radius
                // scale.y == weight?  only vehicles seems to have anything other than 1.

                vertices.extend(GizmosRenderer::create_iso_sphere(
                    object.transform,
                    model.scale.y,
                    16,
                ));
                vertices.extend(GizmosRenderer::create_iso_sphere(
                    object.transform,
                    model.scale.z,
                    16,
                ));
            }

            let Some(model) = self.models.get(object.model_index) else {
                continue;
            };
            if object.draw_debug_bones {
                Self::render_skeleton(object.transform, model, vertices);
                // Self::render_bone_orientations(object.transform, model, vertices);
            }
        }
    }

    fn render_skeleton(transform: Mat4, model: &Model, vertices: &mut Vec<GizmoVertex>) {
        fn do_node(
            nodes: &[crate::game::model::Node],
            transform: Mat4,
            node_index: u32,
            vertices: &mut Vec<GizmoVertex>,
        ) {
            const COLORS: &[Vec4] = &[
                Vec4::new(1.0, 0.0, 0.0, 1.0),
                Vec4::new(0.0, 1.0, 0.0, 1.0),
                Vec4::new(0.0, 0.0, 1.0, 1.0),
                Vec4::new(1.0, 1.0, 0.0, 1.0),
                Vec4::new(1.0, 0.0, 1.0, 1.0),
                Vec4::new(1.0, 1.0, 1.0, 1.0),
            ];

            let color = COLORS[node_index as usize % COLORS.len()];

            for (child_index, child_node) in nodes
                .iter()
                .enumerate()
                .filter(|(_, node)| node.parent == node_index)
            {
                let start_position = transform.project_point3(Vec3::ZERO);

                let end_transform = transform * child_node.transform.to_mat4();
                let end_position = end_transform.transform_point3(Vec3::ZERO);

                vertices.push(GizmoVertex::new(start_position, color));
                vertices.push(GizmoVertex::new(end_position, color));

                do_node(nodes, end_transform, child_index as u32, vertices);
            }
        }

        do_node(&model.nodes, transform, 0, vertices);
    }

    fn render_bone_orientations(transform: Mat4, model: &Model, vertices: &mut Vec<GizmoVertex>) {
        for node in model.nodes.iter() {
            let t = transform * node.transform.to_mat4();
            vertices.extend(GizmosRenderer::create_axis(t, 10.0));
        }
    }

    #[cfg(feature = "egui")]
    pub fn debug_panel(&mut self, egui: &egui::Context) {
        if let Some(selected_object) = self.selected_object {
            if let Some(object) = self.objects.get_mut(selected_object as usize) {
                egui::Window::new("Object")
                    .default_open(false)
                    .show(egui, |ui| {
                        ui.label(format!("{:?}", object.object_type));

                        ui.checkbox(&mut object.draw_debug_bones, "Draw debug bones");
                    });
            }
        }
    }
}
