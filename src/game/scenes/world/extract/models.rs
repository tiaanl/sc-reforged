use bevy_ecs::prelude::*;

use crate::{
    engine::storage::Handle,
    game::scenes::world::{
        render::{ModelRenderFlags, ModelRenderSnapshot, ModelToRender},
        sim_world::{ComputedCamera, Object, Objects, SimWorld, ecs::ActiveCamera},
    },
};

pub struct ModelsExtract {
    visible_objects_cache: Vec<Handle<Object>>,

    /// ECS query for the active camera.
    active_camera_query: QueryState<&'static ComputedCamera, With<ActiveCamera>>,
}

impl ModelsExtract {
    pub fn new(sim_world: &mut SimWorld) -> Self {
        Self {
            visible_objects_cache: Vec::default(),
            active_camera_query: sim_world.ecs.query_filtered(),
        }
    }

    pub fn extract(&mut self, sim_world: &SimWorld, snapshot: &mut ModelRenderSnapshot) {
        snapshot.models.clear();

        let computed_camera = self.active_camera_query.single(&sim_world.ecs).unwrap();

        let objects = sim_world.ecs.resource::<Objects>();
        objects
            .static_bvh
            .objects_in_frustum(&computed_camera.frustum, &mut self.visible_objects_cache);

        let state = sim_world.state();
        let selected_objects = &state.selected_objects;

        self.visible_objects_cache
            .iter()
            .filter_map(|object_handle| objects.get(*object_handle).map(|o| (o, *object_handle)))
            .for_each(|(object, handle)| {
                let mut flags = ModelRenderFlags::empty();
                flags.set(
                    ModelRenderFlags::HIGHLIGHTED,
                    selected_objects.contains(&handle),
                );

                use crate::game::scenes::world::sim_world::ObjectData;

                let model = match &object.data {
                    ObjectData::Scenery { model }
                    | ObjectData::Biped { model, .. }
                    | ObjectData::SingleModel { model } => *model,
                };

                snapshot.models.push(ModelToRender {
                    model,
                    transform: object.transform.to_mat4(),
                    flags,
                });
            });
    }
}
