## ADDED Requirements

### Requirement: GGRS rollback session management
The system SHALL wrap the simulation loop in a GGRS `P2PSession` that synchronizes inputs across 2-8 players. Each tick, GGRS SHALL determine which inputs are confirmed versus predicted. On misprediction, the session SHALL roll back the simulation to the last confirmed state, replay with corrected inputs, and resume normal play. The session SHALL be managed by the `marathon-net` crate's `NetSession` struct.

#### Scenario: Normal tick with all inputs confirmed
- **WHEN** GGRS has confirmed inputs from all players for the current frame
- **THEN** `NetSession::advance()` SHALL return the confirmed `TickInput` for each player slot and the simulation SHALL advance one tick with those inputs

#### Scenario: Remote input predicted
- **WHEN** GGRS has not yet received a remote player's input for the current frame
- **THEN** GGRS SHALL predict the remote input (repeating their last known input) and `NetSession::advance()` SHALL return the predicted input set for the simulation to advance speculatively

#### Scenario: Misprediction triggers rollback
- **WHEN** GGRS receives a confirmed remote input that differs from what was predicted
- **THEN** the session SHALL load the simulation state from the last confirmed frame via `SimWorld::load_state()`, replay all frames from the confirmed state forward with corrected inputs, and resume normal play

#### Scenario: Eight-player session
- **WHEN** a session is created with 8 players
- **THEN** GGRS SHALL manage input synchronization for all 8 player slots and `NetSession::advance()` SHALL return 8 `TickInput` values per tick

### Requirement: Configurable input delay and rollback window
The GGRS session SHALL support configurable input delay (number of frames of local input delay, default 2) and maximum rollback window (maximum number of frames that can be rolled back, default 8). The input delay trades responsiveness for reduced rollback frequency. The rollback window bounds worst-case save/restore cost.

#### Scenario: Input delay of 2 frames
- **WHEN** the session is configured with input delay 2
- **THEN** local input SHALL be scheduled 2 frames in the future, reducing the probability of remote input misprediction at the cost of 67 ms additional local latency

#### Scenario: Rollback window exceeded
- **WHEN** a remote player's confirmed input arrives more than 8 frames late (beyond the rollback window)
- **THEN** the session SHALL not roll back beyond the window and SHALL log a warning indicating potential desync risk

### Requirement: ActionFlags to GGRS input serialization
The system SHALL convert between `TickInput` (ActionFlags u32 + mouse_yaw f32 + mouse_pitch f32) and GGRS's fixed-size input representation. The GGRS input SHALL be exactly 12 bytes: 4 bytes for ActionFlags bits, 4 bytes for mouse_yaw (IEEE 754 f32), 4 bytes for mouse_pitch (IEEE 754 f32). Serialization SHALL use `bytemuck` for zero-copy conversion.

#### Scenario: Serialize TickInput to GGRS input
- **WHEN** a local `TickInput` with `ActionFlags(0x0103)`, `mouse_yaw = 0.05`, `mouse_pitch = -0.02` is submitted to the session
- **THEN** the GGRS input SHALL be a 12-byte array containing the ActionFlags bits followed by the two f32 values in little-endian byte order

#### Scenario: Deserialize GGRS input to TickInput
- **WHEN** GGRS provides a confirmed 12-byte input from a remote player
- **THEN** the session SHALL reconstruct a `TickInput` with the correct `ActionFlags`, `mouse_yaw`, and `mouse_pitch` values

### Requirement: SyncTest session for determinism validation
The system SHALL support a GGRS `SyncTestSession` mode that runs two copies of the simulation with identical inputs and verifies they produce identical state. This mode SHALL be usable in automated tests without any network transport. The sync test SHALL detect determinism violations by comparing save states.

#### Scenario: SyncTest with identical inputs produces no desync
- **WHEN** a `SyncTestSession` runs 1000 ticks with a predefined input sequence
- **THEN** no desync SHALL be detected and the test SHALL pass

#### Scenario: SyncTest detects non-determinism
- **WHEN** the simulation contains a non-deterministic operation (e.g., using system time)
- **THEN** the `SyncTestSession` SHALL detect the divergence and report a desync error

### Requirement: Fast simulation save and restore for rollback
The `SimWorld` SHALL provide `save_state() -> Box<SavedState>` and `load_state(&SavedState)` methods for fast in-memory snapshotting. These methods SHALL be separate from the existing `serialize()`/`deserialize()` path. Save and restore SHALL complete in under 1 ms combined for a typical Marathon level. The `SavedState` SHALL capture all ECS components, resources (including RNG state and tick counter), and entity relationships.

#### Scenario: Save and restore produces identical state
- **WHEN** `save_state()` is called after 100 ticks, then 50 more ticks are run, then `load_state()` restores the saved state, then the same 50 ticks are replayed with identical inputs
- **THEN** the resulting simulation state SHALL be identical to the state after the original 150 ticks

#### Scenario: Save/restore performance
- **WHEN** `save_state()` and `load_state()` are called on a level with 50 monsters, 30 items, 8 players, and active projectiles
- **THEN** the combined save + restore time SHALL be under 1 ms

#### Scenario: Multiple save states coexist
- **WHEN** `save_state()` is called at frames 10, 20, and 30
- **THEN** all three `SavedState` handles SHALL be valid and `load_state()` with any of them SHALL restore the corresponding state
