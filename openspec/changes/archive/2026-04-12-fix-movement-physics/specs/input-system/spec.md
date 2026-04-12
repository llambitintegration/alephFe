## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: Camera-immediate mouse application
The rendering layer SHALL apply mouse yaw and pitch deltas directly to the camera orientation within the current render frame, before the next simulation tick processes the input. The simulation SHALL remain authoritative — on each tick completion, the camera orientation SHALL be updated to match the simulation's facing and vertical look values.

#### Scenario: Mouse movement applied before next tick
- **WHEN** the player moves the mouse between simulation ticks
- **THEN** the rendered camera orientation SHALL reflect the mouse delta immediately in the current frame, without waiting for the next simulation tick

#### Scenario: Camera syncs to sim on tick
- **WHEN** a simulation tick completes and updates the player's facing angle
- **THEN** the camera orientation SHALL be set to the simulation's authoritative facing and vertical look values, replacing any preview offset

#### Scenario: No visible jitter from preview
- **WHEN** the camera previews mouse movement and the sim tick confirms it
- **THEN** the transition SHALL be seamless because the sim applies the same delta the camera previewed

### Requirement: Native build passes mouse deltas as floats
The marathon-game (native winit) build SHALL pass proportional mouse yaw and pitch deltas to the simulation via `TickInput.mouse_yaw` and `TickInput.mouse_pitch` fields. Mouse deltas SHALL NOT be converted to binary `TURN_LEFT`/`TURN_RIGHT` or `LOOK_UP`/`LOOK_DOWN` action flags.

#### Scenario: Native mouse yaw passed as float
- **WHEN** the native build receives a mouse motion event with delta_x = 50 pixels
- **THEN** `TickInput.mouse_yaw` SHALL be set to `50.0 * sensitivity` in radians, and `TURN_LEFT`/`TURN_RIGHT` flags SHALL NOT be set from mouse input

#### Scenario: Native mouse pitch passed as float
- **WHEN** the native build receives a mouse motion event with delta_y = -30 pixels
- **THEN** `TickInput.mouse_pitch` SHALL be set to `-30.0 * sensitivity` in radians, and `LOOK_UP`/`LOOK_DOWN` flags SHALL NOT be set from mouse input
