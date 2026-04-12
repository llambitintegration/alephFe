## Why

Mouse look in the web build is jerky and too fast. The current input pipeline converts accumulated mouse deltas into binary ActionFlags (TURN_LEFT/TURN_RIGHT, LOOK_UP/LOOK_DOWN) using a threshold, so the sim only ever sees "turning" or "not turning" — regardless of how fast or slow the mouse moves. This makes the game unplayable with a mouse, which is the only input method in the browser.

## What Changes

- Replace binary mouse-to-ActionFlags conversion with proportional mouse delta input that passes floating-point yaw/pitch deltas to the sim
- Add a `MouseDelta` resource (or extend `TickInput`) to carry per-tick yaw/pitch values alongside ActionFlags
- Update `compute_facing()` and `compute_vertical_look()` to accept and apply proportional mouse deltas when present, bypassing the acceleration/deceleration curve for mouse-driven rotation
- Tune mouse sensitivity scaling for natural feel at 30 ticks/second
- Keyboard turning (arrow keys / ActionFlags) continues to work through the existing angular velocity system

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities

- `input-system`: Mouse input now produces proportional yaw/pitch deltas instead of binary turn flags
- `player-physics`: Facing and vertical look computation accepts proportional mouse deltas alongside ActionFlags

## Impact

- `marathon-web/src/render.rs` — Input accumulation and `to_action_flags()` conversion
- `marathon-sim/src/tick.rs` — `TickInput` resource, `run_player_physics()` call site
- `marathon-sim/src/player/movement.rs` — `compute_facing()`, `compute_vertical_look()` signatures and logic
- `marathon-sim/src/components.rs` — Possible new `MouseDelta` component or `TickInput` extension
- No breaking API changes — ActionFlags remain for keyboard input; mouse delta is additive
