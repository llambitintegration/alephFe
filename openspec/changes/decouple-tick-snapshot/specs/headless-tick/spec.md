## ADDED Requirements

### Requirement: Simulation ticks and snapshots with no GPU

`SimWorld` SHALL be constructable, tickable, and able to produce `render_snapshot()` without any GPU device, window, or rendering context. The render-snapshot path SHALL have no dependency on a graphics backend.

#### Scenario: Headless tick-and-snapshot loop

- **WHEN** a `SimWorld` is constructed in a headless test process and ticked N times, calling `render_snapshot()` after each tick
- **THEN** each call SHALL succeed and return a `WorldSnapshot` without initializing any GPU resources

### Requirement: Serialized snapshots are deterministic across runs

Given identical construction inputs and identical per-tick `TickInput` sequences, the serialized bytes of `render_snapshot()` at each tick SHALL be reproducible across separate runs of the headless harness.

#### Scenario: Two headless runs produce identical snapshot bytes

- **WHEN** the headless harness is run twice with the same seed/level and the same input sequence, serializing each frame's `render_snapshot()`
- **THEN** the per-tick serialized byte streams from the two runs SHALL be identical

#### Scenario: render_snapshot does not perturb determinism

- **WHEN** the harness calls `render_snapshot()` between ticks in one run but not in an otherwise-identical reference run
- **THEN** the simulation state and resulting snapshot bytes at each tick SHALL match the reference run (read-only snapshot has no effect on the deterministic tick sequence)
