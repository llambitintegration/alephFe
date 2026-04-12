## MODIFIED Requirements

### Requirement: Construct simulation world from map and physics data
The system SHALL construct a `SimWorld` from a `MapData` and `PhysicsData` loaded via marathon-formats. Before construction, the system SHALL apply MML override data from the resolved `MmlOverrideSet` to the `PhysicsData`, modifying monster definitions, weapon definitions, projectile definitions, effect definitions, and physics constants according to the overrides. Construction SHALL then proceed with the overridden physics data: spawning ECS entities for all map objects (monsters, items, players) at their initial positions, initializing platform state from `StaticPlatformData`, initializing light state from `StaticLightData`, and initializing media state from `MediaData`. The deterministic PRNG SHALL be seeded from a provided seed value. If the `MmlOverrideSet` contains dynamic limits overrides, the system SHALL use the overridden limits for entity pool allocation.

#### Scenario: Construct world with MML-overridden monster stats
- **WHEN** `SimWorld::new()` is called with `PhysicsData` where MML overrides have set monster 5's vitality to 500 (original was 100)
- **THEN** monster entities of type 5 SHALL be initialized with vitality 500

#### Scenario: Construct world with overridden dynamic limits
- **WHEN** the `MmlOverrideSet` specifies `monsters=1024` (default was 512)
- **THEN** the simulation world SHALL allocate monster entity pools for up to 1024 monsters

#### Scenario: Construct world with no MML overrides
- **WHEN** the `MmlOverrideSet` is empty (no overrides)
- **THEN** the system SHALL construct the world using the original WAD-parsed physics data unchanged
