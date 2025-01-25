use std::cmp::Ordering;

use glam::Vec4Swizzles;

use crate::{
    engine::prelude::*,
    game::{
        camera::{BoundingBox, Camera, Frustum, Ray},
        mesh_renderer::{BlendMode, MeshItem, MeshRenderer},
        model::Model,
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

    pub model_renderer: MeshRenderer,

    /// Keep a local list of meshes to render each frame.
    opaque_meshes: Vec<MeshItem>,
    ck_meshes: Vec<MeshItem>,
    alpha_meshes: Vec<MeshItem>,

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
        environment_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let model_renderer = MeshRenderer::new(
            asset_store.clone(),
            renderer,
            shaders,
            camera_bind_group_layout,
            environment_bind_group_layout,
        );

        let bounding_box_renderer = BoundingBoxRenderer::new(renderer, camera_bind_group_layout);

        Self {
            asset_store,
            model_renderer,
            opaque_meshes: Vec::default(),
            ck_meshes: Vec::default(),
            alpha_meshes: Vec::default(),
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
                    -object.rotation.z,
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
                        -object.rotation.z,
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

        self.opaque_meshes.clear();
        self.ck_meshes.clear();
        self.alpha_meshes.clear();

        // Update the local mesh list with visible objects only.
        for object in self.objects.iter().filter(|o| o.visible) {
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

            for mesh in model.meshes.iter() {
                let transform = entity_transform * mesh.model_transform;

                // We can use the squared distance here, because we only use it for sorting.
                let mesh_position = transform.col(3);
                let distance_from_camera = camera.position.distance_squared(mesh_position.xyz());

                match mesh.blend_mode {
                    BlendMode::None => self.opaque_meshes.push(MeshItem {
                        transform,
                        mesh: mesh.mesh,
                        distance_from_camera,
                    }),
                    BlendMode::ColorKeyed => self.ck_meshes.push(MeshItem {
                        transform,
                        mesh: mesh.mesh,
                        distance_from_camera,
                    }),
                    BlendMode::Alpha => self.alpha_meshes.push(MeshItem {
                        transform,
                        mesh: mesh.mesh,
                        distance_from_camera,
                    }),
                    BlendMode::Multiply => todo!("not implemented yet"),
                };
            }
        }

        // Sort opaque meshes near to far to take advantage of the depth buffer to discard pixels.
        self.opaque_meshes.sort_unstable_by(|a, b| {
            a.distance_from_camera
                .partial_cmp(&b.distance_from_camera)
                .unwrap_or(Ordering::Equal)
        });

        // Sort color keyed meshes as if they were opaque meshes.
        self.ck_meshes.sort_unstable_by(|a, b| {
            a.distance_from_camera
                .partial_cmp(&b.distance_from_camera)
                .unwrap_or(Ordering::Equal)
        });

        // Sort alpha meshes far to near to avoid overlap in rendering.
        self.alpha_meshes.sort_unstable_by(|a, b| {
            b.distance_from_camera
                .partial_cmp(&a.distance_from_camera)
                .unwrap_or(Ordering::Equal)
        });
    }

    pub fn render_objects(
        &self,
        frame: &mut Frame,
        camera_bind_group: &wgpu::BindGroup,
        environment_bind_group: &wgpu::BindGroup,
    ) {
        self.model_renderer.render_multiple(
            frame,
            camera_bind_group,
            environment_bind_group,
            BlendMode::None,
            &self.opaque_meshes,
        );

        self.model_renderer.render_multiple(
            frame,
            camera_bind_group,
            environment_bind_group,
            BlendMode::ColorKeyed,
            &self.ck_meshes,
        );
    }

    pub fn render_alpha_objects(
        &self,
        frame: &mut Frame,
        camera_bind_group: &wgpu::BindGroup,
        environment_bind_group: &wgpu::BindGroup,
    ) {
        self.model_renderer.render_multiple(
            frame,
            camera_bind_group,
            environment_bind_group,
            BlendMode::Alpha,
            &self.alpha_meshes,
        );
    }

    pub fn render_gizmos(&self, frame: &mut Frame, camera_bind_group: &wgpu::BindGroup) {
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
    }

    pub fn debug_panel(&mut self, ui: &mut egui::Ui) {
        ui.toggle_value(&mut self.render_bounding_boxes, "Render bounding boxes");
    }
}
