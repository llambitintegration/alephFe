## 1. Expand Light Component and Enums

- [x] 1.1 Add `LightState` enum with 6 variants: `BecomingActive`, `PrimaryActive`, `SecondaryActive`, `BecomingInactive`, `PrimaryInactive`, `SecondaryInactive` to `marathon-sim/src/components.rs`
- [x] 1.2 Add `LightType` enum with 3 variants: `Normal`, `Strobe`, `Media` to `marathon-sim/src/components.rs`
- [x] 1.3 Add `LightFunctionSpec` struct with fields: `function: LightFunction`, `period: u16`, `delta_period: u16`, `intensity: f32`, `delta_intensity: f32` to `marathon-sim/src/components.rs`
- [x] 1.4 Add `Random` and `Fluorescent` variants to `LightFunction` enum
- [x] 1.5 Replace the flat `Light` struct fields with state machine fields: `light_index`, `light_type: LightType`, `state: LightState`, `flags: u16`, `phase: u32`, `period: u32`, `current_intensity: f32`, `initial_intensity: f32`, `final_intensity: f32`, `functions: [LightFunctionSpec; 6]`, `tag: i16`
- [x] 1.6 Add constants for light flags: `LIGHT_IS_INITIALLY_ACTIVE: u16 = 0x0001`, `LIGHT_HAS_SLAVED_INTENSITIES: u16 = 0x0002`, `LIGHT_IS_STATELESS: u16 = 0x0004`
- [x] 1.7 Add `next_state()` method on `LightState` that returns the next state in the cycle
- [x] 1.8 Update existing tests in `components.rs` for new `LightFunction` variants

## 2. Implement Lighting Functions

- [x] 2.1 Refactor `compute_light_intensity()` in `marathon-sim/src/world_mechanics/lights.rs` to accept `initial_intensity: f32`, `final_intensity: f32`, `phase: u32`, `period: u32`, `function: LightFunction`, and `rng`
- [x] 2.2 Update `Constant` function: return `final_intensity`
- [x] 2.3 Update `Linear` function: `initial + (final - initial) * phase / period`
- [x] 2.4 Update `Smooth` function: `initial + (final - initial) * (cos(phase * PI / period + PI) + 1) / 2`
- [x] 2.5 Correct `Flicker` function to Alephone semantics: compute smooth base, add `rng * (final - smooth_value)`
- [x] 2.6 Implement `Random` function: `initial + rng * (final - initial)`
- [x] 2.7 Implement `Fluorescent` function: `if rng > 0.5 { final } else { initial }`
- [x] 2.8 Add unit tests for all 6 functions with known initial/final values
- [x] 2.9 Add unit test: constant returns final_intensity regardless of phase
- [x] 2.10 Add unit test: linear at phase=0 returns initial, at phase=period returns final
- [x] 2.11 Add unit test: smooth at phase=0 returns initial, at phase=period returns final
- [x] 2.12 Add unit test: flicker values stay within [initial, final] range over 100 ticks
- [x] 2.13 Add unit test: random values stay within [initial, final] range over 100 ticks
- [x] 2.14 Add unit test: fluorescent returns only initial or final (no intermediate values)

## 3. Implement State Machine Transitions

- [x] 3.1 Add `advance_light_state()` function in `lights.rs` that transitions a `Light` to its next state: sets `initial_intensity = current_intensity`, computes `final_intensity = spec.intensity + rng * spec.delta_intensity`, computes `period = spec.period + rng % (spec.delta_period + 1)`, resets `phase = 0`
- [x] 3.2 Handle slaved_intensities flag: when set, secondary_active uses primary_active's intensity values, secondary_inactive uses primary_inactive's intensity values
- [x] 3.3 Add `update_single_light()` function that increments phase, checks `phase >= period`, calls `advance_light_state()` on transition, then evaluates the current function to set `current_intensity`
- [x] 3.4 Add unit test: light transitions from becoming_active to primary_active when phase reaches period
- [x] 3.5 Add unit test: full 6-state cycle returns to becoming_active
- [x] 3.6 Add unit test: delta_period produces varied periods across transitions (seeded RNG)
- [x] 3.7 Add unit test: delta_intensity produces varied final intensities across transitions
- [x] 3.8 Add unit test: initial_intensity is set to previous current_intensity on transition

## 4. Integrate Light Updates into Tick Loop

- [x] 4.1 Add `run_light_updates()` method on `SimWorld` in `tick.rs` that queries all `Light` entities and calls `update_single_light()` for each
- [x] 4.2 Call `run_light_updates()` in `SimWorld::tick()` after player physics, in the world mechanics phase
- [ ] 4.3 Add `run_media_updates()` method on `SimWorld` that queries all `Media` entities, looks up the corresponding `Light` by `light_index`, and calls `compute_media_height()` to update `current_height`
- [ ] 4.4 Call `run_media_updates()` in `SimWorld::tick()` after `run_light_updates()`
- [x] 4.5 Add integration test: construct a SimWorld with a smooth-cycling light, tick 60 times, verify `current_intensity` changes over ticks
- [x] 4.6 Add integration test: construct a SimWorld with a media entity linked to a cycling light, tick 60 times, verify `current_height` changes

## 5. Update spawn_lights() for Full State Machine

- [x] 5.1 Rewrite `spawn_lights()` in `world.rs` to populate all 6 `LightFunctionSpec` entries from `StaticLightData`
- [x] 5.2 Map `StaticLightData.light_type` to `LightType` enum (0=Normal, 1=Strobe, 2=Media)
- [x] 5.3 Map each `LightingFunctionSpec.function` (0-5) to `LightFunction` enum including new `Random` (4) and `Fluorescent` (5) variants
- [x] 5.4 Set initial state based on `_light_is_initially_active` flag: if set start in `BecomingActive`, otherwise `BecomingInactive`
- [x] 5.5 Set initial phase from `StaticLightData.phase`
- [x] 5.6 Compute initial period and final_intensity for the starting state with delta randomization
- [x] 5.7 Set initial `initial_intensity` to 0.0 for becoming_active or 1.0 for becoming_inactive (matching Alephone defaults)
- [x] 5.8 Store `StaticLightData.tag` on the Light component
- [x] 5.9 Store `StaticLightData.flags` on the Light component

## 6. Expose Light Data for Renderers

- [ ] 6.1 Add `light_intensities(&mut self) -> Vec<f32>` method on `SimWorld` that returns a Vec indexed by `light_index` with each light's `current_intensity`
- [ ] 6.2 Add `media_heights(&mut self) -> Vec<f32>` method on `SimWorld` that returns a Vec indexed by media index with each media's `current_height`

## 7. Update Renderer GPU Buffer Writes

- [ ] 7.1 In `marathon-web/src/render.rs`, update the frame loop to call `sim.light_intensities()` and rebuild `PolygonInfo.floor_light` from live light values each frame before writing to the GPU buffer
- [ ] 7.2 In `marathon-game/src/mesh.rs` (or render loop), update floor_light values from sim light intensities each frame
- [ ] 7.3 In `marathon-viewer/src/level.rs` (or render loop), update floor_light values from sim light intensities each frame
- [ ] 7.4 Update media height in polygon buffer from sim media heights each frame (all three renderers)

## 8. Update Serialization

- [ ] 8.1 Update `SimSnapshot` in `world.rs` to work with the new `Light` struct shape (ensure Serialize/Deserialize derives still work)
- [ ] 8.2 Verify `snapshot()` and `deserialize()` round-trip correctly with new Light fields
- [ ] 8.3 Add unit test: serialize and deserialize a SimWorld with state-machine lights, verify light state is preserved

## 9. Testing and Validation

- [ ] 9.1 Run full `cargo test` suite in Docker, verify all existing tests pass with updated Light struct
- [ ] 9.2 Run marathon-viewer on a Marathon 2 level with known animated lights, verify visual animation
- [ ] 9.3 Deploy marathon-web and verify flickering/pulsing lights visible in-browser
- [ ] 9.4 Verify media (water/lava) surfaces rise and fall with their associated light animation
