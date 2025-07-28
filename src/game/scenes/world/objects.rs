use crate::{
    engine::{gizmos::GizmosRenderer, prelude::*},
    game::{
        camera::Camera,
        config::ObjectType,
        geometry_buffers::GeometryBuffers,
        model_renderer::{ModelInstanceHandle, ModelRenderer},
    },
};

/// Represents an object inside the game world.a
#[derive(Debug)]
pub struct Object {
    pub transform: Mat4,
    pub model_instance_handle: ModelInstanceHandle,
    pub visible: bool,
}

impl Object {
    pub fn new(transform: Mat4, model_instance_handle: ModelInstanceHandle) -> Self {
        Self {
            transform,
            model_instance_handle,
            visible: true,
        }
    }
}

pub struct Objects {
    // models: ModelManager,
    model_renderer: ModelRenderer,

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
        // let models = ModelManager::new(renderer, shaders, camera_bind_group_layout);

        let model_renderer = ModelRenderer::new(renderer, shaders, camera_bind_group_layout);

        Self {
            // models,
            model_renderer,
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

        // Because we're using a left handed coordinate system, the z rotations have
        // to be reversed.  (Why though!???)
        let rotation = Vec3::new(rotation.x, rotation.y, -rotation.z);

        let transform = Mat4::from_rotation_translation(
            Quat::from_euler(glam::EulerRot::XYZ, rotation.x, rotation.y, rotation.z),
            translation,
        );

        let model_instance_handle =
            self.model_renderer
                .add_model_instance(renderer, model_handle, transform);

        self.objects.push(Object {
            transform,
            model_instance_handle,
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

    pub fn update(&mut self, _camera: &Camera) {}

    pub fn render_objects(
        &mut self,
        frame: &mut Frame,
        geometry_buffers: &GeometryBuffers,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        self.model_renderer
            .render(frame, geometry_buffers, camera_bind_group);
    }

    pub fn render_gizmos(
        &self,
        _frame: &mut Frame,
        _camera_bind_group: &wgpu::BindGroup,
        _gizmos_renderer: &GizmosRenderer,
    ) {
    }

    #[cfg(feature = "egui")]
    pub fn debug_panel(&mut self, _ui: &mut egui::Ui) {}
}
