## MODIFIED Requirements

### Requirement: Capture raw input events from winit
In the web build, the input system SHALL register event listeners on the canvas element for keyboard, mouse movement, and mouse button events. The canvas SHALL have tabindex="0" and receive focus after loading. Pointer lock SHALL be requested on canvas click to enable relative mouse movement for look controls.

#### Scenario: Pointer lock engages on click
- **WHEN** the user clicks on the marathon canvas
- **THEN** the browser SHALL enter pointer lock mode and subsequent mousemove events SHALL provide movementX/movementY deltas

#### Scenario: Keyboard events reach the game
- **WHEN** the canvas has focus and the user presses W
- **THEN** the input state SHALL set forward=true and the next tick's ActionFlags SHALL contain MOVE_FORWARD

#### Scenario: Mouse movement translates to look
- **WHEN** pointer lock is active and the user moves the mouse right
- **THEN** the input state SHALL accumulate positive mouse_dx and the next tick's ActionFlags SHALL contain LOOK_RIGHT
