# Window Derived Classes

All 30 classes that derive from `Window` (constructor at `00503110`). Each calls
`Window::Constructor` and then installs its own vtable. Widgets are owned as a
flat linked list on `Window_Parent::m_children`.

## Inheritance Chain

```
Window_Parent_Parent (100 bytes)  — vtable, m_name, m_rect, flags
  └─ Window_Parent (244 bytes)   — m_children (Widget_List), input/layout state
       └─ Window (248 bytes)     — m_window_base (data-driven layout from config files)
            └─ <derived classes below>
```

## Window Class Table

| # | Constructor | VTable | Title / Name | Config File | Notes |
|---|-------------|--------|-------------|-------------|-------|
| 1 | `004cc600` | `0053b8a4` | "Communications" | `satcom_interface` | Satcom/radio communication interface. Creates 3 child widgets. |
| 2 | `004ce730` | `vtable_window_main_menu` | "Shadow Company: Main Menu" | `main_menu` | Main menu with 7 `Main_Menu_Button` widgets (New Game, Training, Options, Exit, Load Game, Multiplayer, Intro) plus a version label ("v1.31"). Stores 5 frame geometries for fade animation. Singleton stored at `DAT_0057991c`. |
| 3 | `004d68b0` | `0053c460` | "launch_status" | — | Launch status display. Sets rect and title directly, no config file. |
| 4 | `004d6e40` | `0053c510` | "Game: %s" / "Game: %s \<HOSTING\>" | `mplayer_game` | Multiplayer game lobby. Title formatted dynamically via `sprintf`. Creates ~20 child widgets including text buttons, tab bars, list boxes, text displays, and scroll bars. |
| 5 | `004d8930` | `0053c5c0` | "Select Network Service:" | `mplayer_service` | Multiplayer network service selection. Creates many child widgets including list boxes, text buttons, radio groups, text inputs, and setting widgets. Loaded from `mplayer_service` config. |
| 6 | `004db4e0` | `0053c670` | "Select Game Session" | `mplayer_session` | Multiplayer session browser. Creates ~14 child widgets including list boxes, text buttons, and labels. |
| 7 | `004dc6a0` | `0053c720` | "File: %s" (dynamic) | (from constructor param) | Generic file viewer window. Title formatted via `sprintf` from a filename parameter. Config file name also passed as a parameter. |
| 8 | `004dcb40` | `0053c7d0` | Bottom Bar | `bottombar_800x600` / `bottombar_640x480` | HUD bottom bar. Selects config based on screen resolution. Creates 6 child widgets including button widgets and text displays. |
| 9 | `004dd5e0` | `0053c884` | "Command Pad" | `command_pad` | In-game command pad. Creates 4 child widgets. |
| 10 | `004ddf10` | `0053c938` | "Inventory Bar" | `inventory_bar` / `inventory_bar_small` | In-game inventory bar. Selects config variant based on size. |
| 11 | `004e2c30` | `vtable_window_anim_sequence` | "Window Anim Sequence" | — | Animation sequence playback window. Sets rect directly, no config file. Named `Window_Anim_Sequence` in Ghidra. |
| 12 | `004e3aa0` | `0053cae8` | "chat" | `window_chat` | Multiplayer chat window. |
| 13 | `004e4070` | `0053cb98` | (dynamic from param) | — | Generic message/dialog box. Title passed as constructor parameter. Sets rect programmatically. |
| 14 | `004e4600` | `0053cc48` | "Diff Selection" | — | Difficulty selection dialog. Sets rect directly. |
| 15 | `004e49c0` | `0053ccf8` | (dynamic — quit/disconnect) | — | Confirmation dialog. Title determined by a switch on `param_1`. References string IDs `0x27`–`0x2e` for various quit/disconnect prompts. |
| 16 | `004e5070` | `0053cda8` | "Keys Display" | — | Key bindings display window. Sets rect programmatically. |
| 17 | `004e5680` | `0053ce58` | Load Screen (`Get_String_Reference(0xaf)`) | `loadscrn` | Loading screen. Title from string table. |
| 18 | `004e5b70` | `0053cf0c` | Key Assignment (`Get_String_Reference(0x25)`) | — | In-game key assignment/controls window. Delegates setup to `FUN_004e5c50`. |
| 19 | `004e66a0` | `0053cfec` | Key Assignment (`Get_String_Reference(0x2d)`) | — | Post-game key assignment/controls window variant. Also delegates to `FUN_004e5c50`. |
| 20 | `004e6ea0` | `0053d0cc` | "obj_display" | — | Objective display (likely internal/debug name). Sets rect programmatically. |
| 21 | `004e7430` | `vtable_game_options_window` | "Shadow Company Options" | `game_options` | Game options/settings window. Named `Game_Options_Window` in Ghidra. |
| 22 | `004e9a10` | `0053d2a8` | "Quick Menu" | — | In-game quick menu (pause/escape menu). Sets rect programmatically. |
| 23 | `004ea550` | `0053d358` | "sl_game" | `window_save_load` | Save/Load game window. |
| 24 | `004eabb0` | `0053d408` | "Window Subtitle" | — | Subtitle display overlay. Minimal setup (rect 0,0,1,1). |
| 25 | `004eb160` | `0053d4b8` | "Mission Equiping" | `mission_equip` | Mission equipment/loadout screen. Note original typo "Equiping". |
| 26 | `004ee2d0` | `0053d568` | "Mission HR" | `mission_hr` | Mission human resources — mercenary hiring/roster screen. |
| 27 | `004ef810` | `0053d618` | "Mission Planning" | `mission_planning` | Mission planning/briefing screen. Named `Window_Mission_Planning` in Ghidra. |
| 28 | `004f0490` | `0053d6c8` | "Objective Display" | — | In-game objective display overlay. Uses sprites, sets rect programmatically. |
| 29 | `004f2f80` | `0053d9f0` | "Help Window" | — | Help/tutorial window. Sets rect programmatically. |
| 30 | `004fe7a0` | `0053e584` | "Window Terrain" | — | Main game/terrain rendering window. Initializes video mode and terrain map. Fullscreen. |

## Groupings

### Main Menu Flow
- **Main Menu** (#2) → branches to New Game, Load Game, Training, Multiplayer, Options, Intro, Exit

### Multiplayer Flow
- **Network Service Selection** (#5) → **Session Browser** (#6) → **Game Lobby** (#4) → **Launch Status** (#3)
- **Chat** (#12) available during multiplayer

### Mission Flow
- **Mission HR** (#26) → **Mission Planning** (#27) → **Mission Equipping** (#25) → **Load Screen** (#17)

### In-Game HUD
- **Window Terrain** (#30) — main game view / rendering
- **Bottom Bar** (#8) — HUD bar with action buttons
- **Command Pad** (#9) — unit command interface
- **Inventory Bar** (#10) — item/weapon slots
- **Objective Display** (#20, #28) — mission objectives
- **Subtitle** (#24) — subtitle overlay
- **Quick Menu** (#22) — pause/escape menu

### Dialogs
- **Generic Dialog** (#13) — reusable message box with dynamic title
- **Confirmation Dialog** (#15) — quit/disconnect prompts
- **Difficulty Selection** (#14) — pre-mission difficulty picker
- **File Viewer** (#7) — generic file display

### Settings & Info
- **Game Options** (#21) — settings screen
- **Keys Display** (#16) — shows current key bindings
- **Key Assignment** (#18, #19) — two variants for key rebinding (in-game vs. post-game context)
- **Help Window** (#29) — help/tutorial overlay
- **Save/Load** (#23) — save and load game

### Misc
- **Satcom/Communications** (#1) — in-game radio/comms interface
- **Anim Sequence** (#11) — cutscene/animation playback (intro, briefings)
