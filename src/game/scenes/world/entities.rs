use crate::{
    engine::{gizmos::GizmosRenderer, prelude::*},
    game::{
        animation::AnimationSet,
        camera::Ray,
        mesh_renderer::{MeshItem, MeshList, MeshRenderer},
        model::Model,
    },
};

use super::bounding_boxes::{BoundingBox, BoundingBoxRenderer};

/// Represents an object inside the game world.
#[derive(Debug)]
pub struct Entity {
    pub translation: Vec3,
    pub rotation: Vec3,
    pub model: Handle<Model>,
    pub animation_set: AnimationSet,
}

impl Entity {
    pub fn new(translation: Vec3, rotation: Vec3, model: Handle<Model>) -> Self {
        Self {
            translation,
            rotation,
            model,
            animation_set: AnimationSet::default(),
        }
    }
}

pub struct Entities {
    asset_manager: AssetManager,

    pub model_renderer: MeshRenderer,

    render_bounding_boxes: bool,
    pub bounding_box_renderer: BoundingBoxRenderer,

    pub entities: Vec<Entity>,
    /// The entity index that the mouse is currently over.
    selected_entity: Option<usize>,
}

impl Entities {
    pub fn new(
        asset_manager: AssetManager,
        renderer: &Renderer,
        shaders: &mut Shaders,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let model_renderer = MeshRenderer::new(
            asset_manager.clone(),
            renderer,
            shaders,
            camera_bind_group_layout,
        );

        let bounding_box_renderer = BoundingBoxRenderer::new(renderer, camera_bind_group_layout);

        Self {
            asset_manager,
            model_renderer,
            render_bounding_boxes: false,
            bounding_box_renderer,
            entities: vec![],
            selected_entity: None,
        }
    }

    pub fn spawn(&mut self, entity: Entity) {
        self.entities.push(entity);
    }

    pub fn get(&self, entity_index: usize) -> Option<&Entity> {
        self.entities.get(entity_index)
    }

    pub fn get_mut(&mut self, entity_index: usize) -> Option<&mut Entity> {
        self.entities.get_mut(entity_index)
    }

    pub fn ray_intersection(&self, ray: &Ray) -> Option<usize> {
        let mut closest = f32::MAX;
        let mut closest_entity = None;

        // Gather up a list of bounding boxes for each entity/model.
        for (entity_index, entity) in self.entities.iter().enumerate() {
            let Some(model) = self.asset_manager.get(entity.model) else {
                continue;
            };

            let entity_transform = Mat4::from_rotation_translation(
                Quat::from_euler(
                    glam::EulerRot::XYZ,
                    entity.rotation.x,
                    entity.rotation.y,
                    entity.rotation.z,
                ),
                entity.translation,
            );

            for bounding_box in model.bounding_boxes.iter() {
                let transform = entity_transform * model.global_transform(bounding_box.node_id);
                let bbox = BoundingBox::new(transform, bounding_box.min, bounding_box.max, false);
                if let Some(distance) = bbox.intersect_ray(ray) {
                    if distance < closest {
                        closest = distance;
                        closest_entity = Some(entity_index);
                    }
                }
            }
        }

        closest_entity
    }

    pub fn set_selected(&mut self, selected: Option<usize>) {
        self.selected_entity = selected;
    }

    pub fn render_frame(
        &self,
        frame: &mut Frame,
        camera_bind_group: &wgpu::BindGroup,
        gizmos: &GizmosRenderer,
    ) {
        // Build a list of all the meshes that needs rendering.
        let mut meshes = MeshList::default();
        let mut boxes = vec![];
        let mut gv = vec![];

        for (entity_index, entity) in self.entities.iter().enumerate() {
            let Some(model) = self.asset_manager.get(entity.model) else {
                continue;
            };

            let entity_transform = Mat4::from_rotation_translation(
                Quat::from_euler(
                    glam::EulerRot::XYZ,
                    entity.rotation.x,
                    entity.rotation.y,
                    entity.rotation.z,
                ),
                entity.translation,
            );

            for mesh in model.meshes.iter() {
                let mut transform = entity_transform * model.global_transform(mesh.node_id);
                if let Some(animation_transform) = entity.animation_set.set.get(&mesh.node_id) {
                    transform = transform * animation_transform.to_mat4();
                }

                gv.append(&mut GizmosRenderer::create_axis(&transform, 100.0));

                meshes.meshes.push(MeshItem {
                    transform,
                    mesh: mesh.mesh,
                });
            }

            if self.render_bounding_boxes {
                for bounding_box in model.bounding_boxes.iter() {
                    let transform = entity_transform * model.global_transform(bounding_box.node_id);
                    let highlight = if let Some(hover_index) = self.selected_entity {
                        hover_index == entity_index
                    } else {
                        false
                    };
                    boxes.push(BoundingBox::new(
                        transform,
                        bounding_box.min,
                        bounding_box.max,
                        highlight,
                    ));
                }
            }
        }

        self.model_renderer
            .render_multiple(frame, camera_bind_group, meshes);

        if self.render_bounding_boxes {
            self.bounding_box_renderer
                .render_all(frame, camera_bind_group, &boxes);
        }

        gizmos.render_frame(frame, camera_bind_group, &gv);
    }

    pub fn debug_panel(&mut self, ui: &mut egui::Ui) {
        ui.toggle_value(&mut self.render_bounding_boxes, "Render bounding boxes");
    }
}
