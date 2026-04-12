## MODIFIED Requirements

### Requirement: Advance simulation by one tick with multi-player input
The system SHALL advance the simulation state by exactly one tick (1/30th of a second) when `tick()` is called with a slice of `TickInput` values, one per player slot. All systems SHALL execute in the defined order: input processing, player physics, monster AI, weapon/combat, projectile physics, damage resolution, world mechanics, cleanup. Player physics SHALL iterate over all player entities in slot order, each consuming their slot's `TickInput`. The single-player path SHALL pass a one-element slice. The tick ordering SHALL be identical regardless of the number of players, preserving determinism.

#### Scenario: Single-player tick (backward compatible)
- **WHEN** `tick(&[input])` is called with a one-element slice containing `ActionFlags::MOVE_FORWARD`
- **THEN** the single player entity's position SHALL change according to movement physics, identically to the pre-multiplayer behavior

#### Scenario: Two-player tick
- **WHEN** `tick(&[input_0, input_1])` is called where player 0 moves forward and player 1 strafes left
- **THEN** player 0's position SHALL change from forward movement and player 1's position SHALL change from strafing, both processed in the same tick in slot order

#### Scenario: Eight-player tick
- **WHEN** `tick(inputs)` is called with 8 `TickInput` values
- **THEN** all 8 player entities SHALL be updated in slot order (0 through 7) within the same tick, and the resulting state SHALL be deterministic given the same inputs

#### Scenario: Empty action flags for a player slot
- **WHEN** one player slot's `TickInput` has empty action flags
- **THEN** that player SHALL have no input-driven movement but the simulation SHALL still advance for all other systems

### Requirement: Construct simulation world with multiple players
The system SHALL accept a `num_players` parameter when constructing a `SimWorld`. When `num_players > 1`, the system SHALL spawn multiple `Player` entities at distinct spawn points selected by the active `GameMode` via `get_spawn_point()`. Each `Player` entity SHALL have a `PlayerSlot` component identifying which input slot (0-based index) drives that entity. When `num_players == 1`, the system SHALL behave identically to the current single-player construction.

#### Scenario: Construct world with 4 players
- **WHEN** `SimWorld::new()` is called with `num_players = 4` and a map containing at least 4 spawn points
- **THEN** the world SHALL contain 4 player entities, each with a unique `PlayerSlot` value (0, 1, 2, 3), at distinct spawn point positions

#### Scenario: Construct world with 1 player (backward compatible)
- **WHEN** `SimWorld::new()` is called with `num_players = 1`
- **THEN** the world SHALL contain 1 player entity with `PlayerSlot(0)`, behaving identically to the current implementation

#### Scenario: Insufficient spawn points
- **WHEN** `SimWorld::new()` is called with `num_players = 8` but the map has only 4 spawn points
- **THEN** the system SHALL reuse spawn points (cycling through available points) to place all 8 players

### Requirement: Fast in-memory save and restore for rollback
The `SimWorld` SHALL provide `save_state() -> Box<SavedState>` and `load_state(&SavedState)` methods for fast in-memory snapshotting. These methods SHALL be separate from the existing `serialize()`/`deserialize()` path, which remains for save files and film recording. `SavedState` SHALL capture all ECS components, all resources (including RNG state and tick counter), and all entity relationships. Multiple `SavedState` instances SHALL coexist independently. The combined time for one `save_state()` plus one `load_state()` SHALL be under 1 ms for a typical Marathon level.

#### Scenario: Save and restore produces identical state
- **WHEN** `save_state()` is called after 100 ticks, then 50 more ticks are run, then `load_state()` restores the saved state, then the same 50 ticks are replayed with identical inputs
- **THEN** the resulting simulation state SHALL be identical to the state after the original 150 ticks

#### Scenario: Save/restore does not affect serialize/deserialize
- **WHEN** `save_state()` and `load_state()` are used for rollback
- **THEN** the existing `serialize()` and `deserialize()` methods SHALL continue to function correctly for save files and film recording

### Requirement: Query player state by slot
The system SHALL expose accessor methods to query player state by slot index. `player_position(slot)`, `player_facing(slot)`, `player_health(slot)`, etc. SHALL return the state for the specified player slot. The existing no-argument versions SHALL continue to return the first player's state (slot 0) for backward compatibility.

#### Scenario: Query player 0 position (backward compatible)
- **WHEN** `sim_world.player_position()` is called (no slot argument)
- **THEN** the system SHALL return player slot 0's position, identical to pre-multiplayer behavior

#### Scenario: Query player 2 position by slot
- **WHEN** `sim_world.player_position_for_slot(2)` is called in a 4-player game
- **THEN** the system SHALL return the position of the player entity with `PlayerSlot(2)`

#### Scenario: Query invalid slot
- **WHEN** `sim_world.player_position_for_slot(5)` is called in a 4-player game
- **THEN** the system SHALL return `None`

### Requirement: Deterministic simulation with multiple players
Two `SimWorld` instances initialized with the same map data, physics data, seed, and `num_players` SHALL produce identical simulation state given the same sequence of multi-player input slices. Determinism SHALL hold regardless of the number of players.

#### Scenario: Deterministic multi-player replay
- **WHEN** two `SimWorld` instances are created with the same seed, map, and `num_players = 4`, and both receive the identical sequence of 100 multi-player tick inputs
- **THEN** both worlds SHALL have identical state for all 4 players, all monsters, and all entities
