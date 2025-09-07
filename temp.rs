use bevy_ecs::{prelude as ecs, system::ResMut};
use glam::Vec3;

use crate::{
    engine::{prelude::Transform, storage::Handle},
    game::{
        animations::{Sequencer, sequences},
        config::ObjectType,
        model::Model,
        renderer::RenderInstance,
        scenes::world::{
            actions::PlayerAction,
            object::BipedalOrder,
            resources::{DeltaTime, HeightMapResource, ModelRendererResource, SelectedEntity},
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
        match *order {
            BipedalOrder::Stand => {
                // if let Some(sequence) = sequences().get_by_name("MSEQ_STAND") {
                //     sequencer.play_sequence(sequence);
                // } else {
                //     tracing::warn!("Sequence not found! (MSEQ_STAND)");
                // }
            }
            BipedalOrder::MoveTo { .. } => {}
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
            println!("updating render instance");
            updater.set_transform(transform.to_mat4());
            if let Some((render_animation, time)) = animation {
                updater.set_animation(render_animation, time);
            }
        });
    }
}

pub fn handle_player_actions(
    mut player_actions: ecs::EventReader<PlayerAction>,
    mut bipeds: ecs::Query<(&ObjectType, &mut BipedalOrder)>,
    mut selected_entity: ResMut<SelectedEntity>,
) {
    // Runs every frame even if there were no events.

    for player_action in player_actions.read() {
        match player_action {
            PlayerAction::ClearSelection => {
                // Deselect the selected entity, if any.
                selected_entity.0 = None;
            }
            PlayerAction::ObjectClicked { entity, .. } => {
                tracing::info!("clicked entity: {entity}");

                // Set a new selected entity.
                selected_entity.0 = Some(*entity);
            }
            PlayerAction::TerrainClicked { _position } => {
                // If the player clicked on the terrain and has a biped selected, issue an order
                // for the biped to move there.
                if let Some(selected_entity) = selected_entity.0 {
                    let (object_type, mut order) = bipeds.get_mut(selected_entity).unwrap();
                    match object_type {
                        ObjectType::Ape | ObjectType::Bipedal => {
                            *order = BipedalOrder::MoveTo {
                                target_location: _position.truncate(),
                                speed: 100.0,
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

pub fn handle_orders(
    delta_time: ecs::Res<DeltaTime>,
    height_map: ecs::Res<HeightMapResource>,
    mut query: ecs::Query<(ecs::Entity, &mut Transform, &mut BipedalOrder)>,
) {
    for (_entity, mut transform, mut order) in query.iter_mut() {
        match *order {
            BipedalOrder::MoveTo {
                target_location,
                speed,
            } => {
                let current_xy = transform.translation.truncate();
                let (current_pos, current_normal) =
                    height_map.0.world_position_and_normal(current_xy);

                // Snap to the ground.
                transform.translation.z = current_pos.z;

                // Create a vector to the target.
                let to_target_xy = target_location - current_xy;
                let distance_to_target = to_target_xy.length();

                // Arrived already?
                if distance_to_target <= speed {
                    let (target_pos, target_normal) =
                        height_map.0.world_position_and_normal(target_location);

                    transform.translation = target_pos;

                    // Keep facing to the previous forward, but align up to the ground.
                    let previous_forward = transform.rotation.mul_vec3(Vec3::Y);
                    let forward =
                        project_onto_plane(previous_forward, target_normal).normalize_or_zero();
                    transform.rotation = quat_from_lfu(forward, target_normal);

                    // Issue a *stand* order.
                    *order = BipedalOrder::Stand;

                    continue;
                }

                // Desired planar direction, then slide along the terrain tangent.
                let desired_dir_world = Vec3::new(to_target_xy.x, to_target_xy.y, 0.0).normalize();
                let tangent_dir =
                    project_onto_plane(desired_dir_world, current_normal).normalize_or_zero();

                // Constant-speed step, clamped to avoid overshooting.
                let step_xy = (speed * delta_time.0).min(distance_to_target);
                let new_xy = current_xy + tangent_dir.truncate() * step_xy;

                // Stick to the ground at the new (x, y) position.
                let (new_pos, new_normal) = height_map.0.world_position_and_normal(new_xy);

                transform.translation = new_pos;

                // Face motion and align up with ground.
                let forward = project_onto_plane(tangent_dir, new_normal).normalize_or_zero();
                transform.rotation = quat_from_lfu(forward, new_normal);
            }
            _ => {}
        }
    }
}

fn project_onto_plane(v: Vec3, n: Vec3) -> Vec3 {
    v - n * v.dot(n)
}

/// Build a quaternion from your basis: Left (+X), Forward (+Y), Up (+Z).
/// In your LH system:
///   - forward is provided (tangent along ground)
///   - up is ground normal
///   - left must be computed so that columns are (left, forward, up)
///
/// For a LH basis with math using RH cross products, we can get "left" with up × forward.
fn quat_from_lfu(forward: Vec3, up: Vec3) -> glam::Quat {
    let forward = forward.normalize_or_zero();
    let up = up.normalize_or_zero();

    // Left = up × forward  (yields +X = left, consistent with your handedness)
    let left = up.cross(forward).normalize_or_zero();

    // Re-orthogonalize forward in case of drift
    let forward = project_onto_plane(forward, up).normalize_or_zero();

    let basis = glam::Mat3::from_cols(left, forward, up);
    glam::Quat::from_mat3(&basis)
}
