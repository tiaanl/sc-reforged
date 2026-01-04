use bevy_ecs::prelude::*;

use crate::{
    engine::{storage::Handle, transform::Transform},
    game::{
        model::Model,
        scenes::world::{
            render::{ModelRenderFlags, ModelRenderSnapshot, ModelToRender},
            sim_world::{ComputedCamera, Objects, SimWorld, ecs::ActiveCamera},
        },
    },
};

pub struct ModelsExtract {
    visible_objects_cache: Vec<Entity>,

    /// ECS query for the active camera.
    active_camera_query: QueryState<&'static ComputedCamera, With<ActiveCamera>>,

    /// ECS query for all renderable models.
    models_query: QueryState<(Entity, &'static Transform, &'static Handle<Model>)>,
}

impl ModelsExtract {
    pub fn new(sim_world: &mut SimWorld) -> Self {
        Self {
            visible_objects_cache: Vec::default(),
            active_camera_query: sim_world.ecs.query_filtered(),
            models_query: sim_world.ecs.query(),
        }
    }

    pub fn extract(&mut self, sim_world: &SimWorld, snapshot: &mut ModelRenderSnapshot) {
        snapshot.models.clear();

        let computed_camera = self.active_camera_query.single(&sim_world.ecs).unwrap();
        let static_bvh = &sim_world.ecs.resource::<Objects>().static_bvh;
        let selected_objects = &sim_world.state().selected_objects;

        static_bvh.objects_in_frustum(&computed_camera.frustum, &mut self.visible_objects_cache);

        for (entity, transform, model_handle) in self
            .models_query
            .iter_many(&sim_world.ecs, &self.visible_objects_cache)
        {
            let mut flags = ModelRenderFlags::empty();

            flags.set(
                ModelRenderFlags::HIGHLIGHTED,
                selected_objects.contains(&entity),
            );

            snapshot.models.push(ModelToRender {
                model: *model_handle,
                transform: transform.to_mat4(),
                flags,
            });
        }
    }
}
