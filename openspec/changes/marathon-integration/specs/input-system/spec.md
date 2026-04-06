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
The system SHALL translate the active gameplay bindings into Marathon action flags consumed by marathon-sim. The action flags SHALL include: move forward, move backward, strafe left, strafe right, turn left, turn right, look up, look down, fire primary weapon, fire secondary weapon, action (use), cycle weapons forward, cycle weapons backward, toggle map, and microphone.

#### Scenario: Forward movement key held
- **WHEN** the forward movement key is held in Gameplay context
- **THEN** the `MoveForward` action flag SHALL be set for the current tick

#### Scenario: Mouse turn
- **WHEN** mouse delta X is positive in Gameplay context
- **THEN** the `TurnRight` action flag SHALL be set with magnitude proportional to the delta multiplied by the mouse sensitivity setting

#### Scenario: No input
- **WHEN** no keys are pressed and no mouse/gamepad input is received in Gameplay context
- **THEN** all action flags SHALL be cleared (zero)

#### Scenario: Simultaneous opposing inputs
- **WHEN** both forward and backward movement keys are held simultaneously
- **THEN** both `MoveForward` and `MoveBackward` action flags SHALL be set (marathon-sim resolves the conflict)

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
