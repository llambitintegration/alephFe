## MODIFIED Requirements

### Requirement: Light animation executed each tick
The system SHALL call `compute_light_intensity()` for every `Light` entity each tick, passing the current tick count and the deterministic `SimRng`. The computed intensity SHALL be written back to the `Light` component's `current_intensity` field. Flickering lights SHALL consume RNG draws to maintain determinism.

#### Scenario: Smooth light updates each tick
- **WHEN** a smooth-function light with period 60 exists
- **THEN** after 30 ticks its `current_intensity` SHALL equal the maximum intensity (cosine peak at half period)

#### Scenario: Flickering light uses deterministic RNG
- **WHEN** two identically-seeded SimWorlds each contain a flickering light
- **THEN** the light's `current_intensity` SHALL be identical on every tick across both worlds

#### Scenario: Constant light intensity unchanged
- **WHEN** a constant-function light exists
- **THEN** its `current_intensity` SHALL remain at `intensity_max` regardless of tick count

### Requirement: Media height updated each tick from light intensity
The system SHALL update every `Media` entity's `current_height` each tick by calling `compute_media_height()` with the intensity of the media's associated light (looked up by `light_index`). The media update SHALL run after the light update so it uses the freshly computed light intensity.

#### Scenario: Water rises as light intensity increases
- **WHEN** a water media has `height_low` = 0.0, `height_high` = 2.0, and its associated light's `current_intensity` increases to 0.75
- **THEN** the media's `current_height` SHALL be 1.5

#### Scenario: Media height at minimum when light at zero
- **WHEN** a media's associated light has `current_intensity` = 0.0
- **THEN** the media's `current_height` SHALL equal `height_low`

### Requirement: Platform state machine advanced each tick
The system SHALL call `tick_platform()` for every `Platform` entity each tick. After ticking, the platform's `current_floor` and `current_ceiling` SHALL be written back to `MapGeometry::floor_heights` and `MapGeometry::ceiling_heights` for the platform's `polygon_index`. The system SHALL check `should_activate()` for player-entry triggered platforms when the player's `PolygonIndex` matches a platform's polygon. Crush checks SHALL be performed via `check_platform_crush()` for entities on the platform's polygon.

#### Scenario: Platform extends over multiple ticks
- **WHEN** a platform with speed 0.5 is activated from floor_rest=0.0 to floor_extended=1.0
- **THEN** after 1 tick `MapGeometry::floor_heights[polygon_index]` SHALL be 0.5, and after 2 ticks it SHALL be 1.0

#### Scenario: Platform auto-activates on player entry
- **WHEN** the player's `PolygonIndex` matches a platform's `polygon_index` and the platform has `PLATFORM_ACTIVATE_ON_PLAYER_ENTRY` flag
- **THEN** the platform's state SHALL transition from `AtRest` to `Extending`

#### Scenario: Platform delays then returns
- **WHEN** a platform reaches its extended position with `return_delay` = 30
- **THEN** the platform SHALL wait 30 ticks at `AtExtended` then transition to `Returning`

#### Scenario: Platform crushes entity
- **WHEN** a ceiling platform descends and an entity on the polygon has insufficient clearance, and the platform has `crushes = true`
- **THEN** the entity SHALL take 10 damage via `apply_damage()`

#### Scenario: Non-crushing platform reverses
- **WHEN** a descending platform encounters an entity but `crushes = false`
- **THEN** the platform's state SHALL reverse direction

### Requirement: Effect entities count down and despawn
The system SHALL decrement `Effect::ticks_remaining` for every `Effect` entity each tick. When `ticks_remaining` reaches zero, the effect entity SHALL be despawned from the ECS world.

#### Scenario: Effect counts down
- **WHEN** an effect entity has `ticks_remaining` = 5
- **THEN** after 5 ticks the effect entity SHALL no longer exist in the world

#### Scenario: Effect visible while counting down
- **WHEN** an effect entity has `ticks_remaining` > 0
- **THEN** the effect SHALL appear in `entities()` query results for rendering

### Requirement: Item pickup detection each tick
The system SHALL check for overlap between the player's position/collision radius and all item entity positions each tick. When overlap is detected, the system SHALL call `item_effect()` to determine the pickup result and apply it to the player (restore health/shield/oxygen, add weapon, add ammo, add inventory item). The item entity SHALL be despawned after pickup. Items at maximum capacity (e.g., health pickup when health is already at max) SHALL not be picked up.

#### Scenario: Player picks up health item
- **WHEN** the player's collision radius overlaps a health item and player health is below maximum
- **THEN** the player's health SHALL increase by the item's defined amount and the item entity SHALL be despawned

#### Scenario: Player picks up weapon
- **WHEN** the player's collision radius overlaps a weapon item the player does not already have
- **THEN** the weapon SHALL be added to the player's inventory and the item entity SHALL be despawned

#### Scenario: Full health ignores health pickup
- **WHEN** the player's health is at maximum and their collision radius overlaps a health item
- **THEN** the item SHALL remain in the world

#### Scenario: Ammo pickup adds to reserves
- **WHEN** the player's collision radius overlaps an ammo item
- **THEN** the corresponding weapon's reserve ammunition SHALL increase by the item's defined amount and the item entity SHALL be despawned

### Requirement: Item respawn timers tick each frame
The system SHALL decrement `ItemRespawnState::remaining` for all active respawn timers each tick. When a timer reaches zero, the system SHALL be prepared to respawn the item at its original position (actual respawn entity spawning is deferred to multiplayer mode integration).

#### Scenario: Respawn timer counts down
- **WHEN** an `ItemRespawnState` has `remaining` = 100
- **THEN** after 100 ticks `remaining` SHALL be 0 and `tick()` SHALL return true
