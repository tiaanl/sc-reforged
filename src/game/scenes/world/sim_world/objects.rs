use crate::{
    engine::{
        prelude::*,
        storage::{Handle, Storage},
    },
    game::{
        config::ObjectType,
        math::{BoundingSphere, RaySegment},
        model::{Model, ModelRayHit},
        models::{ModelName, models},
        scenes::world::systems::RenderWrapper,
    },
};

pub enum ObjectData {
    Scenery {
        model: Handle<Model>,
    },
    Biped {
        model: Handle<Model>,
    },
    /// Temporary for use with more complicated objects that is not implemented yet.
    SingleModel {
        model: Handle<Model>,
    },
}

pub struct Object {
    pub transform: Transform,
    pub bounding_sphere: BoundingSphere,
    pub data: ObjectData,
}

impl Object {
    pub fn gather_models_to_render(&self, renderer: &mut RenderWrapper, highlight: f32) {
        match self.data {
            ObjectData::Scenery { model } => {
                renderer.render_model(self.transform.to_mat4(), model, highlight)
            }

            ObjectData::Biped { model } => {
                renderer.render_model(self.transform.to_mat4(), model, highlight);
            }

            ObjectData::SingleModel { model } => {
                renderer.render_model(self.transform.to_mat4(), model, highlight)
            }
        }
    }

    /// Intersect this object with a world-space ray segment using the model's collision boxes.
    /// Returns Some((t, world_position)) for the closest hit, or None if no hit.
    pub fn ray_intersection(&self, ray_segment: &RaySegment) -> Option<ModelRayHit> {
        // Quad tree already applied coarse culling; do only fine model test here.
        let object_to_world = self.transform.to_mat4();

        let model_handle = match &self.data {
            ObjectData::Scenery { model }
            | ObjectData::Biped { model }
            | ObjectData::SingleModel { model } => *model,
        };

        let model = models().get(model_handle)?;
        model.intersect_ray_segment_with_transform(object_to_world, ray_segment)
    }
}

pub struct Objects {
    /// A list for all objects iun the world.
    pub objects: Storage<Object>,

    /// Keep a list of handles to try and load.
    models_to_prepare: Vec<Handle<Model>>,
}

impl Objects {
    pub fn new() -> Result<Self, AssetError> {
        Ok(Self {
            objects: Storage::default(),
            models_to_prepare: Vec::default(),
        })
    }

    pub fn spawn(
        &mut self,
        transform: Transform,
        object_type: ObjectType,
        name: &str,
        _title: &str,
    ) -> Result<(Handle<Object>, &Object), AssetError> {
        let mut bounding_sphere = BoundingSphere::ZERO;

        let object_data = match object_type {
            _ => {
                let (model_handle, model) = models().load_model(ModelName::Object(name.into()))?;

                bounding_sphere.expand_to_include(&model.bounding_sphere);

                self.models_to_prepare.push(model_handle);

                ObjectData::SingleModel {
                    model: model_handle,
                }
            }
        };

        // Move to bounding sphere into position.
        bounding_sphere.center += transform.translation;

        let handle = self.objects.insert(Object {
            transform,
            bounding_sphere,
            data: object_data,
        });

        Ok((handle, self.objects.get(handle).unwrap()))
    }

    #[inline]
    pub fn get(&self, handle: Handle<Object>) -> Option<&Object> {
        self.objects.get(handle)
    }
}
