## ADDED Requirements

### Requirement: Player camera extraction from simulation
The system SHALL extract the player's position (Vec3), facing yaw, and look pitch from marathon-sim after each tick. These values SHALL be used to construct the view matrix for first-person rendering. The camera position SHALL be offset to eye height (player position + vertical eye offset from physics data).

#### Scenario: Camera follows player movement
- **WHEN** the simulation advances one tick with the player moving forward
- **THEN** the camera position SHALL update to match the player's new position at eye height

#### Scenario: Camera follows player facing
- **WHEN** the player turns right during a tick
- **THEN** the camera yaw SHALL update to match the player's new facing angle

### Requirement: Interpolated rendering state between ticks
The system SHALL maintain two rendering snapshots: the state at the previous tick and the state at the current tick. Each render frame, the system SHALL compute an interpolation factor `alpha = time_since_last_tick / tick_duration` (where tick_duration = 1/30 second). Entity positions and the camera SHALL be linearly interpolated between the two snapshots using this alpha value.

#### Scenario: Mid-tick frame at 60Hz
- **WHEN** the display renders at 60Hz (2 frames per tick) and the second frame occurs halfway between ticks
- **THEN** entities and the camera SHALL be rendered at positions 50% between their previous-tick and current-tick positions

#### Scenario: Frame immediately after tick
- **WHEN** a frame renders immediately after a simulation tick (alpha ≈ 0)
- **THEN** entities SHALL be rendered at approximately their current-tick positions

#### Scenario: Entity spawned this tick
- **WHEN** an entity exists in the current tick but not the previous tick
- **THEN** the entity SHALL be rendered at its current-tick position (no interpolation, since there is no previous state)

#### Scenario: Entity despawned this tick
- **WHEN** an entity existed in the previous tick but not the current tick
- **THEN** the entity SHALL not be rendered

### Requirement: Entity state collection for rendering
The system SHALL query marathon-sim each tick for all active entities and collect their renderable state: entity ID, position (Vec3), facing angle, entity type (monster/item/projectile/effect), collection index, sequence index, frame index, and any active transfer mode. This collected state forms the tick snapshot used for interpolation and sprite rendering.

#### Scenario: Collect monster state
- **WHEN** the simulation has 3 active monsters
- **THEN** the entity state collection SHALL contain 3 entries with positions, facing angles, and sprite references for each monster

#### Scenario: Collection updated each tick
- **WHEN** a monster moves between tick N and tick N+1
- **THEN** the tick N+1 snapshot SHALL contain the monster's updated position

### Requirement: Audio event dispatch from simulation
The system SHALL query marathon-sim after each tick for pending audio events. Each audio event SHALL contain a sound ID and a world position. The system SHALL forward these events to marathon-audio as one-shot spatial sounds positioned at the event's world coordinates. The audio listener position SHALL be updated to the player's position each tick.

#### Scenario: Weapon fire sound
- **WHEN** the simulation produces a weapon-fire audio event at the player's position
- **THEN** the system SHALL play the corresponding sound via marathon-audio at that position

#### Scenario: Monster alert sound
- **WHEN** a monster triggers its alert sound at position (4096, 2048, 0)
- **THEN** the system SHALL play the monster's alert sound spatially positioned at (4096, 2048, 0) relative to the listener

#### Scenario: No audio subsystem available
- **WHEN** the audio engine was not initialized (no audio device)
- **THEN** the system SHALL silently discard audio events without error

### Requirement: Platform and media state synchronization
The system SHALL read platform (elevator/door) positions and media (water/lava) heights from marathon-sim each tick and update the per-polygon storage buffer used by the level rendering pipeline. This ensures that moving platforms and rising/falling liquids are visually synchronized with the simulation state.

#### Scenario: Door opening
- **WHEN** the simulation advances a door platform from closed (ceiling at floor) to open (ceiling raised)
- **THEN** the per-polygon storage buffer SHALL be updated with the new ceiling height, and the door SHALL visually open in the rendered scene

#### Scenario: Water level rising
- **WHEN** the simulation advances a media's height from 0.0 to 512.0
- **THEN** the per-polygon storage buffer SHALL be updated with the new media height, and the liquid surface SHALL visually rise
