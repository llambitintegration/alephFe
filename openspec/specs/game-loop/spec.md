## ADDED Requirements

### Requirement: Construct simulation world from map and physics data
The system SHALL construct a `SimWorld` from a `MapData` and `PhysicsData` loaded via marathon-formats. Construction SHALL spawn ECS entities for all map objects (monsters, items, players) at their initial positions, initialize platform state from `StaticPlatformData`, initialize light state from `StaticLightData`, and initialize media state from `MediaData`. The deterministic PRNG SHALL be seeded from a provided seed value.

#### Scenario: Construct world from Marathon 2 level
- **WHEN** `SimWorld::new()` is called with valid `MapData` containing 5 monsters, 10 items, and 1 player start
- **THEN** the world SHALL contain 5 monster entities, 10 item entities, and 1 player entity at their map-defined positions, with components initialized from the physics data

#### Scenario: Invalid physics data
- **WHEN** `SimWorld::new()` is called with `PhysicsData` where `monsters` is `None` but the map references monster types
- **THEN** the system SHALL return an error indicating missing physics data

### Requirement: Advance simulation by one tick
The system SHALL advance the simulation state by exactly one tick (1/30th of a second) when `tick()` is called with the current frame's `ActionFlags`. All systems SHALL execute in the defined order: input processing, player physics, monster AI, weapon/combat, projectile physics, damage resolution, world mechanics, cleanup.

#### Scenario: Single tick advance
- **WHEN** `tick()` is called with `ActionFlags::MOVE_FORWARD`
- **THEN** the player's position SHALL change according to movement physics, and all other systems SHALL have executed in order

#### Scenario: Empty action flags
- **WHEN** `tick()` is called with empty `ActionFlags`
- **THEN** the simulation SHALL still advance (gravity, monster AI, projectiles, etc.) but the player SHALL have no input-driven movement

### Requirement: Deterministic simulation from seed
The system SHALL produce identical simulation state given the same initial conditions (map data, physics data, seed) and the same sequence of `ActionFlags` inputs. Two `SimWorld` instances initialized identically and given identical tick inputs SHALL have identical state after any number of ticks.

#### Scenario: Deterministic replay
- **WHEN** two `SimWorld` instances are created with the same seed and map, and both receive the identical sequence of 100 `ActionFlags` ticks
- **THEN** both worlds SHALL have identical player positions, monster states, and entity counts

### Requirement: Query player state
The system SHALL expose accessor methods to query the current player state including position (Vec3), facing angle, health, shield, oxygen, equipped weapon, ammunition counts, inventory items, and current polygon index.

#### Scenario: Query player position
- **WHEN** `sim_world.player_position()` is called after advancing several ticks
- **THEN** the system SHALL return the player's current world position as a Vec3

#### Scenario: Query player health after damage
- **WHEN** the player has taken damage during the simulation
- **THEN** `sim_world.player_health()` SHALL return the reduced health value

### Requirement: Query entity states for rendering
The system SHALL expose methods to iterate over all active entities (monsters, items, projectiles, effects) with their positions, facing angles, shape descriptors, and animation frame indices. This data is consumed by the renderer.

#### Scenario: Query monster positions
- **WHEN** `sim_world.entities()` is called
- **THEN** the system SHALL return an iterator over all active entities with position, facing, shape, and frame data

#### Scenario: Despawned entity not returned
- **WHEN** a monster has been killed and its death animation has completed
- **THEN** that entity SHALL not appear in the `entities()` iterator

### Requirement: Detect level completion and teleport events
The system SHALL detect when the player enters an inter-level teleporter polygon or when a terminal teleport is triggered. The system SHALL expose these events via a query method so the integration layer can handle level transitions.

#### Scenario: Player enters inter-level teleporter
- **WHEN** the player's position is inside a polygon with type `Teleporter` that targets another level
- **THEN** `sim_world.pending_events()` SHALL contain a `LevelTeleport` event with the target level index

### Requirement: Serialize and deserialize simulation state
The system SHALL support serializing the complete simulation state (all entity components, PRNG state, tick count) to bytes via serde, and deserializing back to a functional `SimWorld`. This enables save/load functionality.

#### Scenario: Round-trip serialization
- **WHEN** a `SimWorld` is serialized after 100 ticks, then deserialized, then advanced 50 more ticks
- **THEN** the result SHALL be identical to advancing the original world 150 ticks total
