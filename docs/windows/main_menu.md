# Main Menu Reconstruction

This note captures the current reverse-engineered understanding of how the original main menu is built and rendered.

The key functions are:
- `Window_Main_Menu::Constructor` at `0x004ce730`
- `Main_Menu_Button::Constructor` at `0x004cf690`
- `Main_Menu_Button::FUN_004cf770` at `0x004cf770`
- Main-menu render/update hook `FUN_004ceeb0` at `0x004ceeb0`
- Generic window render path `FUN_005031e0`
- `Window_Base::Render` at `0x004dfcc0`
- `Window_Base_Geometry_Tiled::Render` at `0x004e1350`

## Runtime Model

- `Window_Main_Menu` is a runtime window class derived from `Window`.
- `Window::Load_Window_Base(this, 0, 0, "main_menu")` attaches the parsed `Window_Base` definition loaded from `main_menu.txt`.
- The background art is not hard-coded in the window class. It comes from named `WINDOW_BASE_GEOMETRY_TILED` entries in the base definition.
- The interactive menu entries are child widgets of type `Main_Menu_Button`.

## Config Inputs

The constructor relies on these values from `main_menu.txt`:

- `WINDOW_BASE main_menu`
- `WINDOW_BASE_DX 640`
- `WINDOW_BASE_DY 480`
- `WINDOW_BASE_RENDER_DX 0`
- `WINDOW_BASE_RENDER_DY 0`
- `DEFINE_USER_IVAR button_offset_x 30`
- `DEFINE_USER_IVAR button_offset_y 5`
- `DEFINE_USER_IVAR shadow_offset_x 43`
- `DEFINE_USER_IVAR shadow_offset_y 13`
- `DEFINE_BUTTON_ADVICE` entries for the 7 menu buttons
- 5 `WINDOW_BASE_GEOMETRY_TILED` blocks named `frame1` through `frame5`

Each geometry block uses:

- `GEOMETRY_JPG_DIMENSIONS 640 512`
- `GEOMETRY_CHUNK_DIMENSIONS 128 128`

That means each frame is drawn as a `5 x 4` tile grid of `128 x 128` chunks. The source image is taller than the visible window, so the bottom is clipped by the `640 x 480` window rect.

## Construction Sequence

`Window_Main_Menu::Constructor` does the following:

1. Call `Window::Constructor`.
2. Install the `Window_Main_Menu` vtable.
3. Reset a few global/menu-manager states.
4. Abort if a main menu instance already exists.
5. Clear several inherited window flags.
6. Load the `"main_menu"` base definition with `Window::Load_Window_Base`.
7. Set the window title to `"Shadow Company: Main Menu"`.
8. Resolve the 4 user ivars:
   - `button_offset_x`
   - `button_offset_y`
   - `shadow_offset_x`
   - `shadow_offset_y`
9. Resolve the `DEFINE_BUTTON_ADVICE` rectangles for each named button.
10. Allocate and construct 7 `Main_Menu_Button` widgets.
11. Add all 7 buttons to the window with `Window_Parent::Add_Child_Widget`.
12. Build a bottom-right version label `"v1.31"`.
13. Add the main menu window to the `Window_Manager`.
14. Look up 5 named geometries from the `Window_Base`:
   - `frame1`
   - `frame2`
   - `frame3`
   - `frame4`
   - `frame5`
15. Cache those geometry pointers in the window object.
16. Set initial geometry alpha values:
   - `frame1.alpha = 1.0`
   - `frame2.alpha = 1.0`
   - `frame3.alpha = 0.0`
   - `frame4.alpha = 0.0`
   - `frame5.alpha = 0.0`
17. Initialize the frame-animation state:
   - `phase = 0`
   - `fade_out = 1`
   - `last_tick = timeGetTime()`
18. Perform some global UI/audio setup.
19. Purge textures through the texture manager.

## Button Layout

All 7 buttons are placed from `DEFINE_BUTTON_ADVICE`. The constructor uses the advice rect only for the top-left anchor. The actual width and height come from the atlas frames passed into the button ctor.

Button positions:

| Button | Position |
| --- | --- |
| `b_new_game` | `(325, 80)` |
| `b_load_game` | `(320, 120)` |
| `b_training` | `(315, 160)` |
| `b_options` | `(310, 200)` |
| `b_intro` | `(305, 240)` |
| `b_multiplayer` | `(300, 280)` |
| `b_exit` | `(295, 320)` |

Shared offsets:

- text/base offset: `(30, 5)`
- shadow offset: `(43, 13)`

## Button Resources And Actions

All buttons use one common atlas/resource in the first visual slot:

- common art resource: `DAT_00579b0c`
- common art frame index: `3`

The second and third visual slots vary by button and appear to be label-text atlases plus their secondary or shadow frames.

Observed per-button setup:

| Button | Label atlas | Label index | Secondary atlas | Secondary index | Action |
| --- | --- | --- | --- | --- | --- |
| `b_new_game` | `DAT_00579b04` | `0` | `DAT_00579b04` | `1` | custom callback `FUN_004ce6e0` |
| `b_training` | `DAT_00579b10` | `0` | `DAT_00579b10` | `1` | command id `7` |
| `b_options` | `DAT_00579b08` | `0` | `DAT_00579b08` | `1` | command id `51` |
| `b_exit` | `DAT_00579b08` | `3` | `DAT_00579b08` | `4` | command id `70` |
| `b_load_game` | `DAT_00579b00` | `0` | `DAT_00579b00` | `1` | command id `53` |
| `b_multiplayer` | `DAT_00579b04` | `3` | `DAT_00579b04` | `4` | command id `29` |
| `b_intro` | `DAT_00579b00` | `3` | `DAT_00579b00` | `4` | command id `3` |

`FUN_004ce6e0` allocates a new `0xF8` object and calls `FUN_004e4600`, so the `New Game` button is special-cased rather than using the generic numeric dispatch path.

## Main_Menu_Button Shape

`Main_Menu_Button::FUN_004cf770` shows that each button is a composite widget:

- one shared base art frame
- one label frame
- one secondary or shadow frame
- per-button text and shadow offsets
- derived width and height from the atlas frame rectangles

The helper computes:

- base label width and height
- second label width and height
- shadow width and height
- delta from text offset to shadow offset

Then it calls the generic widget rect/setup path:

- `FUN_004cffa0(this, left, top)`
- `FUN_004f4b80(this, left, top, width, height, ...)`

Two button-specific overrides exist:

- `FUN_004cf730`: sets `field_0x80 = 0`
- `FUN_004cf750`: sets `field_0x80 = 1`

Those look like pressed or highlighted state toggles before delegating to the common widget refresh path.

## Main Menu Render / Animation Hook

The main menu-specific runtime hook is `FUN_004ceeb0`.

This function combines animation and rendering. There is no separate cleanly-isolated `tick` step for the menu background; the draw hook advances the alpha animation each time it runs.

### Frame State

The window caches 5 geometry pointers at:

- `+0xf8` = `frame1`
- `+0xfc` = `frame2`
- `+0x100` = `frame3`
- `+0x104` = `frame4`
- `+0x108` = `frame5`

Animation fields:

- `+0x10c` = `phase`
- `+0x110` = `last_tick`
- `+0x114` = `fade_out`

Fade rate constant:

- `_DAT_0053ba08 = 0.0004f`

### Animation Logic

High-level pseudocode that matches `FUN_004ceeb0`:

```c
frames = [frame1, frame2, frame3, frame4, frame5];
current = frames[phase];
dt_ms = timeGetTime() - last_tick;
step = dt_ms * 0.0004f;

if (fade_out) {
    current->alpha -= step;
    clamp current->alpha to [0.0, 1.0];

    if (current->alpha <= 0.0f) {
        current->alpha = 0.0f;

        if (phase == 3) {
            fade_out = false;
        } else {
            phase += 1;
            frames[phase + 1]->alpha = 1.0f;
        }
    }
} else {
    current->alpha += step;
    clamp current->alpha to [0.0, 1.0];

    if (current->alpha >= 1.0f) {
        current->alpha = 1.0f;

        if (phase == 0) {
            fade_out = true;
        } else {
            phase -= 1;
            frames[phase + 2]->alpha = 0.0f;
        }
    }
}

last_tick = now;
Window::Render(this);
```

Important detail:

- This is not a simple adjacent cross-fade.
- The transition points explicitly write `0.0` or `1.0` into other frame slots as `phase` changes.
- If you want to match the original exactly, reproduce the state machine as above rather than simplifying it to a generic ping-pong blend.

## Generic Window Render Path Used By Main Menu

After the alpha step, the menu delegates to `FUN_005031e0`, the inherited `Window` render path.

That render path does:

1. Compute `x`, `y`, `width`, `height` from the window rect.
2. Call `FUN_004ccfb0(x, y)` to set up renderer state for this window.
3. If the window is in the special motif-only mode at `+0xe0`, draw the generic window motif texture.
4. Otherwise:
   - if there is a `Window_Base` with geometry entries, call `Window_Base::Render`
   - else draw a fallback motif and optional title/header area
5. Render child widgets with `FUN_00503450`.
6. Optionally restore renderer state if a flag at `+0x5d` is set.

For the main menu, the normal path is:

- `Window_Base::Render`
- then child widgets

## Window_Base::Render

`Window_Base::Render` iterates the base geometry list and renders each geometry whose alpha is greater than `0.0`.

The render call uses:

- `window_base.render_dx + window_x`
- `window_base.render_dy + window_y`

For `main_menu.txt`, both render offsets are `0`, so the geometries render directly at the window origin.

## Window_Base_Geometry_Tiled::Render

The tiled-geometry renderer does the following:

1. Bail out unless:
   - textures are loaded
   - texture array is non-null
   - `alpha != 0.0`
2. Convert `alpha` into a vertex diffuse color with white RGB and alpha in the high byte.
3. If alpha is not fully opaque, switch to a blended render state.
4. For each tile in `m_tiles_y * m_tiles_x`:
   - compute the quad corners from `chunk_width` and `chunk_height`
   - bind the matching tile texture through `Texture_Manager::Bind_Texture`
   - issue `IDirect3DDevice3::DrawIndexedPrimitive`
5. Restore render state if blending was enabled.

Because `main_menu.txt` uses `640x512` source images and `128x128` chunks, each frame layer renders as 20 textured quads.

## Child Widget Render

`FUN_00503450` iterates the `Window_Parent` child-widget list and calls each visible child's render vtable entry.

For the main menu that means:

1. draw background geometry layers first
2. draw the 7 menu buttons
3. draw the version label last

## Practical Recreation Plan

To recreate the screen faithfully:

1. Build a `640 x 480` window rooted at `(0, 0)`.
2. Load five tiled background layers named `frame1` through `frame5`.
3. Slice each layer into `128 x 128` chunks from a `640 x 512` source image.
4. Initialize alphas to `1, 1, 0, 0, 0`.
5. Implement the exact `phase` and `fade_out` state machine from `FUN_004ceeb0`.
6. Create 7 composite button widgets at the config positions.
7. Use atlas-derived button sizes rather than the zero width and height in `DEFINE_BUTTON_ADVICE`.
8. Use `(30, 5)` for the text offset and `(43, 13)` for the shadow offset.
9. Add a gold `"v1.31"` label at the bottom-right.
10. Render in this order:
    - background geometry layers
    - buttons
    - version label

## Current Unknowns

- The symbolic meaning of the numeric command ids `7`, `51`, `70`, `53`, `29`, and `3` is still not named in the database.
- The exact resource names behind `DAT_00579b00`, `DAT_00579b04`, `DAT_00579b08`, `DAT_00579b0c`, and `DAT_00579b10` are not yet resolved.
- The menu uses some global setup calls after construction that are likely audio or frontend state changes, but they do not affect layout reconstruction directly.
