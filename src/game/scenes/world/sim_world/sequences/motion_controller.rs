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
pub struct SampledMotionFrame {
    pub motion_info: Arc<MotionInfo>,
    pub current_time_ticks: i32,
    pub scaled_ticks_per_frame: i32,
    pub terminal_frame_index: Option<u32>,
}

#[derive(Debug)]
pub struct ActiveMotionInfo {
    pub motion_info: Arc<MotionInfo>,
    pub current_time_ticks: i32,
    pub scaled_ticks_per_frame: i32,
    pub remaining_repeats: i32,
    pub transition_guard: bool,
    pub last_root_sample: Vec3,
}

#[derive(Component, Debug, Default)]
pub struct MotionController {
    pub pending: VecDeque<MotionInfoContext>,
    pub active: Option<ActiveMotionInfo>,
    pub last_sampled_motion: Option<SampledMotionFrame>,

    /// The amount the object should move every frame.
    pub root_motion: Vec3,

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

    /// Returns the controller's effective current posture state.
    ///
    /// When motions are queued, this reflects the most recently queued motion's
    /// target state (the eventual posture after queued transitions complete).
    pub fn current_state(&self) -> State {
        self.pending
            .back()
            .map(|context| context.motion_info.motion.to_state)
            .unwrap_or(self.current_target_state)
    }

    pub fn reset(&mut self) {
        self.pending.clear();
        self.active = None;
        self.last_sampled_motion = None;

        self.root_motion = Vec3::ZERO;
        self.transition_guard = false;
    }

    /// Called each frame with the amount of time passed since the last update in `delta_time`.
    pub fn update(&mut self, delta_time: f32) {
        // The original runtime drives motion updates with a clamped millisecond delta.
        let mut delta_time_ms = (delta_time.max(0.0) * 1000.0).clamp(0.0, 125.0) as i32;
        let has_pending = !self.pending.is_empty();
        self.root_motion = Vec3::ZERO;

        if self.active.as_ref().is_some_and(|active| {
            active
                .motion_info
                .motion
                .has_flags(MotionFlags::SPED_MOTION)
        }) {
            delta_time_ms = (delta_time_ms * 3) / 2;
        }

        if has_pending {
            // Match original behavior: pending work clears transition guard so looped
            // motions can hand off naturally at their next boundary.
            if let Some(active) = self.active.as_mut() {
                active.transition_guard = false;
            }

            let Some(next_is_immediate) = self.pending.front().map(|p| p.motion_info.immediate)
            else {
                return;
            };

            if next_is_immediate {
                // Immediate requests interrupt active playback before active motion advancement.
                if let Some(interrupted) = self.active.as_ref() {
                    self.handle_immediate_interrupt(interrupted);
                }

                if let Some(next) = self.pending.pop_front() {
                    self.promote_to_active(next);
                }
                return;
            }

            // Non-immediate requests become active as soon as there is no active motion.
            if self.active.is_none() {
                if let Some(next) = self.pending.pop_front() {
                    self.promote_to_active(next);
                }
                return;
            }
        }

        let mut disable_active = false;
        let mut sampled_root_motion = None;
        let mut sampled_motion_frame = None;
        if let Some(active) = self.active.as_mut() {
            let mut terminal_frame_index = None;
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
                    active.last_root_sample = Vec3::ZERO;
                } else {
                    terminal_frame_index = Self::active_terminal_frame_index(active);
                    if let Some(frame_index) = terminal_frame_index {
                        active.current_time_ticks = (frame_index as i32)
                            .saturating_mul(active.scaled_ticks_per_frame.max(1));
                    } else {
                        // Fall back to duration-based clamping if no valid terminal keyframe exists.
                        active.current_time_ticks = duration_ticks.max(0);
                    }
                    disable_active = true;
                }
            }

            sampled_root_motion = Some(Self::sample_root_motion_delta(active));
            sampled_motion_frame = Some(SampledMotionFrame {
                motion_info: Arc::clone(&active.motion_info),
                current_time_ticks: active.current_time_ticks,
                scaled_ticks_per_frame: active.scaled_ticks_per_frame,
                terminal_frame_index,
            });

            // Key frame sampling is done by the pose update system using `current_time_ticks`.
        }

        if let Some(sampled_motion_frame) = sampled_motion_frame {
            self.last_sampled_motion = Some(sampled_motion_frame);
        }

        if disable_active {
            self.active = None;
            self.root_motion = Vec3::ZERO;
        } else if let Some(root_motion) = sampled_root_motion {
            self.root_motion = root_motion;
        }

        if self.active.is_none() {
            if let Some(next) = self.pending.pop_front() {
                self.promote_to_active(next);
                return;
            }

            self.root_motion = Vec3::ZERO;
            return;
        }
    }

    /// Promote a queued motion into active runtime state without cloning motion data.
    fn promote_to_active(&mut self, next: MotionInfoContext) {
        let scaled_ticks_per_frame =
            (next.motion_info.base_ticks_per_frame as f32 * next.playback_speed) as i32;
        let scaled_ticks_per_frame = scaled_ticks_per_frame.max(1);

        self.active = Some(ActiveMotionInfo {
            current_time_ticks: next.motion_info.start_time_ticks as i32,
            scaled_ticks_per_frame,
            remaining_repeats: next.motion_info.repeat_count.max(0),
            transition_guard: next.motion_info.transition_guard,
            motion_info: next.motion_info,
            last_root_sample: Vec3::ZERO,
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

    /// Return the explicit terminal frame index used for final keyframe application.
    fn active_terminal_frame_index(active: &ActiveMotionInfo) -> Option<u32> {
        let motion = active.motion_info.motion.as_ref();
        let frame_index = motion.last_frame;
        let frame_count = motion.frame_count;

        if frame_count == 0 {
            return None;
        }

        if frame_index < frame_count {
            return Some(frame_index);
        }

        None
    }

    /// Return the end-frame count used for completion timing.
    fn active_motion_end_frame_count(active: &ActiveMotionInfo) -> i32 {
        let motion = active.motion_info.motion.as_ref();

        // Original completion checks use frame-count duration (not end-frame-index).
        let mut end_frame_count = motion.frame_count as i32;

        if active
            .motion_info
            .motion
            .has_flags(MotionFlags::SKIP_LAST_FRAME)
        {
            end_frame_count = end_frame_count.saturating_sub(1);
        }

        end_frame_count.clamp(0, motion.frame_count as i32)
    }

    /// Sample the active motion's root-motion delta for this update.
    fn sample_root_motion_delta(active: &mut ActiveMotionInfo) -> Vec3 {
        if active
            .motion_info
            .motion
            .has_flags(MotionFlags::NO_LVE_MOTION)
        {
            // Match original apply path: NO_LVE motions do not apply root-motion deltas.
            return Vec3::ZERO;
        }

        if active.scaled_ticks_per_frame <= 0 {
            return Vec3::ZERO;
        }

        let end_frame_count = Self::active_motion_end_frame_count(active);
        if end_frame_count <= 0 {
            return Vec3::ZERO;
        }

        let ticks_per_frame = active.scaled_ticks_per_frame as f32;
        let motion = active.motion_info.motion.as_ref();
        let end_frame = end_frame_count as f32;

        let to_local_frame =
            (active.current_time_ticks.max(0) as f32 / ticks_per_frame).clamp(0.0, end_frame);

        let to_root = motion.sample_linear_velocity(to_local_frame, false);
        let delta = to_root - active.last_root_sample;
        active.last_root_sample = to_root;
        delta
    }
}
