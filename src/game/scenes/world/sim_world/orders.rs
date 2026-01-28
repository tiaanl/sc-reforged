use bevy_ecs::prelude::*;
use glam::{Quat, Vec3};

use crate::engine::{egui_integration::UiExt, transform::Transform};

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

#[derive(Component, Default)]
pub enum Order {
    #[default]
    Idle,
    _Move(OrderMove),
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
        }
    }
}

pub struct OrderMove {
    pub target_location: Vec3,
    pub move_speed: f32,
    pub rotation_speed: f32,
}

impl OrderMove {
    pub fn update(&mut self, transform: &mut Transform, delta_time: f32) {
        // Turn to within this angle before moving.
        const ALIGN_ANGLE_RAD: f32 = 0.035; // ~2 degrees

        let to_target = self.target_location - transform.translation;
        let distance_sq = to_target.length_squared();
        if distance_sq <= 1.0e-6 {
            return;
        }

        let direction = to_target * distance_sq.sqrt().recip();

        let current_forward = transform.rotation * Vec3::Y;

        // Step 1: Turn towards the target.

        let delta_rot = Quat::from_rotation_arc(current_forward, direction);
        let (axis, angle) = delta_rot.to_axis_angle();
        let max_angle = self.rotation_speed * delta_time;
        if angle > 1.0e-6 {
            let clamped = angle.min(max_angle);
            let step = Quat::from_axis_angle(axis, clamped);
            transform.rotation = step * transform.rotation;
        }

        let aligned_dot = (transform.rotation * Vec3::Y).dot(direction);
        let aligned_threshold = ALIGN_ANGLE_RAD.cos();
        if aligned_dot < aligned_threshold {
            return;
        }

        // Step 2: Move towards the target.

        let move_step = self.move_speed * delta_time;
        if move_step >= distance_sq.sqrt() {
            transform.translation = self.target_location;
        } else {
            transform.translation += direction * move_step;
        }
    }
}

impl Default for OrderMove {
    fn default() -> Self {
        Self {
            target_location: Vec3::ZERO,
            move_speed: 100.0,
            rotation_speed: 3.0,
        }
    }
}
