---
tags: [input, controls, configuration, ui]
status: partially-implemented
---

# Control Remapping

How alephone handles input configuration and the current state of input handling in the Rust rebuild.

## How the Original Alephone Handles It

### Input System Architecture

Alephone's input system (in `Source_Files/Misc/preferences.cpp` and related files) provides:

1. **Multiple input devices**: Keyboard, mouse, joystick/gamepad
2. **Configurable bindings**: Most key bindings customizable through Preferences > Controls > Configure Keys / Buttons
3. **Mouse settings**: Sensitivity (horizontal and vertical independent), invert look, acceleration
4. **Joystick settings**: Sensitivity, dead zones, axis mapping
5. **Behavioral modifiers**: Auto-run, auto-recenter view, auto-switch weapons
6. **Persistence**: Settings stored in XML preferences file (`Aleph One Preferences`)

### Default Control Scheme

Alephone's default Marathon controls (modern WASD layout added by Alephone, original used arrow keys):

| Action | Primary Key | Secondary |
|--------|-------------|-----------|
| Move Forward | W | Up Arrow |
| Move Backward | S | Down Arrow |
| Turn Left | Left Arrow | -- |
| Turn Right | Right Arrow | -- |
| Strafe Left | A | -- |
| Strafe Right | D | -- |
| Look Up | -- | (mouse) |
| Look Down | -- | (mouse) |
| Fire Primary | Left Mouse | -- |
| Fire Secondary | Right Mouse | -- |
| Action (use) | Space | -- |
| Cycle Weapons | Tab | -- |
| Toggle Map | M | -- |
| Microphone | Backtick | -- |

### Preferences File

Alephone stores preferences in an XML file with sections for:
- `<input>` -- mouse sensitivity, inversion flags
- `<keyboard>` -- key binding array (action index -> SDL key code)
- `<joystick>` -- axis mapping, sensitivity, dead zones
- `<general>` -- auto-run, auto-recenter, etc.

### Rebinding UI

The alephone preferences dialog provides a "Configure Keys / Buttons" screen where:
1. User selects an action from a list
2. User presses the desired key/button
3. The binding is recorded and displayed
4. Conflicts (same key bound to two actions) are warned about
5. "Restore Defaults" resets all bindings

## Current State in Rust Rebuild

The input system is **partially implemented** with a solid foundation.

### Input Pipeline

The input flows through these layers:

```
Physical Input (winit events)
       |
       v
RawInput events (KeyPress, MouseDelta, GamepadAxis, etc.)
       |
       v
InputBuffer (collects per-frame events)
       |
       v
translate_gameplay_input() / translate_menu_input() / translate_terminal_input()
       |
       v
ActionFlags (gameplay) or MenuAction/TerminalAction (UI)
       |
       v
SimWorld::tick(TickInput) or UI state machine
```

### Key Types

**`RawInput`** (`marathon-integration/src/input/mod.rs`):
```rust
pub enum RawInput {
    KeyPress(KeyCode),
    KeyRelease(KeyCode),
    MouseDelta(f64, f64),
    MouseButtonPress(MouseButton),
    MouseButtonRelease(MouseButton),
    GamepadAxis(GamepadAxisType, f32),
    GamepadButtonPress(GamepadButtonType),
    GamepadButtonRelease(GamepadButtonType),
}
```

**`KeyCode`** -- Subset of keyboard keys covering Marathon-relevant inputs (A-Z, 0-9, arrows, function keys, modifiers).

**`MouseButton`** -- Left, Right, Middle, Button4, Button5.

**`GamepadAxisType`** -- LeftStickX/Y, RightStickX/Y, LeftTrigger, RightTrigger.

**`GamepadButtonType`** -- Standard gamepad buttons (South/East/North/West, bumpers, D-pad, etc.).

### Input Contexts

Three input contexts exist (`marathon-integration/src/input/context.rs`), selected based on `GameState`:

| GameState | InputContext |
|-----------|-------------|
| `Playing` | `Gameplay` |
| `Terminal` | `Terminal` |
| `MainMenu`, `Paused`, `GameOver` | `Menu` |
| `Loading`, `Intermission` | `Menu` |

### Key Bindings

The `KeyBindings` struct (`marathon-integration/src/input/bindings.rs`) maps physical inputs to actions:

```rust
pub struct KeyBindings {
    pub gameplay: HashMap<PhysicalInput, GameplayAction>,
    pub menu: HashMap<PhysicalInput, MenuAction>,
    pub terminal: HashMap<PhysicalInput, TerminalAction>,
}
```

Where `PhysicalInput` is:
```rust
pub enum PhysicalInput {
    Key(KeyCode),
    Mouse(MouseButton),
}
```

Default bindings are provided via `KeyBindings::default()`:

| PhysicalInput | GameplayAction |
|---------------|----------------|
| `Key(W)` / `Key(Up)` | `MoveForward` |
| `Key(S)` / `Key(Down)` | `MoveBackward` |
| `Key(A)` | `StrafeLeft` |
| `Key(D)` | `StrafeRight` |
| `Key(Left)` | `TurnLeft` |
| `Key(Right)` | `TurnRight` |
| `Mouse(Left)` | `FirePrimary` |
| `Mouse(Right)` | `FireSecondary` |
| `Key(Space)` | `Action` |
| `Key(Tab)` | `CycleWeaponForward` |
| `Key(M)` | `ToggleMap` |
| `Key(Backtick)` | `Microphone` |

### GameplayAction to ActionFlags

Each `GameplayAction` maps to exactly one `ActionFlags` bit via `to_flag()`:

```rust
impl GameplayAction {
    pub fn to_flag(self) -> ActionFlags {
        match self {
            Self::MoveForward => ActionFlags::MOVE_FORWARD,
            Self::FirePrimary => ActionFlags::FIRE_PRIMARY,
            // ... etc
        }
    }
}
```

### Input Configuration

`InputConfig` (`marathon-integration/src/input/mod.rs`):
```rust
pub struct InputConfig {
    pub mouse_sensitivity: f64,    // default: 1.0
    pub gamepad_dead_zone: f32,    // default: 0.15
}
```

Used during input translation:
- Mouse delta is scaled by `mouse_sensitivity` before determining turn direction
- Gamepad axes below `dead_zone` threshold are zeroed; values above are remapped from `[dead_zone, 1.0]` to `[0.0, 1.0]`

### Preferences System

`Preferences` (`marathon-integration/src/menu/preferences.rs`):
```rust
pub struct Preferences {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub resolution_width: u32,
    pub resolution_height: u32,
    pub fullscreen: bool,
    pub input: InputConfig,
}
```

The preferences screen supports three categories (`PreferencesCategory`):
- **Controls**: Mouse sensitivity slider (0.1-5.0), gamepad dead zone slider (0.05-0.50)
- **Audio**: Master/Music/SFX volume sliders
- **Video**: Resolution choice, fullscreen toggle

Preferences can be adjusted via `apply_preference()` which modifies the `Preferences` struct.

## What Needs Implementation

### Key Rebinding UI

The most significant missing piece. Need a screen where users can:
1. See a list of all gameplay actions with their current binding
2. Select an action to rebind
3. Enter "listening" mode -- next physical input becomes the new binding
4. Handle conflicts (warn if key already bound to another action)
5. Reset to defaults

**Proposed approach**:
```rust
pub enum RebindState {
    /// Browsing the list of bindings
    Browsing { selected_action: usize },
    /// Waiting for the user to press a key/button
    Listening { target_action: GameplayAction },
}
```

### Multiple Bindings per Action

Currently each `PhysicalInput` maps to one action, but there's no support for binding multiple keys to the same action (e.g., both W and Up Arrow for forward). The `HashMap<PhysicalInput, GameplayAction>` structure supports this naturally since different keys can map to the same action, but the UI needs to display and manage this.

### Gamepad Button Bindings

`GamepadButtonType` is defined but not included in `PhysicalInput` yet. Gamepad buttons should be bindable to gameplay actions:

```rust
pub enum PhysicalInput {
    Key(KeyCode),
    Mouse(MouseButton),
    GamepadButton(GamepadButtonType),  // NEW
}
```

### Analog Input for Movement

Currently, gamepad analog input is translated to binary action flags (left stick X > 0 = STRAFE_RIGHT). For smoother movement, the sim should accept analog values:

```rust
pub struct TickInput {
    pub action_flags: ActionFlags,
    pub mouse_yaw: f32,
    pub mouse_pitch: f32,
    pub move_analog: Vec2,    // NEW: left stick (-1..1, -1..1)
    pub look_analog: Vec2,    // NEW: right stick
}
```

### Persistence (Save/Load Config)

Key bindings and input config need to be persisted to disk. Options:
- **JSON/TOML file**: Human-readable, easy to edit manually
- **bincode**: Compact but opaque
- **RON (Rust Object Notation)**: Rust-native, human-readable

Recommended: **TOML** for config files, matching Rust ecosystem conventions.

```toml
# marathon-config.toml
[input]
mouse_sensitivity = 1.5
gamepad_dead_zone = 0.15

[input.gameplay_bindings]
move_forward = ["W", "Up"]
move_backward = ["S", "Down"]
fire_primary = ["MouseLeft"]
# ...

[audio]
master_volume = 1.0
music_volume = 0.8
sfx_volume = 1.0

[video]
resolution = "1920x1080"
fullscreen = false
```

### Mouse Look Improvements

The current mouse handling translates delta to binary TURN_LEFT/TURN_RIGHT flags. The `TickInput.mouse_yaw` and `mouse_pitch` fields exist for continuous mouse look but need:
- Proper sensitivity curve (linear, accelerated, or custom)
- Y-axis inversion option
- Raw mouse input support (bypass OS acceleration)

### Web Input Handling

For `marathon-web`, input comes from browser events (KeyboardEvent, PointerEvent, Gamepad API) instead of winit. The input abstraction layer (`RawInput`) should work, but the translation from browser events to `RawInput` needs implementation.

## Key Files

- `marathon-integration/src/input/mod.rs` -- RawInput, KeyCode, InputBuffer, InputConfig
- `marathon-integration/src/input/bindings.rs` -- KeyBindings, PhysicalInput, GameplayAction
- `marathon-integration/src/input/action_flags.rs` -- translate_gameplay_input(), dead zone math
- `marathon-integration/src/input/context.rs` -- InputContext from GameState
- `marathon-integration/src/menu/preferences.rs` -- Preferences, preference UI items
- `marathon-integration/src/types.rs` -- ActionFlags bitflags

## See Also

- [[alephone-network-architecture]] -- Action flags are the network protocol unit
- [[film-replay-system]] -- Film records action flags (output of input translation)
- [[game-mode-implementations]] -- Input context affects available actions
- [Alephone Keyboard Shortcuts](https://github.com/Aleph-One-Marathon/alephone/wiki/Keyboard-Shortcuts)
