# Sequences

## Requesting new sequences

### Scope

This section specifies runtime behavior of:

`bool Motion_Sequencer::Request_Sequence(Motion_Sequence_Request* request)`

The goal of this function is to convert a high-level sequence request into one or more queued `Motion_Info` entries on an object's `Motion_Controller`, including optional posture-transition sequences.

### Inputs

`Motion_Sequence_Request` should be treated as a semantic "play sequence" command:

- `target_object` (`m_object`): object whose motion controller receives queued motions.
- `requested_sequence` (`m_motion_hash`): identity of the sequence to play. `0xFFFF_FFFF` means "no sequence" and fails fast.
- `dedupe_if_same_front_motion` (`m_dedupe_if_same_sequence`): if enabled, the request is accepted without enqueue when the first motion of the requested sequence matches the currently queued motion.
- `playback_speed` (`m_playback_speed`): speed scalar attached to every enqueued `Motion_Info` from transition and target sequences.
- `force_clear_queue_on_dedupe_mismatch` (`m_force_clear_queue`): if dedupe is enabled and hashes differ, reset/clear pending motion queue before enqueue.
- `skip_posture_transition` (`m_skip_posture_transition`): if enabled, do not inject transition sequence from current posture state to target begin state.
- `first_motion_start_ticks` (`m_first_entry_flags`): value copied into `m_start_time_ticks` on the first motion of the target sequence only.

Derived object state:

- `Motion_Sequence_Request::Get_Object_Motion_Controller` returns `request.m_object?.m_motion_controller`
- If object or controller is null, request fails

### Required data in `Motion_Sequencer`

- Sequence lookup table/tree keyed by hash (`Find_Sequence(hash)`)
- Transition matrix containing transition `Sequence` values indexed by:
  - current controller state (`Motion_Controller::Get_State()`)
  - requested sequence begin state (`found_sequence.m_begin_state`)
- Transition matrix size is treated as `7 x 7` entries in runtime indexing

### Return value contract

- Returns `false` when request is invalid or cannot resolve target sequence/controller
- Returns `true` when:
  - dedupe short-circuit accepts "already queued"
  - or enqueue path is executed for transition and/or target sequence

Note: enqueue helper return values are not propagated by `Request_Sequence`; the function still returns `true` after entering enqueue loops.

### Behavioral specification (ordered)

1. Resolve the target object's motion controller from the request.
2. If the request does not name a valid sequence (`m_motion_hash == 0xFFFF_FFFF`):
   1. Return `false`.
3. If the target object has no motion controller:
   1. Return `false`.
4. Resolve the requested sequence from the sequencer lookup (`Find_Sequence`).
5. If the sequence is not found:
   1. Return `false`.
6. If `request.m_dedupe_if_same_sequence` is true:
   1. Compare the first motion in the requested sequence against the most recently queued motion.
   2. If they match:
      1. Treat this as "already queued" and return `true`.
   3. If they do not match and `request.m_force_clear_queue` is true:
      1. Reset the controller so the new request starts from a clean queue and motion state.
7. If `request.m_skip_posture_transition` is false:
   1. Determine current controller posture and requested sequence start posture.
   2. If the two postures differ:
      1. Resolve a transition sequence between them.
      2. If posture values are invalid for transition lookup:
         1. Log an invalid-transition error.
      3. If a usable transition sequence exists:
         1. Queue all transition motions at the requested playback speed.
8. Queue all requested sequence motions at the requested playback speed.
   1. For the first requested motion only:
      1. Apply the request-provided start-time override (`m_first_entry_flags`).
9. Return `true`.

### Dedupe semantics

Dedupe compares:

- hash of first motion in target sequence
- hash of motion referenced by current queued request/context

If equal, no reset and no enqueue happen; request is treated as success.

### Transition behavior

- Transition insertion happens before target sequence entries.
- Transition is skipped when:
  - `m_skip_posture_transition == true`
  - or current state already equals target begin state
  - or transition matrix entry is effectively empty (`STATE_NONE` guards)

### Queueing behavior

`Request_Sequence` uses `Motion_Controller::Enqueue_Motion_Info(motion_info, playback_speed)`:

- Clears transition guard in controller motion state
- Allocates a `Motion_Info_Context`
- Writes `motion_info` and `playback_speed` into context
- If `motion_info.m_immediate_` is true, controller `Reset()` is called before enqueue
- Context is pushed into pending queue

### Controller reset side effects

`Motion_Controller::Reset` performs:

- Clear all pending requests/contexts
- Zero object root motion vector
- Clear physics flag `0x200`
- Disable current motion info (`m_enabled_ = false`)

### State enum notes

Observed state strings include:

- `MSEQ_STATE_NO_STATE`
- `MSEQ_STATE_STAND`
- `MSEQ_STATE_CROUCH`
- `MSEQ_STATE_PRONE`
- `MSEQ_STATE_ON_BACK`
- `MSEQ_STATE_SIT`
- `MSEQ_STATE_SCUBA`

`Sequence::Clear` initializes begin/end state to `STATE_NONE` and hash to `0xFFFF_FFFF`.

## Updating sequence controllers each frame

### Scope

This section specifies runtime behavior of:

- `Motion_Controller::Update_Motion_Transition(int frame_count)` (`0x0049cbd0`)
- `Motion_Controller::Begin_Queued_Motion_Transition()` (`0x0049cb20`)
- `Motion_Controller::Activate_Next_Pending_Request()` (`0x0049c9c0`)
- `Motion_Controller::Peek_Pending_Motion_Info()` (`0x0049ca90`)

Primary caller path each frame:

- `Object::FUN_0041cf50` (`0x0041cf50`) calls `Update_Motion_Transition(motion_controller, DAT_00572974)`

### `frame_count` input semantics

- The `frame_count` parameter is a frame-step **time delta in milliseconds**, not a literal frame index/count and not seconds.
- Source:
  - `FUN_0046b090` updates `g_event_processor + 0x1354` (global `DAT_00572974`) each main-loop iteration.
  - It computes: `delta_ms = timeGetTime() - previous_timeGetTime`.
  - It stores that delta into `DAT_00572974`.
- Clamp behavior:
  - If `delta_ms > 125`, it is clamped to `125` before storage/use.
  - This limits large hitches from producing extreme per-update jumps.
- Usage meaning inside motion update:
  - `Update_Motion_Transition(..., frame_count)` treats `frame_count` as elapsed motion time units in ms.
  - Motion timers/countdowns are advanced or reduced by this delta (optionally scaled for `DECLARE_SPED_MOTION`).
  - For engine code that uses seconds-based timers, convert with: `delta_seconds = frame_count as f32 / 1000.0`.

### Queue model used during update

- Pending motion requests are stored as `Motion_Info_Context` nodes.
- `Activate_Next_Pending_Request` consumes `m_pending_head` (oldest pending entry).
- `Get_Current_Queued_Request` reads `m_pending_tail` (most recently queued entry), used by dedupe logic during request time.

Practical meaning:

- Queue consumption is FIFO (head pops first).
- "Most recently queued" checks use tail.

### Per-frame update behavior (ordered)

1. Receive `frame_count` delta ticks from caller.
2. If active motion is enabled and has `DECLARE_SPED_MOTION`:
   1. Scale delta by `1.5x` (`(frame_count * 3) / 2`).
3. Advance the active motion clock by the scaled delta.
4. If no pending and no active motion:
   1. Mark controller as idle/unlocked.
5. Peek pending queue head (`Peek_Pending_Motion_Info`).
6. If no pending entry:
   1. If active motion is enabled:
      1. Compute whether the active motion has reached its playable end.
      2. If active motion reached end:
         1. Apply the terminal key frame when available.
         2. If `m_transition_guard == false` and `m_repeat_count < 1`:
            1. Allow transition to a queued motion.
         3. Else:
            1. Consume a repeat (when repeats remain).
            2. Wrap motion time for the next repetition cycle.
            3. Reset root-motion accumulation for the new cycle.
      3. If still active:
         1. Advance keyframe sampling and callbacks for the active motion.
         2. Return whether advancement succeeded.
   2. If no active motion:
      1. Keep the controller in idle behavior.
      2. Return `true`.
7. If a pending entry exists:
   1. Clear transition-guard blocking so a handoff can occur when allowed.
   2. If pending head motion is **not** immediate:
      1. If active motion is enabled:
         1. Keep running the current active motion this frame.
      2. If active motion is disabled:
         1. Promote the queued motion to active now.
   3. If pending head motion **is** immediate:
      1. Interrupt and promote the queued motion now.
   4. On switch path, if current motion is active and notify-on-interrupt is enabled:
      1. Emit the interrupt callback/event (`0x0D`).
   5. Start queued transition via `Begin_Queued_Motion_Transition()`.
   6. Return its result.

### `Begin_Queued_Motion_Transition` behavior

1. Reset transition-local runtime state (root accumulator and frame-tracking baseline).
2. Call `Activate_Next_Pending_Request()`.
3. If activation fails (queue empty):
   1. Disable active motion playback.
   2. Remove root-motion contribution from the object and clear the related physics flag (`0x200`) if object exists.
   3. Return `false`.
4. If activation succeeds:
   1. Compute effective playback tick rate from motion base timing and queued playback speed.
   2. Start the newly activated motion at its configured start-time offset.
   3. Clear immediate-interrupt state on the active motion.
   4. Update the controller target posture to match the new motion's end posture.
   5. Return `true`.

### `Activate_Next_Pending_Request` behavior

1. Read the oldest pending queue entry (`m_pending_head`).
2. If the queue is empty:
   1. Return `false`.
3. Promote queued motion data into the controller's active motion state (`Motion_Info::Copy_To`).
4. Apply queued playback speed to the active motion runtime.
5. Release the consumed queue context allocation.
6. Remove the consumed node from the pending list and update queue count.
7. Refresh active-motion identity and duration cache values.
8. Return `true`.

### Selection rules summary

- A non-immediate queued motion waits until active motion can transition out.
- An immediate queued motion interrupts into switch path immediately.
- Repeats/transition-guard can keep the current active motion running even after nominal end.
- Queue head is always the next activation candidate.
