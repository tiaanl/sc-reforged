use glam::Vec4Swizzles;

use crate::{
    engine::{gizmos::GizmosRenderer, prelude::*},
    game::{
        camera::{BoundingBox, Camera, Frustum, Ray},
        geometry_buffers::GeometryBuffers,
        model::Model,
        model_renderer::ModelRenderer,
        render::RenderTexture,
        storage::Storage,
    },
};

use super::bounding_boxes::{BoundingBoxRenderer, RawBoundingBox};

/// Represents an object inside the game world.
#[derive(Debug)]
pub struct Object {
    pub translation: Vec3,
    pub rotation: Vec3,
    pub model: Handle<Model>,
    pub visible: bool,
}

impl Object {
    pub fn new(translation: Vec3, rotation: Vec3, model: Handle<Model>) -> Self {
        Self {
            translation,
            rotation,
            model,
            visible: true,
        }
    }
}

pub struct Objects {
    asset_store: AssetStore,

    pub model_renderer: ModelRenderer,

    render_bounding_boxes: bool,
    pub bounding_box_renderer: BoundingBoxRenderer,

    pub objects: Vec<Object>,

    /// The entity index that the mouse is currently over.
    selected_object: Option<usize>,
}

impl Objects {
    pub fn new(
        asset_store: AssetStore,
        renderer: &Renderer,
        shaders: &mut Shaders,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let model_renderer = ModelRenderer::new(renderer, shaders, camera_bind_group_layout);

        let bounding_box_renderer = BoundingBoxRenderer::new(renderer, camera_bind_group_layout);

        Self {
            asset_store,
            model_renderer,
            render_bounding_boxes: false,
            bounding_box_renderer,
            objects: vec![],
            selected_object: None,
        }
    }

    pub fn spawn(&mut self, object: Object) {
        self.objects.push(object);
    }

    pub fn get(&self, object_index: usize) -> Option<&Object> {
        self.objects.get(object_index)
    }

    pub fn get_mut(&mut self, object_index: usize) -> Option<&mut Object> {
        self.objects.get_mut(object_index)
    }

    pub fn ray_intersection(&self, ray: &Ray) -> Option<usize> {
        let mut closest = f32::MAX;
        let mut closest_entity = None;

        // Gather up a list of bounding boxes for each entity/model.
        for (object_index, object) in self.objects.iter().enumerate() {
            let Some(model) = self.asset_store.get(object.model) else {
                continue;
            };

            let entity_transform = Mat4::from_rotation_translation(
                Quat::from_euler(
                    glam::EulerRot::XYZ,
                    object.rotation.x,
                    object.rotation.y,
                    object.rotation.z,
                ),
                object.translation,
            );

            for bounding_box in model.bounding_boxes.iter() {
                let transform = entity_transform * bounding_box.model_transform;
                let bbox =
                    RawBoundingBox::new(transform, bounding_box.min, bounding_box.max, false);
                if let Some(distance) = bbox.intersect_ray(ray) {
                    if distance < closest {
                        closest = distance;
                        closest_entity = Some(object_index);
                    }
                }
            }
        }

        closest_entity
    }

    pub fn set_selected(&mut self, selected: Option<usize>) {
        self.selected_object = selected;
    }

    pub fn update(&mut self, camera: &Camera) {
        let matrices = camera.calculate_matrices();
        let proj_view = matrices.projection * matrices.view;
        let frustum = Frustum::from(proj_view);

        self.objects.iter_mut().for_each(|object| {
            if let Some(model) = self.asset_store.get(object.model) {
                let object_transform = Mat4::from_rotation_translation(
                    Quat::from_euler(
                        glam::EulerRot::XYZ,
                        object.rotation.x,
                        object.rotation.y,
                        object.rotation.z,
                    ),
                    object.translation,
                );

                object.visible = model.bounding_boxes.iter().any(|bounding_box| {
                    let transform = object_transform * bounding_box.model_transform;
                    let bbox = BoundingBox {
                        min: (transform * bounding_box.min.extend(1.0)).xyz(),
                        max: (transform * bounding_box.max.extend(1.0)).xyz(),
                    };

                    frustum.contains_bounding_box(&bbox)
                });
            }
        });
    }

    pub fn render_objects(
        &self,
        frame: &mut Frame,
        geometry_buffers: &GeometryBuffers,
        camera_bind_group: &wgpu::BindGroup,
        texture_storage: &Storage<RenderTexture>,
    ) {
        for object in self.objects.iter() {
            if let Some(model) = self.asset_store.get(object.model) {
                self.model_renderer.render(
                    frame,
                    geometry_buffers,
                    camera_bind_group,
                    model.as_ref(),
                    texture_storage,
                );
            }
        }
        // self.model_renderer.render_multiple(
        //     frame,
        //     geometry_buffers,
        //     camera_bind_group,
        //     BlendMode::Opaque,
        //     &self.opaque_meshes,
        // );

        // self.model_renderer.render_multiple(
        //     frame,
        //     geometry_buffers,
        //     camera_bind_group,
        //     BlendMode::ColorKeyed,
        //     &self.ck_meshes,
        // );
    }

    pub fn render_gizmos(
        &self,
        frame: &mut Frame,
        camera_bind_group: &wgpu::BindGroup,
        gizmos_renderer: &GizmosRenderer,
    ) {
        if self.render_bounding_boxes {
            let mut boxes = vec![];

            for (object_index, object) in self.objects.iter().enumerate() {
                let Some(model) = self.asset_store.get(object.model) else {
                    continue;
                };

                let entity_transform = Mat4::from_rotation_translation(
                    Quat::from_euler(
                        glam::EulerRot::XYZ,
                        object.rotation.x,
                        object.rotation.y,
                        -object.rotation.z,
                    ),
                    object.translation,
                );

                for bounding_box in model.bounding_boxes.iter() {
                    let transform = entity_transform * bounding_box.model_transform;
                    let highlight = if let Some(hover_index) = self.selected_object {
                        hover_index == object_index
                    } else {
                        false
                    };
                    boxes.push(RawBoundingBox::new(
                        transform,
                        bounding_box.min,
                        bounding_box.max,
                        highlight,
                    ));
                }
            }

            self.bounding_box_renderer
                .render_all(frame, camera_bind_group, &boxes);
        }

        // if false {
        //     let mut gv = vec![];

        //     for object in self.objects.iter() {
        //         let Some(model) = self.asset_store.get(object.model) else {
        //             continue;
        //         };

        //         let object_transform = Mat4::from_rotation_translation(
        //             Quat::from_euler(
        //                 glam::EulerRot::XYZ,
        //                 object.rotation.x,
        //                 object.rotation.y,
        //                 -object.rotation.z,
        //             ),
        //             object.translation,
        //         );

        //         for mesh in model.meshes.iter() {
        //             let Some(textured_mesh) = self.asset_store.get(mesh.mesh) else {
        //                 continue;
        //             };

        //             let mesh_transform = object_transform * mesh.model_transform;
        //             let normal_matrix = Mat3::from_mat4(mesh_transform).inverse().transpose();

        //             for vertex in textured_mesh.indexed_mesh.vertices.iter() {
        //                 let position = mesh_transform.project_point3(vertex.position);
        //                 let normal = normal_matrix * vertex.normal;

        //                 gv.push(GizmoVertex::new(position, Vec4::new(0.0, 0.0, 1.0, 1.0)));
        //                 gv.push(GizmoVertex::new(
        //                     position + normal * 10.0,
        //                     Vec4::new(1.0, 0.0, 1.0, 1.0),
        //                 ));
        //             }
        //         }
        //     }

        //     gizmos_renderer.render(frame, camera_bind_group, &gv);
        // }
    }

    pub fn debug_panel(&mut self, ui: &mut egui::Ui) {
        ui.toggle_value(&mut self.render_bounding_boxes, "Render bounding boxes");
    }
}
