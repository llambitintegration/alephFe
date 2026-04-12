## 1. Extend Sim Input Model

- [x] 1.1 Add `mouse_yaw: f32` and `mouse_pitch: f32` fields to `TickInput` in marathon-sim/src/tick.rs
- [x] 1.2 Update `SimWorld::tick()` signature or `TickInput` construction to accept mouse deltas alongside ActionFlags
- [x] 1.3 Add unit test: construct TickInput with mouse_yaw=0.1, verify it round-trips through resource insertion

## 2. Update Player Physics for Mouse Deltas

- [x] 2.1 Modify `compute_facing()` in movement.rs to accept an optional mouse_yaw parameter; when non-zero, add it directly to facing before applying angular velocity from ActionFlags
- [x] 2.2 Modify `compute_vertical_look()` in movement.rs to accept an optional mouse_pitch parameter; when non-zero, add it directly to vertical_look (clamped to elevation limits)
- [x] 2.3 Update `run_player_physics()` in tick.rs to read mouse_yaw/mouse_pitch from TickInput and pass to compute functions
- [x] 2.4 Add unit test: tick with mouse_yaw=0.1 and no ActionFlags changes facing by exactly 0.1 radians
- [x] 2.5 Add unit test: tick with mouse_yaw=0.1 AND TURN_RIGHT flag changes facing by 0.1 + angular_acceleration
- [x] 2.6 Add unit test: tick with mouse_pitch=-0.05 changes vertical_look by -0.05, clamped to limits

## 3. Update Web Input Pipeline

- [x] 3.1 Remove binary threshold conversion in `to_action_flags()` — mouse_dx/mouse_dy no longer set TURN/LOOK flags
- [x] 3.2 Add `to_mouse_delta() -> (f32, f32)` method on InputState that returns accumulated (yaw, pitch) in radians using sensitivity constant, then resets accumulators
- [x] 3.3 Change sensitivity constant from 0.15 (threshold scaling) to ~0.003 (radians-per-pixel)
- [x] 3.4 Update render loop tick call to pass mouse deltas to sim via TickInput
- [x] 3.5 Verify keyboard-only turning still works (TURN_LEFT/TURN_RIGHT flags from arrow keys, no mouse delta)

## 4. Testing

- [x] 4.1 Run full cargo test suite in Docker and verify all existing + new tests pass
- [ ] 4.2 Deploy to marathon.llambit.io and verify smooth mouse look at various speeds
- [ ] 4.3 Verify keyboard arrow-key turning still functions independently of mouse
