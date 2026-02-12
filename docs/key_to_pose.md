1. Key/button input enters command-event layer
    - `0x00500cc0`: wrapper that calls FUN_00501070(key, 1) (key-down style path)
    - `0x00500cd0`: wrapper that calls FUN_00501070(key, 0) (key-up style path)
    - `FUN_00501070` maps active keybinds to command events and dispatches via FUN_00501270(event).


2. Pose events are 0x0B/0x0C/0x0D
    - In FUN_00501270, switch cases:
        - 0x0B -> stand
        - 0x0C -> crouch
        - 0x0D -> prone
    - Each case fetches selected units and calls:
        - `FUN_004aa320` (selected_list, selected_count, state, options)
        - state is 1/2/3 (`STATE_STAND`/`STATE_CROUCH`/`STATE_PRONE`).

3. `FUN_004aa320` builds and queues `ORDER_CHANGE_STATE` per selected object
    - Validates object can accept change-state orders and is not already in the target state.
    - Builds Order_Request with m_order_type = `ORDER_CHANGE_STATE` (`0x1C`).
    - Calls Exact_Type_Order_Create -> constructs Order_Change_State.
    - Calls Order_Controller::Add_Pending (`0x004acb30`).

4. Update tick executes pending orders
    - `Object_Scripted::Tick_Orders` (`0x00430ae0`) calls `FUN_004ac940` each tick.
    - `FUN_004ac940` iterates pending orders and calls order virtual execute.
    - For this order type, that execute is `Order_Change_State::Execute` (`0x004ad9f0`).

5. Order execute triggers action on the target object
    - `Order_Change_State::Execute` builds `Action_Request_Change_State` with `ACTION_CHANGE_STATE` and calls `target->Perform_Action`.
    - `Object_Bipedal::Perform_Action` (`0x00428080`) switches on action and calls `Action_Change_State` (`0x0042bdb0`).

6. `Action_Change_State` requests the exact `MSEQ_*`.
    - Resets motion controller and prepares `Motion_Sequence_Request` (`playback_speed = 0.8`).
    - For new state:
        - stand -> hash of `MSEQ_STAND`
        - crouch -> hash of `MSEQ_CROUCH`
        - prone -> hash of `MSEQ_PRONE`
    - Calls `Motion_Sequencer::Request_Sequence(&g_motion_sequencer, &seq_request)`.
    - Then forces stance via `Force_Stand_State` / `Force_Crouch_State` / `Force_Prone_State`.

7. Command-pad UI path converges into the same event/order pipeline
    - `FUN_004dd5e0` builds stand/crouch/prone command-pad buttons and binds events `0x0B` / `0x0C` / `0x0D`.
    - Callback code at `LAB_004ddeb0` checks button state fields (`field_0xf8`/`field_0xf9`/`field_0xfa`) and dispatches those events via `FUN_00501270`.
    - So hotkeys and command-pad clicks both converge to:
      `FUN_00501270 -> FUN_004aa320 -> ORDER_CHANGE_STATE -> Execute -> Perform_Action -> Action_Change_State -> Request_Sequence`.
