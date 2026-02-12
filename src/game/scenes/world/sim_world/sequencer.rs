use std::collections::VecDeque;

use bevy_ecs::prelude::*;

use crate::{
    engine::storage::Handle,
    game::{AssetReader, scenes::world::animation::motion::Motion},
};

use super::sequences::{
    MotionCallback, Repeat, Sequence, SequenceName, SequencerCallbackEvent, Sequences, State,
};

/// A high-level sequence request consumed by [Sequencer].
#[derive(Clone, Debug)]
pub struct SequencerRequest {
    pub sequence_name: SequenceName,
    dedupe: bool,
    force_restart: bool,
    clear_on_change: bool,
    transitions: bool,
    speed: f32,
}

impl SequencerRequest {
    /// Create a new request for `sequence_name`.
    ///
    /// By default requests dedupe against active/pending entries and will not
    /// forcibly restart playback.
    pub fn new(sequence_name: SequenceName) -> Self {
        Self {
            sequence_name,
            dedupe: true,
            force_restart: false,
            clear_on_change: false,
            transitions: true,
            speed: 1.0,
        }
    }

    /// Disable or enable deduplication against active and queued sequence entries.
    pub fn with_dedupe(mut self, dedupe: bool) -> Self {
        self.dedupe = dedupe;
        self
    }

    /// Force an immediate restart/replacement of the current queue.
    pub fn force_restart(mut self) -> Self {
        self.force_restart = true;
        self
    }

    /// Clear current queued playback when requesting a different sequence.
    pub fn with_clear_on_change(mut self, clear_on_change: bool) -> Self {
        self.clear_on_change = clear_on_change;
        self
    }

    /// Set playback speed for entries created by this request.
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    /// Enable or disable transition-sequence injection for this request.
    pub fn with_transitions(mut self, transitions: bool) -> Self {
        self.transitions = transitions;
        self
    }
}

/// Play a sequence of motions in order.
#[derive(Component, Debug)]
pub struct Sequencer {
    entries: VecDeque<SequencerEntry>,
    time: f32,
    current_state: Option<State>,
    next_request: Option<SequencerRequest>,
    pending_callbacks: VecDeque<SequencerCallbackEvent>,
}

impl Default for Sequencer {
    fn default() -> Self {
        Self {
            entries: VecDeque::default(),
            time: 0.0,
            // Match original controller behavior: default posture state is stand.
            current_state: Some(State::Stand),
            next_request: None,
            pending_callbacks: VecDeque::default(),
        }
    }
}

#[derive(Debug)]
struct SequencerEntry {
    sequence_name: SequenceName,
    motion: Handle<Motion>,
    motion_time: f32,
    motion_fps: f32,
    use_linear_velocity: bool,
    is_transition: bool,
    playback_speed: f32,
    end_state: State,
    callbacks: Vec<MotionCallback>,
    play_time: f32,
}

impl Sequencer {
    /// Minimum non-zero playback speed to keep time advancement stable.
    const MIN_PLAYBACK_SPEED: f32 = 0.01;
    /// Minimum non-zero motion frame rate used for time conversion safety.
    const MIN_MOTION_FPS: f32 = 0.01;

    /// Request playback of a sequence by name.
    ///
    /// This is a convenience wrapper over [Sequencer::request] with default
    /// [SequencerRequest] settings.
    #[inline]
    pub fn play(&mut self, sequence: SequenceName) {
        self.request(SequencerRequest::new(sequence));
    }

    /// Request playback of a known [SequenceName].
    #[inline]
    pub fn play_name(&mut self, sequence: SequenceName) {
        self.request(SequencerRequest::new(sequence));
    }

    /// Queue a high-level sequence request to be applied by the sequence system.
    #[inline]
    pub fn request(&mut self, request: SequencerRequest) {
        if let Some(previous) = self.next_request.as_ref() {
            tracing::debug!(
                previous = %previous.sequence_name,
                replacement = %request.sequence_name,
                "Replacing pending sequence request before it was consumed",
            );
        }
        self.next_request = Some(request);
    }

    /// Take the pending request, if any.
    #[inline]
    pub fn next_request(&mut self) -> Option<SequencerRequest> {
        self.next_request.take()
    }

    /// Reset queued playback and pending requests while preserving controller state.
    ///
    /// Mirrors the original controller reset behavior used before posture changes.
    pub fn reset(&mut self) {
        self.entries.clear();
        self.pending_callbacks.clear();
        self.next_request = None;
        self.time = 0.0;
    }

    /// Return the currently active high-level sequence name, if any.
    #[inline]
    pub fn current_sequence_name(&self) -> Option<SequenceName> {
        self.entries.front().map(|entry| entry.sequence_name)
    }

    /// Return whether the currently playing entry is a posture-transition clip.
    #[inline]
    pub fn is_transition_active(&self) -> bool {
        self.entries
            .front()
            .is_some_and(|entry| entry.is_transition)
    }

    /// Return whether this sequencer still has an unconsumed request.
    #[inline]
    pub fn has_pending_request(&self) -> bool {
        self.next_request.is_some()
    }

    /// Pop the next pending callback that was emitted during playback.
    #[inline]
    pub fn pop_callback(&mut self) -> Option<SequencerCallbackEvent> {
        self.pending_callbacks.pop_front()
    }

    /// Apply a request using the resolved sequence definition.
    ///
    /// If the request dedupes and the same sequence is already active/pending,
    /// this does nothing. If the request forces restart or the first sequence
    /// entry is marked `immediate`, current playback is replaced immediately.
    /// Otherwise the sequence entries are appended.
    pub fn apply_request(
        &mut self,
        assets: &AssetReader,
        sequences: &Sequences,
        request: SequencerRequest,
        sequence_def: &Sequence,
    ) {
        if request.dedupe
            && self
                .entries
                .iter()
                .any(|entry| entry.sequence_name == request.sequence_name)
        {
            tracing::debug!(
                sequence = %request.sequence_name,
                "Skipping sequence request because it is already active or queued",
            );
            return;
        }

        let immediate_request = sequence_def
            .entries
            .first()
            .is_some_and(|entry| entry.immediate);
        let source_state = self.queued_end_state();
        let (target_start_state, target_end_state) =
            Self::sequence_state_bounds(assets, request.sequence_name, sequence_def);

        let mut requested_entries = Self::build_entries(
            assets,
            request.sequence_name,
            request.speed,
            target_end_state,
            Some(sequences),
            sequence_def,
        );

        if request.transitions
            && let Some(from_state) = source_state
            && from_state != target_start_state
        {
            if let Some(transition_motion) =
                sequences.transition_motion(from_state, target_start_state)
            {
                if let Some(transition_entry) = Self::build_transition_entry(
                    assets,
                    request.sequence_name,
                    transition_motion,
                    request.speed,
                    from_state,
                    target_start_state,
                    sequences.uses_linear_velocity(transition_motion),
                ) {
                    requested_entries.push_front(transition_entry);
                } else {
                    tracing::warn!(
                        sequence = %request.sequence_name,
                        from = ?from_state,
                        to = ?target_start_state,
                        transition_motion = ?transition_motion,
                        "Transition was selected but could not be built",
                    );
                }
            } else {
                /*
                tracing::warn!(
                    sequence = %request.sequence_name,
                    from = ?from_state,
                    to = ?State::Stand,
                    "No transition sequence defined for state change",
                );
                */
            }
        }

        if requested_entries.is_empty() {
            tracing::warn!(
                sequence = %request.sequence_name,
                "Sequence request produced no playable entries",
            );
            return;
        }

        let active_sequence_name = self.entries.front().map(|entry| entry.sequence_name);
        let sequence_changed =
            active_sequence_name.is_some_and(|name| name != request.sequence_name);

        if request.clear_on_change && sequence_changed {
            self.time = 0.0;
            self.pending_callbacks.clear();
            self.entries.clear();
        }

        if self.entries.is_empty() || request.force_restart || immediate_request {
            self.time = 0.0;
            self.pending_callbacks.clear();
            self.entries = requested_entries;
            return;
        }

        self.entries.append(&mut requested_entries);
    }

    /// Replace the current playback queue with `sequence_def`.
    pub fn enqueue(&mut self, assets: &AssetReader, sequence_def: &Sequence) {
        self.time = 0.0;
        self.entries = Self::build_entries(
            assets,
            SequenceName::Walk,
            1.0,
            Self::default_state_for_sequence(SequenceName::Walk),
            None,
            sequence_def,
        );
    }

    /// Advance sequence playback time and emit callbacks as entries progress.
    pub fn update(&mut self, delta_time: f32) {
        let mut remaining_seconds = delta_time.max(0.0);

        while remaining_seconds > 0.0 {
            let Some(front) = self.entries.front() else {
                self.time = 0.0;
                break;
            };

            let motion = front.motion;
            let play_time = front.play_time;
            let motion_time = front.motion_time;
            let playback_speed = front.playback_speed.max(Self::MIN_PLAYBACK_SPEED);
            let motion_fps = front.motion_fps.max(Self::MIN_MOTION_FPS);
            let callbacks = front.callbacks.clone();

            let before = self.time;
            let available = play_time - before;

            if available <= 0.0 {
                self.queue_notify_end_callback(motion, &callbacks, play_time);
                self.next();
                self.time = 0.0;
                continue;
            }

            let step_seconds_available = available / (playback_speed * motion_fps);
            let step_seconds = remaining_seconds.min(step_seconds_available);
            let local_step = step_seconds * playback_speed * motion_fps;
            let after = before + local_step;

            self.queue_frame_callbacks(motion, motion_time, &callbacks, before, after);

            self.time = after;
            remaining_seconds -= step_seconds;

            if self.time >= play_time {
                self.queue_notify_end_callback(motion, &callbacks, play_time);
                self.next();
                self.time = 0.0;
            }
        }
    }

    /// Play the next entry in the sequence.
    fn next(&mut self) {
        // Just pop the front, if it was or is empty, that is fine.
        if let Some(entry) = self.entries.pop_front() {
            self.current_state = Some(entry.end_state);
        }
    }

    /// Get the state that newly requested sequences should transition from.
    fn queued_end_state(&self) -> Option<State> {
        self.entries
            .back()
            .map(|entry| entry.end_state)
            .or(self.current_state)
    }

    /// Build a single transition entry.
    fn build_transition_entry(
        assets: &AssetReader,
        sequence_name: SequenceName,
        motion_handle: Handle<Motion>,
        playback_speed: f32,
        _from_state: State,
        to_state: State,
        use_linear_velocity: bool,
    ) -> Option<SequencerEntry> {
        let playback_speed = playback_speed.max(Self::MIN_PLAYBACK_SPEED);
        let motion = assets.get_motion(motion_handle)?;
        let motion_time = motion.max_frame_num().max(1) as f32;
        let motion_fps = motion.frames_per_second().max(Self::MIN_MOTION_FPS);

        Some(SequencerEntry {
            sequence_name,
            motion: motion_handle,
            motion_time,
            motion_fps,
            use_linear_velocity,
            is_transition: true,
            playback_speed,
            end_state: to_state,
            callbacks: Vec::new(),
            play_time: motion_time,
        })
    }

    /// Build runtime sequencer entries from a sequence definition.
    fn build_entries(
        assets: &AssetReader,
        sequence_name: SequenceName,
        playback_speed: f32,
        fallback_end_state: State,
        sequences: Option<&Sequences>,
        sequence_def: &Sequence,
    ) -> VecDeque<SequencerEntry> {
        let playback_speed = playback_speed.max(Self::MIN_PLAYBACK_SPEED);
        let log_sequence_name = sequence_name;

        let mut entries = VecDeque::with_capacity(sequence_def.entries.len());

        for entry in sequence_def.entries.iter() {
            let Some(motion) = assets.get_motion(entry.motion) else {
                tracing::warn!(
                    sequence = %log_sequence_name,
                    motion = ?entry.motion,
                    "Skipping sequence entry because motion asset is unavailable",
                );
                continue;
            };

            let motion_time = motion.max_frame_num().max(1) as f32;
            let motion_fps = motion.frames_per_second().max(Self::MIN_MOTION_FPS);
            let use_linear_velocity = sequences
                .map(|defs| defs.uses_linear_velocity(entry.motion))
                .unwrap_or(true);
            let end_state =
                Self::decode_motion_state(motion.to_state()).unwrap_or(fallback_end_state);
            let play_time = match entry.repeat {
                Repeat::None => motion_time,
                Repeat::Infinite => f32::INFINITY,
                Repeat::Count(count) => count.max(0) as f32 * motion_time,
            };
            entries.push_back(SequencerEntry {
                sequence_name,
                motion: entry.motion,
                motion_time,
                motion_fps,
                use_linear_velocity,
                is_transition: false,
                playback_speed,
                end_state,
                callbacks: entry.callbacks.clone(),
                play_time,
            });
        }

        entries
    }

    /// Decode the numeric posture state value stored in BMF headers.
    fn decode_motion_state(state: u32) -> Option<State> {
        match state {
            1 => Some(State::Stand),
            2 => Some(State::Crouch),
            3 => Some(State::Prone),
            4 => Some(State::OnBack),
            5 => Some(State::Sit),
            6 => Some(State::Scuba),
            _ => None,
        }
    }

    /// Fallback posture state for sequences when motion headers are missing/invalid.
    fn default_state_for_sequence(sequence_name: SequenceName) -> State {
        match sequence_name {
            SequenceName::Crouch => State::Crouch,
            SequenceName::Prone => State::Prone,
            _ => State::Stand,
        }
    }

    /// Resolve the requested sequence's start/end posture from motion metadata.
    fn sequence_state_bounds(
        assets: &AssetReader,
        sequence_name: SequenceName,
        sequence_def: &Sequence,
    ) -> (State, State) {
        let fallback = Self::default_state_for_sequence(sequence_name);
        let mut start_state = None;
        let mut end_state = None;

        for entry in sequence_def.entries.iter() {
            let Some(motion) = assets.get_motion(entry.motion) else {
                continue;
            };

            if start_state.is_none() {
                start_state = Self::decode_motion_state(motion.from_state());
            }

            if let Some(decoded_end_state) = Self::decode_motion_state(motion.to_state()) {
                end_state = Some(decoded_end_state);
            }
        }

        let start_state = start_state.unwrap_or(fallback);
        let end_state = end_state.unwrap_or(start_state);
        (start_state, end_state)
    }

    /// Queue frame callbacks crossed in the half-open interval `[from, to)`.
    fn queue_frame_callbacks(
        &mut self,
        motion: Handle<Motion>,
        motion_time: f32,
        callbacks: &[MotionCallback],
        from: f32,
        to: f32,
    ) {
        if to <= from || callbacks.is_empty() || motion_time <= 0.0 {
            return;
        }

        let start_cycle = (from / motion_time).floor() as i32;
        let end_cycle = ((to - f32::EPSILON) / motion_time).floor() as i32;

        let mut fired = Vec::new();

        for (order, callback) in callbacks.iter().enumerate() {
            let MotionCallback::Frame { frame, .. } = callback else {
                continue;
            };
            if *frame < 0 {
                continue;
            }

            let frame = *frame as f32;
            for cycle in start_cycle..=end_cycle {
                let at_frame = cycle as f32 * motion_time + frame;
                if at_frame >= from && at_frame < to {
                    fired.push((at_frame, order, callback.clone()));
                }
            }
        }

        fired.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));

        for (at_frame, _, callback) in fired {
            self.pending_callbacks.push_back(SequencerCallbackEvent {
                motion,
                callback,
                at_frame,
            });
        }
    }

    /// Queue `NotifyEnd` callbacks for the completed motion entry.
    fn queue_notify_end_callback(
        &mut self,
        motion: Handle<Motion>,
        callbacks: &[MotionCallback],
        at_frame: f32,
    ) {
        for callback in callbacks.iter() {
            if matches!(callback, MotionCallback::NotifyEnd) {
                self.pending_callbacks.push_back(SequencerCallbackEvent {
                    motion,
                    callback: callback.clone(),
                    at_frame,
                });
            }
        }
    }

    /// Get the currently playing motion handle and its local time.
    #[inline]
    pub fn get(&self) -> Option<(Handle<Motion>, f32)> {
        self.entries.front().map(|entry| (entry.motion, self.time))
    }

    /// Sample current motion root-motion delta in local space for `delta_time`.
    ///
    /// This mirrors the original engine flow where sampled root values are
    /// differenced frame-to-frame (`current_root - last_root`) before being
    /// rotated by object orientation.
    pub fn current_root_motion_delta(
        &self,
        assets: &AssetReader,
        delta_time: f32,
    ) -> Option<glam::Vec3> {
        let entry = self.entries.front()?;
        if !entry.use_linear_velocity {
            return None;
        }

        let motion = assets.get_motion(entry.motion)?;
        let fps = entry.motion_fps.max(Self::MIN_MOTION_FPS);
        let speed = entry.playback_speed.max(Self::MIN_PLAYBACK_SPEED);
        let frame_advance = delta_time.max(0.0) * fps * speed;
        if frame_advance <= f32::EPSILON {
            return Some(glam::Vec3::ZERO);
        }

        let from_play_time = self.time;
        let to_play_time = (self.time + frame_advance).min(entry.play_time);
        if to_play_time <= from_play_time {
            return Some(glam::Vec3::ZERO);
        }

        let motion_time = entry.motion_time.max(1.0);
        let from_local = Self::play_time_to_local_frame_start(from_play_time, motion_time);
        let mut remaining = to_play_time - from_play_time;
        let mut local = from_local;

        let root_at_end = motion.sample_linear_velocity(motion_time, false);
        let mut total_delta = glam::Vec3::ZERO;
        let last_root = if local <= f32::EPSILON {
            glam::Vec3::ZERO
        } else {
            motion.sample_linear_velocity(local, false)
        };

        // Segment 1: consume tail of the current cycle.
        let to_cycle_end = motion_time - local;
        let head = remaining.min(to_cycle_end);
        if head > f32::EPSILON {
            let end_local = local + head;
            let current_root = motion.sample_linear_velocity(end_local, false);
            total_delta += current_root - last_root;
            remaining -= head;
        }
        if remaining <= f32::EPSILON {
            return Some(total_delta);
        }

        // Segment 2: whole wrapped cycles.
        let full_cycles = (remaining / motion_time).floor();
        if full_cycles >= 1.0 {
            total_delta += root_at_end * full_cycles;
            remaining -= full_cycles * motion_time;
        }
        if remaining <= f32::EPSILON {
            return Some(total_delta);
        }

        // Segment 3: partial head of the next cycle (baseline resets to zero).
        local = 0.0;
        let end_local = Self::play_time_to_local_frame_end(local + remaining, motion_time);
        let current_root = motion.sample_linear_velocity(end_local, false);
        total_delta += current_root;

        Some(total_delta)
    }

    /// Sample current motion linear velocity in local space.
    pub fn current_linear_velocity(
        &self,
        assets: &AssetReader,
        delta_time: f32,
    ) -> Option<glam::Vec3> {
        if delta_time <= f32::EPSILON {
            return Some(glam::Vec3::ZERO);
        }

        self.current_root_motion_delta(assets, delta_time)
            .map(|delta| delta / delta_time)
    }

    /// Convert play-time frame position to local motion-frame index.
    ///
    /// At exact cycle boundaries (`n * motion_time`), this returns
    /// `motion_time` for non-zero `n` so end-of-motion samples are preserved.
    fn play_time_to_local_frame_end(play_time: f32, motion_time: f32) -> f32 {
        if play_time <= 0.0 {
            return 0.0;
        }

        let local = play_time.rem_euclid(motion_time);
        if local <= f32::EPSILON {
            motion_time
        } else {
            local
        }
    }

    /// Convert play-time frame position to local motion-frame index for a
    /// "from" sample. Exact cycle boundaries map to zero because root-motion
    /// baseline is reset there.
    fn play_time_to_local_frame_start(play_time: f32, motion_time: f32) -> f32 {
        if play_time <= 0.0 {
            return 0.0;
        }

        let local = play_time.rem_euclid(motion_time);
        if local <= f32::EPSILON { 0.0 } else { local }
    }
}

#[cfg(feature = "egui")]
impl Sequencer {
    pub fn ui(&self, ui: &mut egui::Ui, assets: &AssetReader) {
        use crate::engine::egui_integration::UiExt;

        if self.entries.is_empty() {
            return;
        }

        ui.h2("Sequences");

        for entry in self.entries.iter() {
            ui.group(|ui| {
                ui.set_width(ui.available_width());

                ui.horizontal(|ui| {
                    if let Some(motion) = assets.get_motion(entry.motion) {
                        ui.label(&motion.name);
                    }
                    ui.label(format!("Time: {}", entry.play_time));
                });
            });
        }
    }
}
