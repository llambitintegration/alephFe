## ADDED Requirements

### Requirement: Capture raw input events from winit
The system SHALL receive keyboard, mouse, and gamepad events from the winit event loop. The system SHALL normalize these events into an internal `RawInput` representation that is independent of the windowing backend. Keyboard events SHALL include key code and press/release state. Mouse events SHALL include axis deltas and button press/release state. Gamepad events SHALL include axis values and button press/release state.

#### Scenario: Keyboard key press
- **WHEN** winit emits a `KeyboardInput` event with key code `W` and state `Pressed`
- **THEN** the system SHALL record a `RawInput::KeyPress(W)` for the current frame

#### Scenario: Mouse movement
- **WHEN** winit emits a `DeviceEvent::MouseMotion` with delta (dx, dy)
- **THEN** the system SHALL record a `RawInput::MouseDelta(dx, dy)` for the current frame

#### Scenario: Gamepad stick movement
- **WHEN** winit emits a gamepad axis event for the left stick with value 0.75 on the X axis
- **THEN** the system SHALL record a `RawInput::GamepadAxis(LeftStickX, 0.75)` for the current frame

### Requirement: Context-dependent input mapping
The system SHALL maintain separate input binding maps for each input context: `Gameplay`, `Menu`, and `Terminal`. The active input context SHALL be determined by the current game state. Only the bindings for the active context SHALL be evaluated each frame.

#### Scenario: Gameplay context active during Playing state
- **WHEN** the game state is `Playing`
- **THEN** the active input context SHALL be `Gameplay` and only gameplay bindings SHALL be evaluated

#### Scenario: Menu context active during MainMenu state
- **WHEN** the game state is `MainMenu`
- **THEN** the active input context SHALL be `Menu` and only menu bindings SHALL be evaluated

#### Scenario: Terminal context active during Terminal state
- **WHEN** the game state is `Terminal`
- **THEN** the active input context SHALL be `Terminal` and only terminal bindings SHALL be evaluated

#### Scenario: Context switch on state transition
- **WHEN** the game state transitions from `Playing` to `Paused`
- **THEN** the active input context SHALL change from `Gameplay` to `Menu`

### Requirement: Translate gameplay input to Marathon action flags
The system SHALL translate the active gameplay bindings into Marathon action flags consumed by marathon-sim. The action flags SHALL include: move forward, move backward, strafe left, strafe right, turn left, turn right, look up, look down, fire primary weapon, fire secondary weapon, action (use), cycle weapons forward, cycle weapons backward, toggle map, and microphone. Additionally, the system SHALL pass proportional mouse yaw and pitch deltas (in radians) to the sim alongside the action flags via the `TickInput` struct. Mouse deltas SHALL NOT be converted to binary turn/look flags.

#### Scenario: Forward movement key held
- **WHEN** the forward movement key is held in Gameplay context
- **THEN** the `MoveForward` action flag SHALL be set for the current tick

#### Scenario: Mouse turn
- **WHEN** mouse delta X is positive in Gameplay context
- **THEN** a proportional `mouse_yaw` value in radians SHALL be passed to the sim, computed as the accumulated pixel delta multiplied by the sensitivity constant. The `TurnRight` action flag SHALL NOT be set for mouse input.

#### Scenario: Mouse look vertical
- **WHEN** mouse delta Y is non-zero in Gameplay context
- **THEN** a proportional `mouse_pitch` value in radians SHALL be passed to the sim, computed as the accumulated pixel delta multiplied by the sensitivity constant. The `LOOK_UP`/`LOOK_DOWN` action flags SHALL NOT be set for mouse input.

#### Scenario: Keyboard turn
- **WHEN** the turn-right key is held in Gameplay context and no mouse yaw delta exists
- **THEN** the `TurnRight` action flag SHALL be set, and the sim SHALL apply angular acceleration as before

#### Scenario: No input
- **WHEN** no keys are pressed and no mouse/gamepad input is received in Gameplay context
- **THEN** all action flags SHALL be cleared (zero) and mouse yaw/pitch SHALL be 0.0

#### Scenario: Simultaneous opposing inputs
- **WHEN** both forward and backward movement keys are held simultaneously
- **THEN** both `MoveForward` and `MoveBackward` action flags SHALL be set (marathon-sim resolves the conflict)

#### Scenario: Mouse and keyboard turn simultaneously
- **WHEN** the turn-right key is held AND mouse delta X is positive
- **THEN** both the `TurnRight` action flag and a proportional `mouse_yaw` delta SHALL be passed to the sim, and both SHALL contribute to facing change

### Requirement: Configurable key bindings
The system SHALL support user-configurable key bindings for all input actions in all contexts. Bindings SHALL map one or more physical inputs (key, mouse button, gamepad button/axis) to a single logical action. The system SHALL provide a default binding set matching Marathon's original keyboard layout.

#### Scenario: Custom key binding
- **WHEN** the user binds the `F` key to `FirePrimary` in gameplay bindings
- **THEN** pressing `F` during gameplay SHALL set the `FirePrimary` action flag

#### Scenario: Multiple inputs bound to same action
- **WHEN** both `Space` key and mouse button 1 are bound to `FirePrimary`
- **THEN** pressing either input SHALL set the `FirePrimary` action flag

#### Scenario: Default bindings loaded
- **WHEN** no user binding configuration exists
- **THEN** the system SHALL use default bindings with arrow keys for movement, mouse for look, and Marathon-standard key assignments

### Requirement: Mouse sensitivity and dead zone configuration
The system SHALL apply a configurable sensitivity multiplier to mouse axis deltas before converting them to turn/look magnitudes. The system SHALL apply configurable dead zones to gamepad stick axes, treating values below the dead zone threshold as zero.

#### Scenario: Mouse sensitivity applied
- **WHEN** mouse sensitivity is set to 2.0 and a mouse delta of (10, 5) is received
- **THEN** the effective delta used for turn/look calculation SHALL be (20, 10)

#### Scenario: Gamepad dead zone filtering
- **WHEN** gamepad dead zone is set to 0.15 and left stick X reads 0.10
- **THEN** the effective stick value SHALL be 0.0 (below dead zone threshold)

#### Scenario: Gamepad value above dead zone
- **WHEN** gamepad dead zone is set to 0.15 and left stick X reads 0.50
- **THEN** the effective stick value SHALL be remapped from the dead zone range to a 0.0-1.0 range

### Requirement: Menu navigation from input
The system SHALL translate input events in the Menu context into menu navigation actions: `Up`, `Down`, `Left`, `Right`, `Select`, and `Back`. These actions SHALL be emitted as events consumable by the menu system.

#### Scenario: Arrow key menu navigation
- **WHEN** the down arrow key is pressed in Menu context
- **THEN** a `MenuAction::Down` event SHALL be emitted

#### Scenario: Enter key selects menu item
- **WHEN** the Enter key is pressed in Menu context
- **THEN** a `MenuAction::Select` event SHALL be emitted

#### Scenario: Escape key goes back
- **WHEN** the Escape key is pressed in Menu context
- **THEN** a `MenuAction::Back` event SHALL be emitted

### Requirement: Terminal input handling
The system SHALL translate input events in the Terminal context into terminal navigation actions: `ScrollUp`, `ScrollDown`, `PageUp`, `PageDown`, and `Exit`. These actions SHALL be emitted as events consumable by the terminal system.

#### Scenario: Arrow down scrolls terminal
- **WHEN** the down arrow key is pressed in Terminal context
- **THEN** a `TerminalAction::ScrollDown` event SHALL be emitted

#### Scenario: Escape exits terminal
- **WHEN** the Escape key is pressed in Terminal context
- **THEN** a `TerminalAction::Exit` event SHALL be emitted

#### Scenario: Page down advances terminal page
- **WHEN** the Page Down key is pressed in Terminal context
- **THEN** a `TerminalAction::PageDown` event SHALL be emitted

### Requirement: Camera-immediate mouse application
The rendering layer SHALL apply mouse yaw and pitch deltas directly to the camera orientation within the current render frame, before the next simulation tick processes the input. The simulation SHALL remain authoritative — on each tick completion, the camera orientation SHALL be updated to match the simulation's facing and vertical look values. In the web build, the pitch calculation SHALL negate `mouse_dy` so that negative screen-Y delta (mouse moved up) produces positive pitch (look up), consistent with the desktop build's sign convention.

#### Scenario: Mouse movement applied before next tick
- **WHEN** the player moves the mouse between simulation ticks
- **THEN** the rendered camera orientation SHALL reflect the mouse delta immediately in the current frame, without waiting for the next simulation tick

#### Scenario: Camera syncs to sim on tick
- **WHEN** a simulation tick completes and updates the player's facing angle
- **THEN** the camera orientation SHALL be set to the simulation's authoritative facing and vertical look values, replacing any preview offset

#### Scenario: No visible jitter from preview
- **WHEN** the camera previews mouse movement and the sim tick confirms it
- **THEN** the transition SHALL be seamless because the sim applies the same delta the camera previewed

#### Scenario: Web mouse-up pitches camera up
- **WHEN** the web build receives a mouse movement event with negative delta_y (mouse moved up on screen)
- **THEN** the camera pitch SHALL increase (look up), matching the desktop build's behavior where `-mouse_dy` is applied to pitch

#### Scenario: Web mouse-down pitches camera down
- **WHEN** the web build receives a mouse movement event with positive delta_y (mouse moved down on screen)
- **THEN** the camera pitch SHALL decrease (look down), matching the desktop build's behavior

### Requirement: Web build WASD key bindings
The web build's keyboard event handlers SHALL map WASD and arrow keys to movement actions consistent with the desktop build and standard FPS conventions. Specifically: W and ArrowUp SHALL map to `forward`, S and ArrowDown SHALL map to `backward`, A SHALL map to `strafe_left`, and D SHALL map to `strafe_right`. Both keydown (set true) and keyup (set false) handlers SHALL use the same correct mappings.

#### Scenario: W key sets forward
- **WHEN** the web build receives a keydown event with code "KeyW"
- **THEN** `input.forward` SHALL be set to true

#### Scenario: S key sets backward
- **WHEN** the web build receives a keydown event with code "KeyS"
- **THEN** `input.backward` SHALL be set to true

#### Scenario: A key sets strafe_left
- **WHEN** the web build receives a keydown event with code "KeyA"
- **THEN** `input.strafe_left` SHALL be set to true

#### Scenario: D key sets strafe_right
- **WHEN** the web build receives a keydown event with code "KeyD"
- **THEN** `input.strafe_right` SHALL be set to true

#### Scenario: ArrowUp key sets forward
- **WHEN** the web build receives a keydown event with code "ArrowUp"
- **THEN** `input.forward` SHALL be set to true

#### Scenario: ArrowDown key sets backward
- **WHEN** the web build receives a keydown event with code "ArrowDown"
- **THEN** `input.backward` SHALL be set to true

#### Scenario: W key release clears forward
- **WHEN** the web build receives a keyup event with code "KeyW"
- **THEN** `input.forward` SHALL be set to false

#### Scenario: S key release clears backward
- **WHEN** the web build receives a keyup event with code "KeyS"
- **THEN** `input.backward` SHALL be set to false

#### Scenario: A key release clears strafe_left
- **WHEN** the web build receives a keyup event with code "KeyA"
- **THEN** `input.strafe_left` SHALL be set to false

#### Scenario: D key release clears strafe_right
- **WHEN** the web build receives a keyup event with code "KeyD"
- **THEN** `input.strafe_right` SHALL be set to false

### Requirement: Native build passes mouse deltas as floats
The marathon-game (native winit) build SHALL pass proportional mouse yaw and pitch deltas to the simulation via `TickInput.mouse_yaw` and `TickInput.mouse_pitch` fields. Mouse deltas SHALL NOT be converted to binary `TURN_LEFT`/`TURN_RIGHT` or `LOOK_UP`/`LOOK_DOWN` action flags.

#### Scenario: Native mouse yaw passed as float
- **WHEN** the native build receives a mouse motion event with delta_x = 50 pixels
- **THEN** `TickInput.mouse_yaw` SHALL be set to `50.0 * sensitivity` in radians, and `TURN_LEFT`/`TURN_RIGHT` flags SHALL NOT be set from mouse input

#### Scenario: Native mouse pitch passed as float
- **WHEN** the native build receives a mouse motion event with delta_y = -30 pixels
- **THEN** `TickInput.mouse_pitch` SHALL be set to `-30.0 * sensitivity` in radians, and `LOOK_UP`/`LOOK_DOWN` flags SHALL NOT be set from mouse input
