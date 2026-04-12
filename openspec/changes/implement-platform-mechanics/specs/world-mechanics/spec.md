## MODIFIED Requirements

### Requirement: Platform movement
The system SHALL animate platforms (moving floors and ceilings) based on `StaticPlatformData` parsed from the map. Each platform SHALL have a resting position and an extended position computed from its `platform_type` and the polygon's initial heights. Platform movement speed SHALL follow the defined speed. Platforms SHALL support multiple activation types: player entry (standing on the polygon), player action key, monster entry, projectile impact, control panel activation, and linked platform cascading. The platform state machine SHALL support re-activation: activating a platform that is currently Extending SHALL cause it to Reverse (transition to Returning), and activating a platform that is Returning SHALL cause it to transition to Extending.

#### Scenario: Player-activated platform
- **WHEN** the player presses the action key while on a platform polygon with player-action activation
- **THEN** the platform SHALL begin moving from its resting position to its extended position at the defined speed

#### Scenario: Platform reaches destination
- **WHEN** a platform reaches its extended position
- **THEN** the platform SHALL stop and, if configured, begin a delay timer before returning

#### Scenario: Platform crushes entity
- **WHEN** a ceiling platform descends and an entity is between the floor and ceiling with clearance less than the entity's height
- **THEN** if the platform has the crush flag, the entity SHALL take damage via `SimEvent::EntityDamaged`; otherwise the platform SHALL reverse direction

#### Scenario: Platform reverses on re-activation
- **WHEN** a platform is Extending and is activated again (via action key, panel, or linked trigger)
- **THEN** the platform SHALL transition to Returning

#### Scenario: Door platform auto-returns after delay
- **WHEN** a door-type platform (type 0 or 1) reaches its extended position with a return_delay of 30 ticks
- **THEN** the platform SHALL wait 30 ticks at the extended position, then automatically transition to Returning

#### Scenario: Crush damage emits event
- **WHEN** a crushing platform reduces clearance below an entity's height and the entity is on the platform polygon
- **THEN** the system SHALL emit `SimEvent::EntityDamaged` with the crush damage amount and the entity's ECS entity ID

#### Scenario: Non-crushing platform reverses on obstruction
- **WHEN** a non-crushing platform reduces clearance below an entity's height
- **THEN** the platform SHALL reverse direction (Extending becomes Returning, Returning becomes Extending)

### Requirement: Platform triggers
The system SHALL support platforms that trigger other platforms or lights. When a platform reaches its extended or resting position, it SHALL activate linked platforms or toggle linked lights based on per-platform linked indices populated from the map data. Linked platform and light indices SHALL be stored as `linked_platforms: Vec<usize>` and `linked_lights: Vec<usize>` on the `Platform` component.

#### Scenario: Platform triggers linked platform
- **WHEN** platform A reaches its extended position and has platform B's index in `linked_platforms`
- **THEN** platform B SHALL begin its activation sequence

#### Scenario: Platform triggers light
- **WHEN** a platform reaches its extended position and has light index L in `linked_lights`
- **THEN** light L SHALL receive a toggle event

#### Scenario: Platform with tag-based linking
- **WHEN** multiple platforms share the same tag value from `StaticPlatformData.tag`
- **THEN** reaching a destination on one platform SHALL activate the other platforms with the same tag

### Requirement: Platform type enumeration
The `Platform` component SHALL include a `platform_type: PlatformType` field with six variants: `ExtendsFloorToCeiling` (0), `ExtendsCeilingToFloor` (1), `ExtendsFloorAndCeiling` (2), `FromFloor` (3), `FromCeiling` (4), `Teleporter` (5). The `spawn_platforms()` function SHALL read `StaticPlatformData.platform_type` and compute appropriate rest/extended heights for each type.

#### Scenario: Spawn door platform
- **WHEN** a platform with `platform_type = 1` (ExtendsCeilingToFloor) is loaded from a polygon with floor=0.0 and ceiling=3.0
- **THEN** the Platform component SHALL have `ceiling_rest = 3.0`, `ceiling_extended = 0.0`, `floor_rest = 0.0`, `floor_extended = 0.0`

#### Scenario: Spawn elevator platform
- **WHEN** a platform with `platform_type = 3` (FromFloor) is loaded with min_height=0.0, max_height=2.0
- **THEN** the Platform component SHALL have `floor_rest = 0.0`, `floor_extended = 2.0`

#### Scenario: Teleporter platform emits level teleport
- **WHEN** a Teleporter platform (type 5) is activated while the player is on it
- **THEN** the system SHALL emit `SimEvent::LevelTeleport` instead of moving heights
