use crate::{
    engine::{
        prelude::*,
        storage::{Handle, Storage},
    },
    game::{
        config::ObjectType, math::BoundingSphere, model::Model, models::models,
        scenes::world::render::RenderStore,
    },
};

enum ObjectData {
    Scenery {
        model: Handle<Model>,
        _model_des: Handle<Model>,
    },
    /// Temporary for use with more complicated objects that is not implemented yet.
    SingleModel { model: Handle<Model> },
}

pub struct Object {
    pub transform: Transform,
    pub bounding_sphere: BoundingSphere,
    data: ObjectData,
}

impl Object {
    pub fn model_to_render(&self) -> Option<Handle<Model>> {
        Some(match self.data {
            ObjectData::Scenery { model, .. } => model,
            ObjectData::SingleModel { model } => model,
        })
    }
}

#[derive(Default)]
pub struct NewObjects {
    /// A list for all objects iun the world.
    pub objects: Storage<Object>,

    /// Keep a list of handles to try and load.
    models_to_prepare: Vec<Handle<Model>>,
}

impl NewObjects {
    pub fn spawn(
        &mut self,
        transform: Transform,
        object_type: ObjectType,
        name: &str,
        _title: &str,
    ) -> Result<(Handle<Object>, &Object), AssetError> {
        let mut bounding_sphere = BoundingSphere::ZERO;

        let object_data = match object_type {
            ObjectType::Scenery
            | ObjectType::SceneryAlarm
            | ObjectType::SceneryBush
            | ObjectType::SceneryFragile
            | ObjectType::SceneryLit
            | ObjectType::SceneryShadowed
            | ObjectType::SceneryStripLight
            | ObjectType::SceneryTree
            | ObjectType::Structure
            | ObjectType::StructureArmGate
            | ObjectType::StructureBridge
            | ObjectType::StructureBuggable
            | ObjectType::StructureBuilding
            | ObjectType::StructureBuildingGateController
            | ObjectType::StructureDoubleGate
            | ObjectType::StructureFence
            | ObjectType::StructureGuardTower
            | ObjectType::StructureLadderSlant0_11
            | ObjectType::StructureLadderSlant0_14
            | ObjectType::StructureLadderSlant0_16
            | ObjectType::StructureLadderSlant0_2
            | ObjectType::StructureLadderSlant0_3
            | ObjectType::StructureLadderSlant0_5
            | ObjectType::StructureLadderSlant0_6
            | ObjectType::StructureLadderSlant0_9
            | ObjectType::StructureLadderSlant2_2
            | ObjectType::StructureLadderSlant2_4
            | ObjectType::StructureLadderSlant2_5
            | ObjectType::StructureLocker
            | ObjectType::StructureSAM
            | ObjectType::StructureSingleGate
            | ObjectType::StructureSlideBridge
            | ObjectType::StructureSlideBridgeController
            | ObjectType::StructureSlideDoor
            | ObjectType::StructureStripLightController
            | ObjectType::StructureSwingDoor
            | ObjectType::StructureTent
            | ObjectType::StructureWall => {
                let (model_handle, model) = models().load_object_model(name)?;

                bounding_sphere.expand_to_include(&model.bounding_sphere);

                let (model_des_handle, model_des) = models().load_object_model(name)?;

                bounding_sphere.expand_to_include(&model_des.bounding_sphere);

                self.models_to_prepare.push(model_handle);
                self.models_to_prepare.push(model_des_handle);

                ObjectData::Scenery {
                    model: model_handle,
                    _model_des: model_des_handle,
                }
            }
            _ => {
                let (model_handle, model) = models().load_object_model(name)?;

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

    /// Take all models that were used during `spawn` and prepare them to be rendered.
    pub fn prepare_models(&mut self, render_store: &mut RenderStore) {
        if self.models_to_prepare.is_empty() {
            return;
        }

        let mut models_to_prepare = Vec::default();
        std::mem::swap(&mut self.models_to_prepare, &mut models_to_prepare);

        tracing::info!("Preparing {} models for the GPU.", models_to_prepare.len());

        for model_handle in models_to_prepare {
            if let Err(err) = render_store.get_or_create_render_model(model_handle) {
                tracing::warn!("Could not prepare model! ({err})");
            }
        }
    }
}
