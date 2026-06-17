## ADDED Requirements

### Requirement: Native consumes the render snapshot for per-polygon updates

The `marathon-game` (native) per-tick update SHALL write its GPU polygon buffer from `render_snapshot().poly_dynamic`, covering all five per-polygon fields — floor height, ceiling height, media height, floor light, and ceiling light — for every polygon, replacing the previous `snapshot()` + hand-computed byte-offset update path. Writes SHOULD be whole-`PolygonGpuData`-struct writes; the `size_of::<PolygonGpuData>() == 48` assertion SHALL remain as the layout guardrail.

#### Scenario: Native floor and ceiling heights follow the sim

- **WHEN** a platform raises its floor over successive native frames
- **THEN** the rendered native floor for that polygon SHALL track the sim height each frame

#### Scenario: GPU struct layout is validated

- **WHEN** the native renderer builds
- **THEN** the `size_of::<PolygonGpuData>() == 48` assertion SHALL hold, guaranteeing the per-polygon write layout matches the shader's expectations

### Requirement: Native lights animate

The `marathon-game` renderer SHALL update each polygon's floor and ceiling light intensity from the snapshot every tick. The previous no-op light stub SHALL be removed; native lights SHALL no longer be frozen.

#### Scenario: Native light visibly changes when toggled

- **WHEN** a light's intensity changes in the sim (e.g. a control panel toggles it or an animated light pulses)
- **THEN** the affected native polygon surfaces SHALL change brightness on the next frame

#### Scenario: No dead light update path remains

- **WHEN** the native per-tick polygon-buffer update runs
- **THEN** it SHALL write the floor/ceiling light fields from the snapshot (no `let _ = light;` discard or equivalent no-op for lights)

### Requirement: Native wall heights are dynamic

The `marathon-game` renderer SHALL apply the same height-source discriminator trick to wall quads so that native walls stretch with their adjacent polygons' floor/ceiling heights, without rebuilding the vertex or index buffers per frame.

#### Scenario: Native wall follows a moving platform

- **WHEN** an adjacent platform polygon changes height over successive native frames
- **THEN** the bordering native wall quads SHALL stretch to match, leaving no gap, while the vertex and index buffers remain immutable
