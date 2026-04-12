## MODIFIED Requirements

### Requirement: Local input routed through network session in multiplayer
In multiplayer mode, the local player's `TickInput` SHALL be submitted to the `NetSession` rather than directly to `SimWorld::tick()`. The `NetSession` SHALL return the authoritative input set (local + remote, confirmed or predicted) for all player slots. The input capture pipeline (keyboard, mouse, gamepad -> RawInput -> ActionFlags -> TickInput) SHALL remain unchanged. Only the destination of the produced `TickInput` changes in multiplayer mode.

#### Scenario: Single-player input path unchanged
- **WHEN** the game is in single-player mode
- **THEN** the captured `TickInput` SHALL be passed directly to `SimWorld::tick(&[input])` with no network session involvement

#### Scenario: Multiplayer input submission
- **WHEN** the game is in multiplayer mode and the input system produces a `TickInput` for the current tick
- **THEN** the `TickInput` SHALL be submitted to `NetSession::add_local_input()` and SHALL NOT be passed directly to the sim

#### Scenario: Network session provides authoritative inputs
- **WHEN** `NetSession::advance()` is called after local input is submitted
- **THEN** the session SHALL return a slice of `TickInput` values (one per player slot) representing the authoritative inputs for the current tick, incorporating both local and remote inputs (confirmed or predicted)

### Requirement: TickInput serialization for network transmission
The `TickInput` struct (ActionFlags u32 + mouse_yaw f32 + mouse_pitch f32) SHALL be serializable to a fixed-size 12-byte representation for GGRS transmission. The serialization SHALL use `bytemuck::Pod` derivation for zero-copy conversion. The byte layout SHALL be little-endian: bytes 0-3 for ActionFlags bits, bytes 4-7 for mouse_yaw, bytes 8-11 for mouse_pitch.

#### Scenario: TickInput serialized to 12 bytes
- **WHEN** a `TickInput` with `ActionFlags(0x0301)`, `mouse_yaw = 0.1`, `mouse_pitch = -0.05` is serialized for GGRS
- **THEN** the result SHALL be exactly 12 bytes in the specified little-endian layout

#### Scenario: TickInput deserialized from 12 bytes
- **WHEN** a 12-byte GGRS input is received from a remote peer
- **THEN** the system SHALL reconstruct the original `TickInput` with correct `ActionFlags`, `mouse_yaw`, and `mouse_pitch` values

### Requirement: Input context awareness of Lobby state
The input system SHALL recognize the `Lobby` game state and apply `Menu` context bindings when the game is in the `Lobby` state. Menu navigation actions (Up, Down, Select, Back) SHALL be available for navigating lobby UI elements (player list, settings, ready button).

#### Scenario: Lobby uses menu input context
- **WHEN** the game state is `Lobby`
- **THEN** the active input context SHALL be `Menu` and menu navigation actions SHALL be available

#### Scenario: Lobby to Playing context switch
- **WHEN** the game state transitions from `Lobby` to `Loading` to `Playing`
- **THEN** the input context SHALL switch from `Menu` to `Gameplay`
