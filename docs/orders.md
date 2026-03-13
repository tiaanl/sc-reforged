# Orders System Mapping

## Scope

The orders subsystem appears to be a per-object runtime controller that owns:

- An active order list.
- A pending order list.
- A small embedded state block serialized as `order_mode_patrol`.
- A factory path that instantiates concrete order subclasses from an `Order_Type` enum.

The main confirmed entry points are:

- `Object_Scripted::Tick_Orders` at `0x00430ae0`
- `Object_Bipedal::Get_Order_Controller` at `0x00430b10`
- `Issue_Move_Order` at `0x004a9d70`
- `Issue_Move_Order_To_Selected` at `0x004a91b0`
- `Exact_Type_Order_Create` at `0x004b53d0`
- `Order_Controller::Add_Pending` at `0x004acb30`

## Top-Level Flow

The runtime flow reconstructed from Ghidra is:

1. A gameplay/UI/system function builds an `Order_Request`.
2. `Exact_Type_Order_Create` allocates the exact `Order_*` subclass for `request.m_order_type`.
3. The new order is pushed into the unit's controller through `Order_Controller::Add_Pending`.
4. `Tick_Orders` runs on the owning object every update and forwards to the controller.
5. The controller promotes pending orders into the active list according to order class and precedence rules.
6. Each active order gets `Execute(owner)` called.
7. If `Execute` returns `2`, the order stays active. Any other return observed here causes removal and deletion.

That `return == 2` convention is confirmed in the controller tick at `0x004ac940`.

## Object Attachment

`Object_Bipedal::Get_Order_Controller` returns the controller from object field `field4_0x1d4`:

- It calls the object's HP getter first.
- If HP is below `1`, it returns `NULL`.
- If a controller still exists on a dead object, it sets object flag `0x200`.

`Object_Scripted::Tick_Orders` then does:

- If object flag `0x200` is set, call `0x00430d40`, which destroys the controller and clears the pointer.
- If the controller pointer is non-null, call controller tick `0x004ac940`.

So controller lifetime is owned by the game object, and dead objects trigger deferred cleanup.

## Controller Layout

`Order_Controller` is a 108-byte type in the Ghidra database. Field names are incomplete, but behavior is fairly clear:

- `+0x00`: owner object pointer
- `+0x04/+0x08/+0x0c`: active list head, tail, cursor
- `+0x10..+0x1c`: a small cursor stack used while mutating the active list
- `+0x20`: cursor stack depth
- `+0x24`: active count
- `+0x2c/+0x30/+0x34`: pending list head, tail, cursor
- `+0x4c`: pending count
- `+0x54`: serialized controller flag
- `+0x5c`: embedded `order_mode_patrol` state

This is supported by:

- Controller destructor/clear path at `0x004ac500`
- Controller serialization at `0x004ac660`
- Active-order iteration in `0x004ac940`
- Pending-order arbitration/promotion in `0x004ad2d0`

## Creation Path

### `Order_Request`

`Order_Request` is a 56-byte type. The confirmed fields used in named callers are:

- `m_order_type`
- `m_subject`
- `m_location`
- `m_arg0`
- `m_arg1`
- `m_flags`
- `m_sequence_request`

Named issue helpers zero the request, call `Motion_Sequence_Request::Reset`, then fill only the fields needed for that order.

### Factory

`Exact_Type_Order_Create` at `0x004b53d0` is the central polymorphic factory.

Observed behavior:

- Switches on `request->m_order_type`
- Allocates a concrete `Order_*` object with `operator_new`
- Calls the concrete constructor
- Returns `false` on failure
- Logs `ERROR! Exact_Type_Order_Create() tried to create an unknown order...` on unknown enum values

The same factory is also used for save/load rehydration in `0x004b5be0`: the file stores the order type first, then the factory creates the matching subclass before deserializing its body.

## Queueing Semantics

### Adding Pending Orders

`Order_Controller::Add_Pending` at `0x004acb30` does the following:

- Rejects null order or null owner.
- Calls an owner vtable method at `+0xbc`; if that says no, the order is deleted immediately.
- Optionally writes a byte at order offset `+0x04`.
- Inserts into the pending list.

Insertion behavior is notable:

- If the pending list is empty, the new node becomes both head and tail.
- Otherwise, the new node is inserted at the head.

That means pending processing is effectively newest-first once the queue is non-empty.

The byte at order offset `+0x04` behaves like a precedence/priority byte:

- `Add_Pending` writes it when its second flag parameter is non-zero, or when `Get_Order_Class() == 3`.
- The pending arbitration logic compares that byte against the current active order and discards lower-or-equal candidates in some cases.

### Pending To Active Promotion

The unlabeled function at `0x004ad2d0` is the key pending-order arbitration step. Ghidra typed it incorrectly, but its behavior matches controller internals, not object physics.

Confirmed behavior:

- Iterates the pending list.
- Reads each candidate's `Get_Order_Class()` through the order vtable.
- Compares the candidate against the current active order.
- Either discards it, merges it, or promotes it to the active list.

Important observations:

- Order class drives the activation policy.
- The active-list mutation helper at `0x004acd30` marks existing active orders for retirement by setting byte `+0x19` on them.
- When `0x004acd30` is called with `0`, it exempts `ORDER_TRACK` (`0x14`) from cancellation.
- If both the current active order and the candidate report order class `1`, helper `0x004af2a0` is used instead of simply queueing both. This looks like a merge/update path for move-like orders.

I do not have fully stable human-readable names for the order classes yet, but the class value clearly matters as much as `Order_Type` for scheduling.

## Tick Loop

Controller tick is `0x004ac940`.

The core behavior is:

- Run pending arbitration/promotion first.
- If there are no active orders, notify the owner through vtable calls.
- Iterate the active list.
- For each active order:
  - If byte `+0x19` is set, delete/remove it immediately.
  - Otherwise, if byte `+0x18` is clear and enough time has passed since `m_time_created`, call `Execute(owner)`.
  - Keep the order only when `Execute` returns `2`.
  - Any other result removes the node and calls the order's delete method.

After an order is removed, the controller also:

- Touches owner flags/state
- Clears `RUN_INTENT` on the owner
- Notifies another linked object if present at owner offset `0x1b0`

The exact owner-side meaning of those notifications is outside this pass, but the order controller definitely participates in movement/intent cleanup when an order finishes.

## Base Order Contract

The base order vtable is at `0x0053abe4`.

Confirmed slots from the base and derived vtables:

- `+0x00`: destructor
- `+0x04`: struct size
- `+0x08`: serialize
- `+0x0c`: delete this
- `+0x14`: execute
- `+0x1c`: get order class
- `+0x20`: get order type

`Order::Serialize` at `0x004ad790` stores:

- byte `+0x04`
- dword `+0x08`
- dword `+0x0c`
- byte `+0x10`
- `m_time_created`
- byte `+0x18`
- byte `+0x19`
- byte `+0x1a`
- `m_og_data`

Field meanings are partly inferential, but two are strongly supported:

- byte `+0x04`: precedence/priority-like byte
- byte `+0x19`: cancel/remove marker used by controller maintenance

The `+0x08/+0x0c/+0x18` group appears to participate in delayed execution gating, because tick compares current time against stored timestamps before calling `Execute`.

## Confirmed Concrete Orders

### `ORDER_MOVE`

Named helpers:

- `Issue_Move_Order` at `0x004a9d70`
- `Issue_Move_Order_To_Selected` at `0x004a91b0`

`Issue_Move_Order`:

- Builds `ORDER_MOVE`
- Copies destination into `request.m_location`
- Sets `request.m_subject = object`
- Applies option-driven physics/timing data to the created `Order_Move`
- Enqueues it with `Add_Pending`

`Issue_Move_Order_To_Selected`:

- Enumerates the selected objects
- Applies formation offsets if available
- Creates one `ORDER_MOVE` per object
- Writes extra per-order fields after creation
- Enqueues each order separately

`Order_Move::Execute` is at `0x004af4b0`.

Observed behavior:

- Requires a valid subject object.
- Stops immediately if the subject has no local intelligence component.
- Performs path lookup / path refresh through helpers around `0x004c0010` and `0x004c0060`.
- Steers the unit by turning toward heading and accelerating forward.
- Can transition into an action phase on arrival using embedded RPC/action data.
- Returns `2` while still active.
- Returns `1` or `3` when it wants the controller to retire it.

This is the main evidence that orders are long-lived state machines, not one-shot commands.

### `ORDER_MOVE_TO_ATTACK`

`Order_Move_To_Attack::Execute` is at `0x004b0570`.

Observed behavior:

- Tracks a target object (`m_obj_to_attack`)
- Checks target validity and geometric relationship to the subject
- May set `RUN_INTENT`
- Uses helper `0x004b0920` before closing/attack resolution
- Falls back to direct attack/action helpers when in range
- Returns `2` while still pursuing the attack sequence

This looks like a composed order built on top of move-plus-engage behavior, not just a simple attack action.

### `ORDER_CHANGE_STATE`

`Order_Change_State::Execute` is at `0x004ad9f0`.

Observed behavior:

- Validates the target object
- Builds an `ACTION_CHANGE_STATE` request
- Calls `Perform_Action`
- Returns `2` if the action was not yet accepted
- Returns `3` once the state change is accepted

That makes `ORDER_CHANGE_STATE` a retry-until-accepted wrapper around the normal action system.

## Order Type Taxonomy

`Order_Type` is a real enum in the Ghidra database. The confirmed values are:

- `ORDER_NONE = 0`
- `ORDER_MOVE = 1`
- `ORDER_02 = 2`
- `ORDER_MOVE_TO_USE_VEHICLE = 3`
- `ORDER_MOVE_TO_ATTACK = 4`
- `ORDER_MOVE_TO_CUT_FENCE = 5`
- `ORDER_MOVE_TO_CRAWL_THROUGH_FENCE = 6`
- `ORDER_MOVE_TO_CLIMB_WALL = 7`
- `ORDER_MOVE_TO_PICK_UP = 8`
- `ORDER_MOVE_TO_USE_STRUCTURE = 9`
- `ORDER_MOVE_TO_DROP_ITEM = 10`
- `ORDER_MOVE_TO_ACTIVATE_STRUCTURE = 11`
- `ORDER_MOVE_TO_TRANSFER_ITEM = 12`
- `ORDER_MOVE_TO_TRANSFER_ITEM_WEAPONS_LOCKER = 13`
- `ORDER_MOVE_TO_TRANSFER_ITEM_FROM_BODY = 14`
- `ORDER_MOVE_TO_INVESTIGATE = 15`
- `ORDER_MOVE_TO_USE_LADDER = 16`
- `ORDER_MOVE_TO_INVESTIGATE_BODY = 17`
- `ORDER_MOVE_TO_USE_COVER = 18`
- `ORDER_MOVE_TO_AVOID_VEHICLE = 19`
- `ORDER_TRACK = 20`
- `ORDER_TRACK_TO_ATTACK = 21`
- `ORDER_DEFENSIVE_ATTACK = 22`
- `ORDER_FORCE_ATTACK = 23`
- `ORDER_EQUIP_SELF = 24`
- `ORDER_UNEQUIP_SELF = 25`
- `ORDER_UNKNOWN_1A = 26`
- `ORDER_UNKNOWN_1B = 27`
- `ORDER_CHANGE_STATE = 28`
- `ORDER_UNKNOWN_1D = 29`
- `ORDER_EXIT_STRUCTURE = 30`
- `ORDER_SCUBA = 31`
- `ORDER_USE_SPECIAL_ITEM = 32`
- `ORDER_MOVE_TO_USE_ITEM = 33`
- `ORDER_MOVE_TO_PLACE_ITEM = 34`
- `ORDER_DROP_ITEM = 35`
- `ORDER_UNKNOWN_24 = 36`
- `ORDER_MOVE_TO_LOOT_BODY = 37`

String anchors near `0x0055d450` through `0x0055de34` line up with many of these concrete types and their serialized member names.

## Serialization

The subsystem is fully serializable:

- Individual orders serialize polymorphically through the base vtable.
- Controller serialization lives at `0x004ac660` under the `object_orders` section.
- The controller saves both active and pending lists.
- The embedded `+0x5c` state serializes under `order_mode_patrol` at `0x004aaf10`.

This indicates orders are intended to survive save/load with their exact runtime subtype intact.

## Embedded `order_mode_patrol` State

The controller contains a serialized block named `order_mode_patrol`.

Confirmed facts:

- It stores a byte flag, a count, a pointer to `count * 0x0c` bytes, and an index.
- The pointer payload size strongly suggests an array of 3-float vectors.
- Tick calls helper `0x004ab020` on this block after active-order processing.
- That helper auto-creates an `ORDER_MOVE` when the block is enabled and the controller has no active orders.

What remains unclear:

- The current decompilation of `0x004ab020` does not clearly materialize a waypoint into `request.m_location`.
- The serialized data shape strongly suggests patrol points, but the exact waypoint-to-request handoff still needs manual stack/alias cleanup in Ghidra.

So the safe conclusion is: the controller has embedded patrol/autorepeat state that can inject move orders, but the exact location-feed path is not yet fully resolved.

## Practical Model

The safest current mental model is:

- Orders are heap-allocated polymorphic objects.
- Objects own a dedicated `Order_Controller`.
- New commands go to a pending list, not straight to execution.
- A controller helper arbitrates pending orders against active orders using order class and precedence.
- Active orders are long-lived state machines advanced by `Execute`.
- Completion is controller-driven through an integer return code contract.
- The system is designed to serialize and restore exact order runtime state.

## Open Questions

These are the remaining gaps after this pass:

- Human-readable meaning of every `Get_Order_Class()` value.
- Exact names for some shared controller helpers at `0x004ad2d0`, `0x004acd30`, and `0x004acf60`.
- Exact meaning of every base-order byte in the `0x18..0x1a` range.
- The precise location handoff for `order_mode_patrol`.
- Better names for the many unnamed issue helpers that also call `Exact_Type_Order_Create` and `Add_Pending`.

Even with those gaps, the queueing model, tick contract, serialization model, and the move / move-to-attack / change-state behaviors are all directly supported by the current Ghidra evidence.
