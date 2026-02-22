use bevy_ecs::prelude::*;
use glam::Vec3;

use std::{collections::VecDeque, sync::Arc};

use super::{motion::MotionFlags, motion_info::MotionInfo, state::State};

#[derive(Debug)]
pub struct MotionInfoContext {
    pub motion_info: Arc<MotionInfo>,
    pub playback_speed: f32,
}

#[derive(Debug)]
pub struct ActiveMotionInfo {
    pub motion_info: Arc<MotionInfo>,
    pub playback_speed: f32,
    pub current_time_ticks: i32,
    pub scaled_ticks_per_frame: i32,
    pub remaining_repeats: i32,
    pub transition_guard: bool,
}

#[derive(Component, Debug, Default)]
pub struct MotionController {
    pub pending: VecDeque<MotionInfoContext>,
    pub active: Option<ActiveMotionInfo>,

    /// The amount the object should move every frame.
    root_motion: Vec3,

    /// The [State] of the biped when all pending motions are completed.
    current_target_state: State,

    transition_guard: bool,

    reject_new_requests: bool,
}

impl MotionController {
    pub fn push_motion_info(&mut self, motion_info: Arc<MotionInfo>, playback_speed: f32) -> bool {
        self.transition_guard = false;

        if self.reject_new_requests {
            return true;
        }

        if motion_info.immediate {
            self.reset();
        }

        self.pending.push_back(MotionInfoContext {
            motion_info,
            playback_speed,
        });

        true
    }

    /// Returns the most recently queued motion.
    pub fn get_most_recent_motion(&self) -> Option<&Arc<MotionInfo>> {
        self.pending.back().map(|context| &context.motion_info)
    }

    /// Returns the state used as the source when resolving transition sequences.
    pub fn transition_check_state(&self) -> State {
        self.current_target_state
    }

    pub fn reset(&mut self) {
        self.pending.clear();
        self.active = None;

        self.root_motion = Vec3::ZERO;
        self.transition_guard = false;
    }

    /// Called each frame with the amount of time passed since the last update in `delta_time`.
    pub fn update(&mut self, delta_time: f32) {
        // The original runtime drives motion updates with a clamped millisecond delta.
        let mut delta_time_ms = (delta_time.max(0.0) * 1000.0).clamp(0.0, 125.0) as i32;
        let has_pending = !self.pending.is_empty();

        if self.active.as_ref().is_some_and(|active| {
            active
                .motion_info
                .motion
                .has_flags(MotionFlags::SPED_MOTION)
        }) {
            delta_time_ms = (delta_time_ms * 3) / 2;
        }

        let mut disable_active = false;
        if let Some(active) = self.active.as_mut() {
            if has_pending {
                // Match original behavior: pending work clears transition guard so looped
                // motions can hand off naturally at their next boundary.
                active.transition_guard = false;
            }

            active.current_time_ticks = active.current_time_ticks.saturating_add(delta_time_ms);

            if Self::is_active_motion_finished(active) {
                let duration_ticks = Self::active_motion_duration_ticks(active);

                if active.transition_guard || active.remaining_repeats > 0 {
                    if active.remaining_repeats > 0 {
                        active.remaining_repeats -= 1;
                    }

                    active.current_time_ticks =
                        active.current_time_ticks.saturating_sub(duration_ticks);
                    if active.current_time_ticks < 0 {
                        active.current_time_ticks = 0;
                    }
                    self.root_motion = Vec3::ZERO;
                } else {
                    disable_active = true;
                }
            }

            // Key frame sampling is done by the pose update system using `current_time_ticks`.
        }

        if disable_active {
            self.active = None;
            self.root_motion = Vec3::ZERO;
        }

        if self.pending.is_empty() && self.active.is_none() {
            self.root_motion = Vec3::ZERO;
            return;
        }

        // Pending work exists; allow transition/handoff logic to run.
        if has_pending {
            self.transition_guard = false;
        }

        let Some(next_is_immediate) = self
            .pending
            .front()
            .map(|pending| pending.motion_info.immediate)
        else {
            return;
        };

        if !next_is_immediate {
            if self.active.is_some() {
                return;
            }

            let Some(next) = self.pending.pop_front() else {
                return;
            };

            self.promote_to_active(next);
            return;
        }

        // Immediate motions should interrupt and hand off right away.
        let Some(next) = self.pending.pop_front() else {
            return;
        };

        if let Some(interrupted) = self.active.as_ref() {
            self.handle_immediate_interrupt(interrupted);
        }
        self.promote_to_active(next);
    }

    /// Promote a queued motion into active runtime state without cloning motion data.
    fn promote_to_active(&mut self, next: MotionInfoContext) {
        let scaled_ticks_per_frame =
            (next.motion_info.base_ticks_per_frame as f32 * next.playback_speed).round() as i32;
        let scaled_ticks_per_frame = scaled_ticks_per_frame.max(1);

        self.active = Some(ActiveMotionInfo {
            current_time_ticks: next.motion_info.start_time_ticks as i32,
            scaled_ticks_per_frame,
            playback_speed: next.playback_speed,
            remaining_repeats: next.motion_info.repeat_count.max(0),
            transition_guard: next.motion_info.transition_guard,
            motion_info: next.motion_info,
        });

        if let Some(active) = self.active.as_ref() {
            self.current_target_state = active.motion_info.motion.to_state;
        }

        self.transition_guard = false;
    }

    /// Handle an immediate handoff that interrupts the currently active motion.
    fn handle_immediate_interrupt(&self, interrupted: &ActiveMotionInfo) {
        tracing::debug!(
            "Interrupting active motion \"{}\" for immediate handoff.",
            interrupted.motion_info.motion.name
        );
        // Callback emission is intentionally deferred until callback parsing/runtime support exists.
    }

    /// Return whether the active motion has reached the end of its timeline.
    fn is_active_motion_finished(active: &ActiveMotionInfo) -> bool {
        let duration_ticks = Self::active_motion_duration_ticks(active);
        if duration_ticks <= 0 {
            return true;
        }
        active.current_time_ticks > duration_ticks.saturating_sub(active.scaled_ticks_per_frame)
    }

    /// Return duration of active motion timeline in ticks.
    fn active_motion_duration_ticks(active: &ActiveMotionInfo) -> i32 {
        let end_frame_count = Self::active_motion_end_frame_count(active);
        active
            .scaled_ticks_per_frame
            .saturating_mul(end_frame_count)
    }

    /// Return the end-frame count used for completion timing.
    fn active_motion_end_frame_count(active: &ActiveMotionInfo) -> i32 {
        // Original controller can finish a frame early when SKIP_LAST_FRAME is set.
        let mut end_frame_count = active.motion_info.motion.frame_count as i32;
        if active
            .motion_info
            .motion
            .has_flags(MotionFlags::SKIP_LAST_FRAME)
        {
            end_frame_count = end_frame_count.saturating_sub(1);
        }
        end_frame_count.max(0)
    }
}
