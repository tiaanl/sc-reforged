use bevy_ecs::prelude::*;
use glam::{Quat, Vec3};

use crate::{
    engine::{egui_integration::UiExt, transform::Transform},
    game::{
        AssetReader,
        // scenes::world::sim_world::{SequenceName, Sequencer, SequencerRequest},
    },
};

/// A message sent when a new order is issues to an Entity.
#[derive(Message)]
pub enum OrderMessage {
    /// Issue a new order.
    New { entity: Entity, order: Order },
}

// 1  -> order_move
// 3  -> order_move_to_use_vehicle
// 4  -> order_move_to_attack
// 5  -> order_move_to_cut_fence
// 6  -> order_move_to_crawl_through_fence
// 7  -> order_move_to_climb_wall
// 8  -> order_move_to_pick_up
// 9  -> order_move_to_use_structure
// 10 -> order_move_to_drop_item
// 11 -> order_move_to_activate_structure
// 12 -> order_move_to_tranfer_item
// 13 -> order_move_to_transger_item_weapons_locker
// 14 -> order_move_to_transfer_item_from_body
// 15 -> order_move_to_investigate
// 16 -> order_move_to_use_ladder
// 17 -> order_move_to_investigate_body
// 18 -> order_move_to_use_cover
// 19 -> order_move_to_avoid_vehicle
// 20 -> order_track
// 21 -> order_track_to_attack
// 22 -> order_force_attack
// 23 -> order_equip_self
// 24 -> order_unequip_self
// 25 -> order_move_to_place_item
// 26 -> order_defensive_attack
// 27 -> order_info
// 28 -> order_change_state
// 29 -> order_exit_structure
// 30 -> order_use_special_item
// 31 -> order_track (base; lightweight)
// 32 -> order_track_to_attack (alt entry)
// 33 -> order_move_to_place_item (alt entry)
// 34 -> order_move_to_pick_up (alt entry)
// 35 -> order_move_to_transfer_item (alt entry)
// 36 -> order_move_to_transfer_item_weapons_locker (alt entry)
// 37 -> order_move_to_transfer_item_from_body (alt entry)

#[derive(Component, Clone, Debug, Default, PartialEq)]
pub enum Order {
    #[default]
    Idle,
    _Move(OrderMove),
    ChangeState(OrderChangeState),
    // MoveToUseVehicle,
    // MoveToAttack,
    // MoveToCutFence,
    // MoveToCrawlThroughFence,
    // MoveToClimbWall,
    // MoveToPickUp,
    // MoveToUseStructure,
    // MoveToDropItem,
    // MoveToActivateStructure,
    // MoveToTransferItem,
    // MoveToTransferItemWeaponsLocker,
    // MoveToTransferItemFromBody,
    // MoveToInvestigate,
    // MoveToUseLadder,
    // MoveToInvestigateBody,
    // MoveToUseCover,
    // MoveToAvoidVehicle,
    // Track,
    // TrackToAttack,
    // ForceAttack,
    // EquipSelf,
    // UnequipSelf,
    // MoveToPlaceItem,
    // DefensiveAttack,
    // Info,
    // ChangeState,
    // ExitStructure,
    // UseSpecialItem,
}

impl Order {
    pub fn _ui(&self, ui: &mut egui::Ui) {
        match self {
            Order::Idle => {
                ui.vertical(|ui| {
                    ui.h3("Idle");
                });
            }

            Order::_Move(OrderMove {
                target_location: world_position,
                move_speed,
                rotation_speed,
                ..
            }) => {
                ui.vertical(|ui| {
                    ui.h3("Move");
                    ui.horizontal(|ui| {
                        ui.label(format!("{:.0}", world_position.x));
                        ui.label(format!("{:.0}", world_position.y));
                        ui.label(format!("{:.0}", world_position.z));
                    });
                    ui.label(format!("Move speed: {move_speed}"));
                    ui.label(format!("Rotate speed: {rotation_speed}"));
                });
            }
            Order::ChangeState(order) => {
                ui.vertical(|ui| {
                    ui.h3("ChangeState");
                    ui.label(format!("Target: {:?}", order.target));
                    ui.label(format!(
                        "Requested sequence: {}",
                        order.has_requested_sequence
                    ));
                });
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeStateTarget {
    Stand,
    Crouch,
    Prone,
}

impl ChangeStateTarget {
    /// Resolve the motion-sequencer sequence used to reach this target posture.
    #[inline]
    pub const fn sequence_name(self) -> SequenceName {
        match self {
            Self::Stand => SequenceName::Stand,
            Self::Crouch => SequenceName::Crouch,
            Self::Prone => SequenceName::Prone,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrderChangeState {
    pub target: ChangeStateTarget,

    /// Internal guard to emit the sequence request only once.
    pub has_requested_sequence: bool,
}

impl OrderChangeState {
    /// Construct a posture-change order targeting `target`.
    #[inline]
    pub const fn new(target: ChangeStateTarget) -> Self {
        Self {
            target,
            has_requested_sequence: false,
        }
    }

    /// Update this posture change order and report completion.
    pub fn update(&mut self, sequencer: &mut Sequencer) -> bool {
        let target_sequence = self.target.sequence_name();

        if !self.has_requested_sequence {
            // Match original behavior: clear motion-controller queue before
            // issuing a posture sequence request.
            sequencer.reset();
            sequencer.request(
                SequencerRequest::new(target_sequence)
                    .with_speed(0.8)
                    .with_clear_on_change(true),
            );
            self.has_requested_sequence = true;
            return false;
        }

        sequencer.current_sequence_name() == Some(target_sequence)
            && !sequencer.has_pending_request()
            && !sequencer.is_transition_active()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrderMove {
    pub target_location: Vec3,
    pub move_speed: f32,
    pub rotation_speed: f32,

    /// Whether the entity finished turning towards the target and is moving.
    pub is_moving: bool,

    /// Tracks whether forward locomotion was already requested at least once.
    pub has_requested_forward_locomotion: bool,
}

impl OrderMove {
    /// Rotate the object toward `direction` by at most `rotation_speed * delta_time`.
    fn rotate_towards_direction(
        transform: &mut Transform,
        direction: Vec3,
        rotation_speed: f32,
        delta_time: f32,
    ) {
        let current_forward = transform.rotation * Vec3::Y;
        let delta_rot = Quat::from_rotation_arc(current_forward, direction);
        let (axis, angle) = delta_rot.to_axis_angle();
        let max_angle = rotation_speed * delta_time;
        if angle > 1.0e-6 {
            let clamped = angle.min(max_angle);
            let step = Quat::from_axis_angle(axis, clamped);
            transform.rotation = step * transform.rotation;
        }
    }

    /// Request forward locomotion sequence data in the same style as the
    /// original accelerate-forward path: request every tick, use an "into"
    /// sequence on first request, then request steady walk with a different
    /// speed scalar after locomotion has started.
    fn request_forward_locomotion(
        &mut self,
        remaining_distance: f32,
        sequencer: &mut Sequencer,
    ) -> f32 {
        const FAST_DISTANCE_THRESHOLD: f32 = 100.0;

        let sequence_name = if self.has_requested_forward_locomotion {
            SequenceName::Walk
        } else {
            SequenceName::IntoWalk
        };

        let mut sequence_speed = if self.has_requested_forward_locomotion {
            0.8
        } else {
            0.6
        };

        if remaining_distance > FAST_DISTANCE_THRESHOLD {
            sequence_speed = 1.2;
        }

        sequencer.request(
            SequencerRequest::new(sequence_name)
                .with_speed(sequence_speed)
                .with_clear_on_change(false),
        );

        self.has_requested_forward_locomotion = true;
        sequence_speed
    }

    /// Update this move order for one frame.
    pub fn update(
        &mut self,
        transform: &mut Transform,
        delta_time: f32,
        sequencer: &mut Sequencer,
        assets: &AssetReader,
    ) -> bool {
        const ARRIVAL_DISTANCE: f32 = 4.0;
        // Turn to within this angle before moving.
        const ALIGN_ANGLE_RAD: f32 = 0.035; // ~2 degrees

        // Posture/locomotion deltas should move along the ground plane only.
        let to_target = self.target_location - transform.translation;
        let to_target_planar = Vec3::new(to_target.x, to_target.y, 0.0);
        let distance_sq = to_target_planar.length_squared();
        if distance_sq <= 1.0e-6 {
            return true;
        }

        let direction = to_target_planar * distance_sq.sqrt().recip();

        // Step 1: Turn towards the target.
        if !self.is_moving {
            Self::rotate_towards_direction(transform, direction, self.rotation_speed, delta_time);

            let aligned_dot = (transform.rotation * Vec3::Y).dot(direction);
            let aligned_threshold = ALIGN_ANGLE_RAD.cos();
            if aligned_dot < aligned_threshold {
                return false;
            }

            self.is_moving = true;
        }

        // Step 2: Move towards the target.

        let remaining_distance = distance_sq.sqrt();
        if remaining_distance > ARRIVAL_DISTANCE {
            // Keep steering while locomoting so animation-root movement tracks
            // the order target instead of continuing on stale heading.
            Self::rotate_towards_direction(transform, direction, self.rotation_speed, delta_time);

            let speed_scalar = self.request_forward_locomotion(remaining_distance, sequencer);
            let move_delta = sequencer
                .current_root_motion_delta(assets, delta_time)
                .map(|local_delta| {
                    let mut world_delta = transform.rotation * local_delta;
                    world_delta.z = 0.0;
                    world_delta
                })
                .filter(|delta| delta.length_squared() > f32::EPSILON)
                .unwrap_or(direction * (self.move_speed * speed_scalar * delta_time));

            // If root motion points away from the goal, preserve magnitude but
            // redirect toward the target to avoid endless walk-away behavior.
            let move_delta = if move_delta.dot(direction) <= 0.0 {
                direction * move_delta.length()
            } else {
                move_delta
            };

            let move_distance = move_delta.length();
            if move_distance > f32::EPSILON {
                let clamped_delta = if move_distance > remaining_distance {
                    move_delta * (remaining_distance / move_distance)
                } else {
                    move_delta
                };
                transform.translation += clamped_delta;
            }
            return false;
        }

        // We're within the threshold to the target.
        transform.translation.x = self.target_location.x;
        transform.translation.y = self.target_location.y;

        // Request the out-of-move sequence once when the move order completes.
        sequencer.request(
            SequencerRequest::new(SequenceName::OutOfWalk)
                .with_speed(0.4)
                .with_clear_on_change(true),
        );

        // Order is done.
        true
    }
}

impl Default for OrderMove {
    fn default() -> Self {
        Self {
            target_location: Vec3::ZERO,
            move_speed: 61.44,
            rotation_speed: 3.0,
            is_moving: false,
            has_requested_forward_locomotion: false,
        }
    }
}
