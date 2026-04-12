## ADDED Requirements

### Requirement: Per-tick world state checksum on confirmed frames
The system SHALL compute a checksum of the simulation state after each confirmed tick (not predicted ticks). The checksum SHALL cover all determinism-critical state: player positions, velocities, facing angles, health, shield, oxygen for all player slots; monster positions and AI states; projectile positions; RNG state; and the tick counter. The checksum algorithm SHALL be Fletcher-64 for speed and low collision probability.

#### Scenario: Checksum computed after confirmed tick
- **WHEN** GGRS confirms frame 100 and the simulation advances with confirmed inputs
- **THEN** a Fletcher-64 checksum SHALL be computed over the determinism-critical state and associated with frame 100

#### Scenario: Predicted ticks do not compute checksums
- **WHEN** the simulation advances with predicted (unconfirmed) inputs
- **THEN** no checksum SHALL be computed for that tick

#### Scenario: Checksum covers all critical state
- **WHEN** the checksum is computed
- **THEN** the inputs to the hash SHALL include: all players' (position, velocity, facing, health, shield, oxygen), all monsters' (position, state), all projectiles' (position, distance_traveled), the PRNG internal state, and the tick counter

### Requirement: Checksum included in GGRS sync payload
Each client's checksum for confirmed frames SHALL be included in the GGRS synchronization data. GGRS transmits this data alongside input confirmations, allowing peers to compare checksums without additional protocol messages.

#### Scenario: Checksums match across all peers
- **WHEN** all peers compute the same checksum for confirmed frame 100
- **THEN** no desync is detected and gameplay continues normally

#### Scenario: Checksum mismatch detected
- **WHEN** peer A computes checksum X and peer B computes checksum Y for the same confirmed frame, and X != Y
- **THEN** the system SHALL detect a desync condition for that frame

### Requirement: Desync diagnostic logging
When a desync is detected (checksum mismatch), the system SHALL log diagnostic information including: the frame number, the local checksum value, the remote checksum value, and a full dump of the local determinism-critical state at the divergent frame. The diagnostic log SHALL be written to the game's log output at ERROR level.

#### Scenario: Desync triggers diagnostic dump
- **WHEN** a checksum mismatch is detected at frame 500
- **THEN** the system SHALL log: the frame number (500), both checksum values, and the complete determinism-critical state (all player positions, monster states, RNG state, etc.) for that frame

#### Scenario: Desync does not crash the game
- **WHEN** a desync is detected
- **THEN** the game SHALL continue running (not panic or abort) but SHALL display a warning indicator to the player indicating that the game state may be inconsistent

### Requirement: Cross-platform determinism validation
The desync detection system SHALL serve as the primary mechanism for validating cross-platform determinism (native x86_64 vs WASM). Automated tests SHALL run identical input sequences on both platforms and compare the resulting checksums to verify the simulation produces identical state.

#### Scenario: Native and WASM produce identical checksums
- **WHEN** the same map, seed, and 1000-tick input sequence are run on native and WASM builds
- **THEN** the checksums at every confirmed frame SHALL be identical

#### Scenario: Floating-point divergence detected
- **WHEN** a floating-point operation produces different results on native vs WASM
- **THEN** the checksum comparison SHALL detect the divergence and the diagnostic log SHALL identify which component(s) diverged
