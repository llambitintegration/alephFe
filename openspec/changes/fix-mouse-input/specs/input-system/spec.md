## MODIFIED Requirements

### Requirement: Translate gameplay input to Marathon action flags
The system SHALL translate the active gameplay bindings into Marathon action flags consumed by marathon-sim. The action flags SHALL include: move forward, move backward, strafe left, strafe right, turn left, turn right, look up, look down, fire primary weapon, fire secondary weapon, action (use), cycle weapons forward, cycle weapons backward, toggle map, and microphone. Additionally, the system SHALL pass proportional mouse yaw and pitch deltas (in radians) to the sim alongside the action flags.

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
