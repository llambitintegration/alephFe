## 1. Angular Unit Conversion

- [x] 1.1 Add angular unit conversion in `PlayerPhysicsParams::from_physics_constants()` — multiply `angular_acceleration`, `angular_deceleration`, `max_angular_velocity`, `maximum_elevation`, and other angular fields by `2*PI/512` to convert from Marathon angle units to radians
- [x] 1.2 Update unit tests in `movement.rs` to use radian-scale values in `test_params()` (current test values are arbitrary, so just ensure they're realistic for radians)

## 2. Running Physics Default

- [x] 2.1 Change `world.rs` physics loading from `p.first()` to `p.get(1).or_else(|| p.first())` to prefer running physics (index 1) with walking fallback
- [x] 2.2 Add a test that verifies running physics is preferred when two entries exist

## 3. Axis-Decomposed Velocity Model

- [x] 3.1 Rewrite `compute_player_velocity()` to accept and return separate forward/perpendicular/vertical velocity scalars instead of Vec3. Track forward and perpendicular velocity independently.
- [x] 3.2 Implement direction-reversal boost: when input opposes current velocity direction on an axis, apply `acceleration + deceleration` combined
- [x] 3.3 Implement independent axis deceleration: decelerate forward and perpendicular axes separately (stopping strafe must not affect forward speed)
- [x] 3.4 Update `run_player_physics()` in `tick.rs` to use the new velocity API — decompose velocity into forward/perpendicular before the call, recompose into Vec3 for position update
- [x] 3.5 Update or replace `Velocity` component if needed — may keep Vec3 for collision but derive from axis scalars each tick
- [x] 3.6 Update all velocity-related tests: `forward_movement_accelerates`, `no_input_decelerates`, `gravity_when_airborne`, `no_gravity_when_grounded`, and collision tests

## 4. Mouse Input Latency Reduction

- [x] 4.1 In `marathon-web/src/render.rs`, apply mouse delta directly to camera yaw/pitch in the `frame()` method before tick processing, then sync camera to sim state after tick completes
- [x] 4.2 In `marathon-game/src/render.rs`, fix the mouse-to-binary-flag bug: pass actual mouse deltas as `TickInput.mouse_yaw`/`TickInput.mouse_pitch` instead of converting to TURN_LEFT/TURN_RIGHT flags
- [x] 4.3 In `marathon-game/src/render.rs`, apply same camera-immediate mouse pattern as web build

## 5. Integration Testing

- [x] 5.1 Build and deploy marathon-web via Docker to verify movement feel in browser
- [x] 5.2 Run existing `marathon-sim` test suite to verify no regressions in collision, gravity, and media physics
- [x] 5.3 Verify keyboard turning speed is reasonable after angular unit conversion (should be noticeably slower than before, matching Aleph One's feel)
