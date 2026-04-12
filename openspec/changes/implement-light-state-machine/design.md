## Context

Lights in the Rust sim are frozen at load time. The `Light` component stores only a single `LightFunction`, `period`, `phase`, `intensity_min`, and `intensity_max`. The `compute_light_intensity()` function evaluates this as a pure function of the global tick counter, but nothing in `SimWorld::tick()` actually calls it -- `current_intensity` is set once during `spawn_lights()` and never updated. Additionally, `spawn_lights()` discards five of the six `LightingFunctionSpec` blocks from `StaticLightData`, reading only `primary_active`.

The result: every light in every level is static. No flickering hallways, no pulsing alien machinery, no strobing alarms. Media height is also frozen because it derives from its associated light's intensity (`height = low + (high - low) * light_intensity`).

The original Alephone light system is a 6-state machine where each state has its own lighting function, period, delta parameters, and target intensity. The `StaticLightData` struct in `marathon-formats` already parses all six `LightingFunctionSpec` blocks from the WAD. The `LightFunction` enum implements 4 of the 6 original lighting functions -- `Random` (pure random each tick) and `Fluorescent` (binary on/off toggle) are missing.

Current light data flow:
```
WAD parse -> StaticLightData (6 function specs) -> spawn_lights() reads only primary_active
    -> Light component (single function, static intensity) -> never updated
```

Target light data flow:
```
WAD parse -> StaticLightData (6 function specs) -> spawn_lights() populates full state machine
    -> Light component (6 function specs, current state, phase tracking)
    -> run_light_updates() called each tick -> evaluates current state's function
    -> current_intensity updated -> renderers read live values each frame
```

## Goals / Non-Goals

**Goals:**
- Full 6-state light machine matching original Alephone behavior
- All 6 lighting functions: constant, linear, smooth, flicker, random, fluorescent
- Delta randomization for period and intensity on state transitions
- Per-tick light evaluation integrated into the sim tick loop
- Media height driven by live light intensity each tick
- GPU polygon buffer refreshed each frame with updated light values
- Light flags: initially_active, slaved_intensities, stateless
- Light types: normal, strobe, media

**Non-Goals:**
- Firing light (weapon discharge temporary light boost) -- separate feature
- Light-activated triggers / tag system wiring -- separate feature
- Shared mesh builder crate refactoring -- separate scope
- Interpolation between ticks for sub-tick smooth rendering
- Old light format (Marathon 1) state machine conversion

## Decisions

### Decision 1: Replace flat Light fields with per-state function spec array

**Choice:** Replace the single `function`, `period`, `intensity_min`, `intensity_max` fields on `Light` with a `[LightFunctionSpec; 6]` array indexed by `LightState`, plus `initial_intensity` and `final_intensity` fields that track the current state's entry and target values.

**Alternative considered:** Keep the flat fields and swap them on state transitions -- rejected because it loses the original data, making serialization/deserialization lossy and preventing re-entry into states with their original parameters.

**Rationale:** Matches the Alephone `static_light_data` layout. All six specs are immutable after spawn. The mutable runtime state is `state`, `phase`, `period` (with delta applied), `initial_intensity`, and `final_intensity`.

### Decision 2: State machine advances in-place, no separate timeline

**Choice:** Each `Light` entity tracks its own `phase` counter. `run_light_updates()` increments phase by 1 each tick, checks `phase >= period`, and calls `advance_light_state()` to transition. No global timeline or animation scheduler.

**Alternative considered:** A centralized light animation system with a sorted event queue -- rejected because Marathon lights are independent; each light has its own period and phase. Per-entity phase tracking is simpler and matches the original.

**Rationale:** Original Alephone iterates all lights each tick in `update_lights()`. The per-entity approach maps directly to a bevy_ecs query over `&mut Light`.

### Decision 3: Evaluate lighting functions with initial/final intensity (not min/max)

**Choice:** `compute_light_intensity()` takes `initial_intensity` (value at state entry) and `final_intensity` (target for this state) instead of `intensity_min` and `intensity_max`. Each function interpolates from initial toward final over the period.

**Alternative considered:** Keep min/max semantics and compute initial/final from them -- rejected because the state machine's initial intensity is the *previous state's final value*, not a fixed minimum. The asymmetry is fundamental to how transitions chain.

**Rationale:** Matches Alephone's `light_data.initial_intensity` / `light_data.final_intensity` fields. When entering a new state, `initial_intensity` = current intensity, `final_intensity` = state's spec intensity + random delta.

### Decision 4: Random and Fluorescent as new LightFunction variants

**Choice:** Add `Random` and `Fluorescent` to the existing `LightFunction` enum.

- Random: `initial + rng.gen::<f32>() * (final - initial)` -- pure random each tick between initial and final.
- Fluorescent: `if rng.gen::<bool>() { final } else { initial }` -- binary random toggle.

**Rationale:** Completes the 6-function set from Alephone. The existing `Flicker` variant currently implements pure random (which is actually Alephone's `Random`). We must fix `Flicker` to match Alephone's definition: smooth base oscillation + random jitter.

### Decision 5: Flicker function corrected to match Alephone semantics

**Choice:** Redefine `Flicker` to compute a smooth base value and add random variation on top:
```
smooth_value = smooth(phase, period, initial, final)
intensity = smooth_value + rng * (final - smooth_value)
```

**Alternative considered:** Leave current Flicker as-is and call the new one "AlephoFlicker" -- rejected for fidelity. We want to match the original engine.

**Rationale:** The current `Flicker` implementation (`initial + range * rng`) is actually the `Random` function from Alephone. The real `Flicker` is a smooth oscillation with random perturbation.

### Decision 6: GPU buffer update approach -- per-vertex light baked at mesh build, refreshed via sim query

**Choice:** The current architecture bakes light intensity into each vertex at mesh build time via `PolygonInfo.floor_light`. For dynamic lighting, renderers will query the sim's light entities each frame and rebuild the `PolygonInfo` array, then use `queue.write_buffer()` to update the vertex buffer's light values. In the near term, a simpler approach: the sim exposes a `light_intensities() -> Vec<f32>` method, and the render loop updates the vertex light values by writing to the GPU buffer at known offsets.

**Alternative considered:** Move light values to the per-polygon storage/uniform buffer instead of per-vertex -- this is cleaner but requires shader changes. Deferred to a future pass.

**Rationale:** Minimal shader changes. The vertex `light` field already exists and the fragment shader already reads it. We just need to update the values each frame instead of once at load time.

### Decision 7: Media height updates driven by light intensity each tick

**Choice:** Add `run_media_updates()` to the tick loop, after `run_light_updates()`. For each `Media` entity, look up its `light_index`, find the corresponding `Light` entity's `current_intensity`, and call `compute_media_height()` to update `current_height`.

**Alternative considered:** Update media in the same pass as lights -- possible but separating concerns makes the code clearer and matches Alephone's separate `update_media()` call.

**Rationale:** `compute_media_height()` already exists and is correct. We just need to call it each tick with live intensity values.

## Risks / Trade-offs

- [Flicker behavior change] Redefining `Flicker` to match Alephone breaks any existing test expectations for the current random behavior -> Mitigation: update tests to match new semantics, add `Random` variant to carry the old behavior.
- [Serialization breakage] Changing the `Light` struct shape breaks existing save data -> Mitigation: save/load is not yet used in production; update `SimSnapshot` to match new struct. Old snapshots are incompatible regardless.
- [Performance -- per-vertex buffer writes] Updating light values per-vertex each frame could be slow for large levels with many polygons -> Mitigation: Marathon levels typically have <1000 polygons with ~6 vertices each, so ~6000 vertex updates is negligible. Future optimization: move to per-polygon uniform/storage buffer.
- [State machine fidelity] Alephone's state machine has edge cases around stateless lights and slaved intensities -> Mitigation: implement the common case first (initially_active, no slaving), add flag handling incrementally.
