use crate::{
    engine::{gizmos::GizmosRenderer, prelude::*},
    game::{
        camera::Camera,
        geometry_buffers::GeometryBuffers,
        models::{ModelManager, RenderModel},
    },
};

/// Represents an object inside the game world.
#[derive(Debug)]
pub struct Object {
    pub translation: Vec3,
    pub rotation: Vec3,
    pub model: Handle<RenderModel>,
    pub visible: bool,
}

impl Object {
    pub fn new(translation: Vec3, rotation: Vec3, model: Handle<RenderModel>) -> Self {
        Self {
            translation,
            rotation,
            model,
            visible: true,
        }
    }
}

pub struct Objects {
    models: ModelManager,

    pub objects: Vec<Object>,

    /// The entity index that the mouse is currently over.
    selected_object: Option<usize>,
}

impl Objects {
    pub fn new(
        renderer: &Renderer,
        shaders: &mut Shaders,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let models = ModelManager::new(renderer, shaders, camera_bind_group_layout);

        Self {
            models,
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
    ) -> Result<(), AssetError> {
        let model = self.models.load_object(renderer, model_name)?;

        self.objects.push(Object {
            translation,
            rotation,
            model,
            visible: true,
        });

        Ok(())
    }

    pub fn get(&self, object_index: usize) -> Option<&Object> {
        self.objects.get(object_index)
    }

    pub fn get_mut(&mut self, object_index: usize) -> Option<&mut Object> {
        self.objects.get_mut(object_index)
    }

    pub fn set_selected(&mut self, selected: Option<usize>) {
        self.selected_object = selected;
    }

    pub fn update(&mut self, _camera: &Camera) {
        // let matrices = camera.calculate_matrices();
        // let proj_view = matrices.projection * matrices.view;
        // let frustum = Frustum::from(proj_view);

        // self.objects.iter_mut().for_each(|object| {
        //     let Some(model) = self.models.get(object.model) else {
        //         return;
        //     };

        //     let object_transform = Mat4::from_rotation_translation(
        //         Quat::from_euler(
        //             glam::EulerRot::XYZ,
        //             object.rotation.x,
        //             object.rotation.y,
        //             object.rotation.z,
        //         ),
        //         object.translation,
        //     );

        //     object.visible = model.collision_boxes.iter().any(|bounding_box| {
        //         let transform = object_transform; // * bounding_box.model_transform;
        //         let bbox = BoundingBox {
        //             min: (transform * bounding_box.min.extend(1.0)).xyz(),
        //             max: (transform * bounding_box.max.extend(1.0)).xyz(),
        //         };

        //         frustum.contains_bounding_box(&bbox)
        //     });
        // });
    }

    pub fn render_objects(
        &self,
        frame: &mut Frame,
        geometry_buffers: &GeometryBuffers,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        let mut set = self.models.new_render_set();
        for Object {
            model,
            translation,
            rotation,
            ..
        } in self.objects.iter()
        {
            // Because we're using a left handed coordinate system, the z rotations have
            // to be reversed.  (Why though!???)
            let rotation = Vec3::new(rotation.x, rotation.y, -rotation.z);

            let transform = Mat4::from_rotation_translation(
                Quat::from_euler(glam::EulerRot::XYZ, rotation.x, rotation.y, rotation.z),
                *translation,
            );

            set.push(*model, transform);
        }
        self.models
            .render_model_set(frame, geometry_buffers, camera_bind_group, set);
    }

    pub fn render_gizmos(
        &self,
        _frame: &mut Frame,
        _camera_bind_group: &wgpu::BindGroup,
        _gizmos_renderer: &GizmosRenderer,
    ) {
    }

    pub fn debug_panel(&mut self, _ui: &mut egui::Ui) {}
}
