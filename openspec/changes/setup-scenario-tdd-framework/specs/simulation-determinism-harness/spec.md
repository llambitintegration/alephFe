## ADDED Requirements

### Requirement: Simulation determinism harness loads real levels
The `marathon-sim` crate SHALL have a test harness that loads golden levels from real scenario data via `marathon-formats` and constructs a `SimWorld` for determinism testing.

#### Scenario: Harness loads Marathon 2 Waterloo Waterpark
- **WHEN** the determinism harness loads the `m2-waterloo-waterpark` golden level
- **THEN** it SHALL successfully parse the WAD file, extract level 0 map data, and construct a `SimWorld`
- **AND** the `SimWorld` SHALL have a valid player entity positioned at the level's spawn point

#### Scenario: Harness loads Marathon Infinity Ne Cede Malis
- **WHEN** the determinism harness loads the `minf-ne-cede-malis` golden level
- **THEN** it SHALL successfully parse the WAD v4 file, extract level 0 map data, and construct a `SimWorld`

#### Scenario: Harness skips gracefully when data is absent
- **WHEN** the determinism harness attempts to load a golden level whose fixture files are not present
- **THEN** the test SHALL print `SKIP: <level-id> data not found` to stderr and return success

### Requirement: Simulation produces deterministic output
Running the same simulation with the same inputs SHALL produce identical physics state every time.

#### Scenario: Idle simulation is deterministic
- **WHEN** a golden level is loaded and simulated for 60 ticks with the `idle` input script (no player input)
- **THEN** the player position (x, y, z), facing angle, and containing polygon index SHALL be identical across two independent runs of the same test

#### Scenario: Walk-forward simulation is deterministic
- **WHEN** a golden level is loaded and simulated for 60 ticks with the `walk-forward` input script (forward key held every tick)
- **THEN** the player position and velocity SHALL be identical across two independent runs

#### Scenario: Determinism check compares full state
- **WHEN** the determinism harness runs a simulation twice
- **THEN** it SHALL compare at minimum: player world position (x, y, z), player velocity vector, player facing angle, and player containing polygon index

### Requirement: Golden physics values are asserted
The harness SHALL assert that simulation output matches expected golden values declared in `tests/scenarios.toml`.

#### Scenario: Idle golden values match
- **WHEN** a golden level with `tier2` values and `input_script = "idle"` is simulated for the specified `tick_count`
- **THEN** the player's x position SHALL be within 0.01 world units of the expected `player_x`
- **AND** the player's y position SHALL be within 0.01 world units of the expected `player_y`
- **AND** the player's containing polygon SHALL equal the expected `player_polygon`

#### Scenario: Walk-forward golden values match
- **WHEN** a golden level with `tier2` values and `input_script = "walk-forward"` is simulated for the specified `tick_count`
- **THEN** the player position SHALL have changed from the spawn point
- **AND** the final position SHALL be within 0.01 world units of the expected golden values

### Requirement: Input scripts are declaratively defined
The harness SHALL support named input scripts that define per-tick `ActionFlags` sequences.

#### Scenario: Idle input script
- **WHEN** the `idle` input script is loaded
- **THEN** it SHALL return `ActionFlags` with no movement or action bits set for every tick

#### Scenario: Walk-forward input script
- **WHEN** the `walk-forward` input script is loaded
- **THEN** it SHALL return `ActionFlags` with the forward movement bit set for every tick

#### Scenario: Strafe-left input script
- **WHEN** the `strafe-left` input script is loaded
- **THEN** it SHALL return `ActionFlags` with the left-strafe bit set for every tick

#### Scenario: Unknown input script produces error
- **WHEN** the harness attempts to load an input script named `nonexistent`
- **THEN** the test SHALL fail with a clear error message indicating the script was not found
