use bevy_ecs::{hierarchy::Children, prelude as ecs, system::ResMut};

use crate::{
    engine::{prelude::Transform, storage::Handle},
    game::{
        animations::{Sequencer, sequences},
        model::Model,
        renderer::RenderInstance,
        scenes::world::{
            actions::PlayerAction,
            object::BipedalOrder,
            resources::{ModelRendererResource, SelectedEntity},
        },
    },
};

type ModelsQuery<'world, 'state, 'a> = ecs::Query<
    'world,
    'state,
    (&'a Transform, &'a Handle<Model>, ecs::Entity),
    ecs::Added<Handle<Model>>,
>;

/// Take entities with [Handle<Model>] and create [Handle<RenderInstance>]s for them.
pub fn create_render_instances(
    mut commands: ecs::Commands,
    models: ModelsQuery,
    mut model_renderer: ResMut<ModelRendererResource>,
) {
    let model_renderer = &mut model_renderer.0;
    for (transform, model, entity) in models.iter() {
        let Ok(render_instance) =
            model_renderer.add_render_instance(*model, transform.to_mat4(), entity.index())
        else {
            tracing::warn!("Could not create render instance for model.");
            continue;
        };

        commands.entity(entity).insert(render_instance);
    }
}

pub fn handle_new_orders(
    mut orders: ecs::Query<(&BipedalOrder, &mut Sequencer), ecs::Changed<BipedalOrder>>,
) {
    for (order, mut sequencer) in orders.iter_mut() {
        tracing::info!("Issuing order");
        // The bipedal's orders have changed, update the sequencer.
        match order {
            BipedalOrder::Stand => {
                if let Some(sequence) = sequences().get_by_name("MSEQ_STAND") {
                    sequencer.play_sequence(sequence);
                } else {
                    tracing::warn!("Sequence not found! (MSEQ_STAND)");
                }
            }
            BipedalOrder::MoveTo { .. } => todo!(),
        }
    }
}

type ParentsQuery<'world, 'state, 'a> = ecs::Query<
    'world,
    'state,
    (&'a Transform, &'a Children),
    (ecs::Changed<Transform>, ecs::Without<ecs::ChildOf>),
>;

pub fn update_child_transforms(
    parents: ParentsQuery,
    mut transforms: ecs::Query<&mut Transform, ecs::With<ecs::ChildOf>>,
) {
    for (transform, children) in parents.iter() {
        for child in children.iter() {
            let mut child_transform = transforms.get_mut(*child).unwrap();
            *child_transform = transform.clone();
        }
    }
}

type RenderInstancesQuery<'world, 'state, 'a> = ecs::Query<
    'world,
    'state,
    (
        &'a Transform,
        &'a Handle<Model>,
        &'a Handle<RenderInstance>,
        Option<&'a Sequencer>,
    ),
    ecs::Changed<Transform>,
>;

pub fn update_render_instances(
    render_instances: RenderInstancesQuery,
    mut model_renderer: ResMut<ModelRendererResource>,
) {
    let model_renderer = &mut model_renderer.0;

    for (transform, model, render_instance, sequencer) in render_instances.iter() {
        // Try to extract animation data from the entity.
        let animation = sequencer
            .and_then(|sequencer| sequencer.get_animation_state())
            .map(|animation_state| {
                (
                    model_renderer.get_or_insert_animation(*model, animation_state.animation),
                    animation_state.time,
                )
            });

        model_renderer.update_instance(*render_instance, |updater| {
            updater.set_transform(transform.to_mat4());
            if let Some((render_animation, time)) = animation {
                updater.set_animation(render_animation, time);
            }
        });
    }
}

pub fn handle_player_actions(
    mut player_actions: ecs::EventReader<PlayerAction>,
    mut selected_entity: ResMut<SelectedEntity>,
) {
    // Runs every frame even if there were no events.

    for player_action in player_actions.read() {
        match player_action {
            PlayerAction::ClearSelection => {
                // Deselect the selected entity, if any.
                selected_entity.0 = None;
            }
            PlayerAction::ObjectClicked { id, .. } => {
                tracing::info!("clicked entity: {id}");

                // Set a new selected entity.
                selected_entity.0 = Some(ecs::Entity::from_raw(*id));
            }
            PlayerAction::TerrainClicked { .. } => {}
        }
    }
}
