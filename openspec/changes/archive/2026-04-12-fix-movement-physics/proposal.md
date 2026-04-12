## Why

Player movement feels sluggish and imprecise compared to the original Aleph One engine. Four compounding issues have been identified: the sim always uses walking physics (never running), the velocity model uses isotropic 2D vectors instead of Marathon's axis-decomposed scalars, angular physics constants are in Marathon angle units but treated as radians, and mouse input accumulates a full tick of latency before affecting the camera.

## What Changes

- **Default to running physics**: Load physics constants index 1 (running) instead of index 0 (walking), or implement run/walk toggle. Running has ~2x velocity and acceleration values.
- **Axis-decomposed velocity model**: Replace the Vec2 velocity model with separate forward and perpendicular velocity scalars matching Aleph One's `physics.cpp`. Add direction-reversal boost (acceleration + deceleration applied simultaneously when moving against current velocity).
- **Angular unit conversion**: Convert angular physics constants from Marathon angle units (512 = full circle) to radians when loading into `PlayerPhysicsParams`. Conversion factor: `value * (2π / 512)`.
- **Reduce mouse input latency**: Apply mouse yaw/pitch deltas to the camera within the render frame rather than waiting for the next simulation tick, eliminating ~33ms of input lag at 30Hz.

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities

- `player-physics`: Velocity model changes from Vec2 to axis-decomposed scalars with direction-reversal boost. Angular unit conversion applied at load time. Running physics used by default.
- `input-system`: Mouse delta applied to camera in render frame for reduced latency. Mouse deltas still passed to sim for authoritative state, but camera preview reduces perceived lag.

## Impact

- `marathon-sim/src/player/movement.rs` — Rewrite `compute_player_velocity()` to use axis-decomposed forward/perpendicular scalars with reversal boost. Update `PlayerPhysicsParams::from_physics_constants()` to convert angular units.
- `marathon-sim/src/world.rs` — Load running physics (index 1) or implement walk/run toggle.
- `marathon-sim/src/components.rs` — Replace `Velocity(Vec3)` with separate forward/perpendicular velocity components if needed, or keep internal to movement calculation.
- `marathon-web/src/render.rs` — Apply mouse delta to camera interpolation target before tick, reducing perceived input latency.
- `marathon-game/src/render.rs` — Same mouse latency fix for native build (also fix the existing bug where mouse deltas are converted to binary flags instead of passed as floats).
- `marathon-sim/src/tick.rs` — No structural changes; TickInput already carries mouse_yaw/mouse_pitch.
- All existing tests in `marathon-sim/src/player/movement.rs` need updating for new velocity model and unit conversions.
