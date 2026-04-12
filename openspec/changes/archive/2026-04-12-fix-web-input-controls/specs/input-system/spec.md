## MODIFIED Requirements

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
