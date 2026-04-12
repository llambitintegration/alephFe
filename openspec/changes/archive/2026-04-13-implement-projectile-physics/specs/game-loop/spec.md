## MODIFIED Requirements

### Requirement: Advance simulation by one tick
The system SHALL advance the simulation state by exactly one tick (1/30th of a second) when `tick()` is called with the current frame's `TickInput`. All systems SHALL execute in the defined order: input processing, player physics, monster AI, weapon/combat, projectile physics, damage resolution, world mechanics, cleanup. The projectile physics step SHALL call `run_projectile_physics()`, which iterates all active `Projectile` entities and runs the per-tick lifecycle: apply gravity, apply homing, apply wander, advance position, check wall collision, check floor/ceiling collision, check entity collision, check media interaction, check range limit, spawn contrails, and handle detonation/cleanup.

#### Scenario: Single tick advance with projectiles
- **WHEN** `tick()` is called and active projectile entities exist in the world
- **THEN** all projectile entities SHALL be advanced by their velocity, checked for collisions, and detonated or continued as appropriate, in step 5 of the tick ordering

#### Scenario: Projectile spawned by weapon in same tick
- **WHEN** the weapon/combat step (step 4) produces a `FireResult` that spawns a projectile entity
- **THEN** the projectile physics step (step 5) SHALL process that newly spawned projectile in the same tick

#### Scenario: Projectile detonation produces damage event
- **WHEN** a projectile detonates and deals damage during step 5
- **THEN** the damage SHALL be recorded via `SimEvent::EntityDamaged` and the damaged entity's `Health` component SHALL be updated within the same tick

#### Scenario: Empty action flags with active projectiles
- **WHEN** `tick()` is called with empty `ActionFlags` but projectile entities exist
- **THEN** all projectile entities SHALL still be advanced and processed (projectile physics does not depend on player input)

### Requirement: Projectile state in simulation snapshots
The system SHALL include all active projectile entities in the `SimSnapshot` produced by `snapshot()`. Each `ProjectileSnapshot` SHALL include `definition_index`, `position`, `velocity`, `distance_traveled`, `ticks_alive`, `contrails_spawned`, and `current_polygon`. Deserialization SHALL restore projectile entities with all fields so that projectile flight continues correctly after a save/load cycle.

#### Scenario: Round-trip serialization with in-flight projectiles
- **WHEN** a `SimWorld` is serialized while projectiles are in flight, then deserialized and advanced one more tick
- **THEN** the deserialized projectiles SHALL continue from their saved positions with their saved velocities, and collision/detonation SHALL behave identically to if the world had not been serialized
