## Why

The marathon-web build has two input mapping bugs that make the game unplayable in the browser. WASD keys are mapped to the opposite movement directions (W moves backward, S moves forward, A strafes right, D strafes left), and the mouse Y-axis is inverted (moving the mouse up pitches the camera down). The desktop build in marathon-game has the correct mappings for both. These are straightforward wiring errors in the web-specific input handlers, not design issues.

## What Changes

- Fix WASD key mappings in the web keydown/keyup event handlers: swap W/S (forward/backward) and A/D (strafe_left/strafe_right) to match the desktop build and standard FPS conventions
- Negate `mouse_dy` when applying pitch in the web render loop's camera-immediate mouse application, matching the desktop build's sign convention where negative screen-Y delta (mouse moved up) produces positive pitch (look up)

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities

- `input-system`: The web build's keyboard event handlers SHALL map W/ArrowUp to `forward` and S/ArrowDown to `backward`, and SHALL map A to `strafe_left` and D to `strafe_right`, consistent with the desktop build's bindings
- `browser-interaction-tests`: The scenario "WHEN the canvas has focus and the user presses W / THEN forward=true" SHALL hold true after this fix (currently W incorrectly sets backward=true, so any e2e test asserting forward movement from W would fail)

## Impact

- `marathon-web/src/render.rs` lines 640-644, 658-661 -- keydown and keyup match arms for WASD need field names swapped
- `marathon-web/src/render.rs` line 206 -- camera pitch calculation needs `mouse_dy` negated to match desktop convention at `marathon-game/src/render.rs:1318`
- No API changes, no new dependencies, no changes to marathon-sim or marathon-game
