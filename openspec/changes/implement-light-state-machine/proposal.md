## Why

Lights in the Rust sim are frozen at load time. The `Light` component stores only the `primary_active` function spec, `compute_light_intensity()` evaluates it as a pure function of the global tick counter, and nothing in `SimWorld::tick()` actually calls it -- `current_intensity` is set once during `spawn_lights()` and never updated. This means every light in every level is static: no flickering hallways, no pulsing alien machinery, no strobing alarms. It also blocks liquid surface animation, because media height is derived from its associated light's intensity (`height = low + (high - low) * light_intensity`), so water and lava sit at a fixed level forever.

The original Alephone light system is a 6-state machine (becoming_active, primary_active, secondary_active, becoming_inactive, primary_inactive, secondary_inactive) where each state has its own lighting function, period, and delta parameters. The `StaticLightData` struct in `marathon-formats` already parses all six `LightingFunctionSpec` blocks from the WAD, but `spawn_lights()` in `marathon-sim` discards five of them and only reads `primary_active`. Additionally, the `LightFunction` enum implements 4 of the 6 original lighting functions -- `Random` (pure random each tick) and `Fluorescent` (binary on/off toggle) are missing.

Without the state machine, lights cannot transition between active and inactive phases, cannot respond to switch/platform triggers, and cannot exhibit the layered animation behavior that gives Marathon levels their atmosphere. This is the single largest visual fidelity gap remaining after mesh geometry and movement physics fixes.

## What Changes

- Expand the `Light` component to hold all six `LightingFunctionSpec` entries (one per state), the current state enum, per-state phase/period tracking, initial/final intensity for the current state, light type (normal/strobe/media), flags (initially_active, slaved_intensities, stateless), and tag
- Add `LightState` enum with the 6 states and implement the state transition logic: when `phase >= period`, advance to the next state, randomize the new period via `delta_period`, and compute the new target intensity via `delta_intensity`
- Add `Random` and `Fluorescent` variants to `LightFunction` and implement their evaluation (random: `initial + rng * (final - initial)`; fluorescent: `if rng > 0.5 { final } else { initial }`)
- Refactor `compute_light_intensity()` to operate on the current state's function using `initial_intensity` and `final_intensity` (the state entry and target values) rather than the flat `intensity_min`/`intensity_max` fields
- Add a `run_light_updates()` system call in `SimWorld::tick()` that iterates all `Light` entities each tick: advance phase, handle state transitions, compute new intensity, and write `current_intensity`
- Update `spawn_lights()` to populate the full state machine from `StaticLightData`, including all six function specs, flags, and initial state selection based on the `_light_is_initially_active` flag
- Add a `run_media_updates()` step (or extend the existing media system) that reads each `Media` entity's `light_index`, looks up the corresponding `Light`'s `current_intensity`, and updates `media.current_height` accordingly
- Update the world-mechanics spec to reflect the full 6-state light machine and the 6 lighting functions, replacing the current 4-function stateless description

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities

- `world-mechanics`: Light animation gains the full 6-state machine with per-state transitions, delta randomization, all 6 lighting functions, light type/flag support, and per-tick evaluation integrated into the sim tick loop. Media height updates become driven by live light intensity each tick rather than being static.
- `level-rendering`: Polygon floor/ceiling light values in the GPU buffer must be refreshed each frame from the sim's updated `current_intensity` values, enabling visible dynamic lighting in all renderers (viewer, game, web).

## Impact

- `marathon-sim/src/components.rs` -- `Light` struct gains state machine fields (state enum, six function specs, initial/final intensity, light type, flags, tag); `LightFunction` enum gains `Random` and `Fluorescent` variants; existing `Light` fields (`function`, `period`, `phase`, `intensity_min`, `intensity_max`) are replaced by per-state equivalents
- `marathon-sim/src/world_mechanics/lights.rs` -- `compute_light_intensity()` refactored to accept initial/final intensity and current state's function; new `advance_light_state()` and `rephase_light()` functions for state transitions; new `update_lights()` system entry point
- `marathon-sim/src/world.rs` -- `spawn_lights()` rewritten to populate full state machine from all six `LightingFunctionSpec` blocks in `StaticLightData`
- `marathon-sim/src/tick.rs` -- `SimWorld::tick()` calls `run_light_updates()` and `run_media_updates()` in the world mechanics phase
- `marathon-sim/src/world_mechanics/media.rs` -- `compute_media_height()` already exists and is correct; new orchestration code calls it each tick with the associated light's live intensity
- `marathon-web/src/render.rs`, `marathon-game/src/mesh.rs`, `marathon-viewer/src/level.rs` -- Polygon buffer update loops must write `floor_light` and `ceiling_light` from the sim's per-light `current_intensity` each frame (currently written once at load)
- `marathon-formats/src/map.rs` -- No changes needed; `StaticLightData` and `LightingFunctionSpec` already parse all required fields
- Existing tests in `lights.rs` and `integration.rs` must be updated to use the new `Light` struct shape; new tests needed for state transitions, delta randomization, `Random`/`Fluorescent` functions, and media-light coupling
