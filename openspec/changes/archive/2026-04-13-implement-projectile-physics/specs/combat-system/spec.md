## MODIFIED Requirements

### Requirement: Projectile creation and movement
The system SHALL spawn projectiles as ECS entities with `Position`, `Velocity`, `Projectile`, `PolygonIndex`, and optionally `ProjectileSource` and `HomingTarget` components. Projectile initial velocity SHALL be along the firing entity's facing direction at the speed defined in `ProjectileDefinition`, offset by `dx`/`dz` from the trigger definition. Each tick, the projectile physics system SHALL advance every active projectile by its velocity vector, accumulating `distance_traveled`. Projectiles with the `AFFECTED_BY_GRAVITY` flag SHALL have gravity applied at `WORLD_ONE/120` per tick squared; `DOUBLE_GRAVITY` doubles this; `HALF_GRAVITY` halves it. Projectiles with the `HORIZONTAL_WANDER` or `VERTICAL_WANDER` flags SHALL receive small random velocity perturbations each tick via `SimRng`.

#### Scenario: Projectile created from weapon fire
- **WHEN** the weapon system produces a `FireResult` with `fired: true` and `projectile_count: 1`
- **THEN** the system SHALL spawn a projectile entity at the player's weapon offset position, with velocity along the facing direction at the `ProjectileDefinition.speed`, and with `PolygonIndex` matching the player's current polygon

#### Scenario: Burst fire spawns multiple projectiles
- **WHEN** the weapon system produces a `FireResult` with `projectile_count` > 1 and `theta_error` > 0
- **THEN** the system SHALL spawn `projectile_count` projectile entities, each with a random angular offset within `theta_error` radians of the facing direction

#### Scenario: Monster ranged attack spawns projectile
- **WHEN** a monster AI produces a `MonsterAction::RangedAttack` with a projectile type
- **THEN** the system SHALL spawn a projectile entity at the monster's position with velocity toward its target, and `ProjectileSource` set to the monster entity

#### Scenario: Gravity-affected projectile arcs
- **WHEN** a projectile has the `AFFECTED_BY_GRAVITY` flag
- **THEN** the projectile's vertical velocity SHALL decrease by `WORLD_ONE/120` WU per tick squared each tick, creating an arc trajectory

#### Scenario: Double gravity projectile
- **WHEN** a projectile has both `AFFECTED_BY_GRAVITY` and `DOUBLE_GRAVITY` flags
- **THEN** gravity acceleration SHALL be `WORLD_ONE/60` WU per tick squared (double the normal rate)

#### Scenario: Half gravity projectile
- **WHEN** a projectile has both `AFFECTED_BY_GRAVITY` and `HALF_GRAVITY` flags
- **THEN** gravity acceleration SHALL be `WORLD_ONE/240` WU per tick squared (half the normal rate)

#### Scenario: Wandering projectile
- **WHEN** a projectile has the `HORIZONTAL_WANDER` flag
- **THEN** the projectile's XY velocity direction SHALL receive a small random perturbation each tick

#### Scenario: Projectile exceeds maximum range
- **WHEN** a projectile has traveled beyond its `maximum_range` (and `maximum_range > 0`)
- **THEN** the projectile SHALL be removed without detonation effects

### Requirement: Homing projectile tracking
The system SHALL support homing projectiles (flagged `GUIDED`) that adjust their velocity toward a target each tick. For player-fired guided projectiles, the target SHALL be a world-space point computed by ray-casting from the player's camera position along their look direction. For monster-fired guided projectiles, the target SHALL be the position of the monster's target entity. The `apply_homing()` function SHALL turn the velocity toward the target at a rate limited by the projectile's turning speed, preserving the projectile's scalar speed.

#### Scenario: Player-fired homing projectile tracks crosshair
- **WHEN** a player-fired projectile has the `GUIDED` flag and the player is looking at a point in the world
- **THEN** the projectile's velocity direction SHALL adjust toward the player's aim point each tick, limited by the turning rate

#### Scenario: Monster-fired homing projectile tracks target entity
- **WHEN** a monster-fired projectile has the `GUIDED` flag and the source monster has a valid target entity
- **THEN** the projectile's velocity direction SHALL adjust toward the target entity's position each tick

#### Scenario: No target available
- **WHEN** a homing projectile has no valid target (player aim hits nothing, monster has no target)
- **THEN** the projectile SHALL continue in a straight line without turning

#### Scenario: Homing preserves speed
- **WHEN** a homing projectile adjusts direction toward its target
- **THEN** the projectile's scalar speed SHALL remain equal to `ProjectileDefinition.speed`

### Requirement: Projectile collision detection
The system SHALL check each projectile's movement path against wall lines and entity collision volumes each tick. Wall collision uses `check_projectile_wall_collision()` with the projectile's current polygon adjacency data. Entity collision uses `check_projectile_entity_collision()` against all collidable entities (monsters, player) excluding the source entity. The projectile's `current_polygon` SHALL be tracked and updated as it crosses polygon boundaries.

#### Scenario: Projectile hits solid wall and detonates
- **WHEN** a projectile's movement path crosses a solid line and the projectile does NOT have the `REBOUNDS_FROM_WALLS` flag
- **THEN** the projectile SHALL detonate at the wall intersection point

#### Scenario: Projectile rebounds from wall
- **WHEN** a projectile's movement path crosses a solid line and the projectile HAS the `REBOUNDS_FROM_WALLS` flag
- **THEN** the projectile's velocity SHALL be reflected across the wall normal, the projectile SHALL be placed at the hit point, and the `rebound_sound` SHALL play

#### Scenario: Projectile hits floor and bounces
- **WHEN** a projectile's new position is below the floor height of its polygon and the projectile HAS the `REBOUNDS_FROM_FLOOR` flag
- **THEN** the projectile's Z velocity SHALL be negated (with energy loss), and the `rebound_sound` SHALL play

#### Scenario: Projectile hits floor and detonates
- **WHEN** a projectile's new position is below the floor height and the projectile does NOT have the `REBOUNDS_FROM_FLOOR` flag
- **THEN** the projectile SHALL detonate at the floor intersection point

#### Scenario: Projectile hits ceiling
- **WHEN** a projectile's new position is above the ceiling height of its polygon
- **THEN** the projectile SHALL detonate at the ceiling intersection point

#### Scenario: Projectile hits monster and detonates
- **WHEN** a projectile's path intersects a monster's collision radius and Z range, and the projectile is NOT persistent
- **THEN** the projectile SHALL deal its direct damage to the monster and detonate

#### Scenario: Persistent projectile passes through entity
- **WHEN** a projectile with the `PERSISTENT` flag intersects an entity's collision volume
- **THEN** the projectile SHALL deal its direct damage to the entity but SHALL NOT detonate, continuing to travel

#### Scenario: Projectile passes through transparent wall
- **WHEN** a projectile's path crosses a transparent line and the projectile has the `USUALLY_PASS_TRANSPARENT_SIDE` flag
- **THEN** the projectile SHALL pass through the line without detonating

#### Scenario: Projectile sometimes passes through transparent wall
- **WHEN** a projectile's path crosses a transparent line and the projectile has the `SOMETIMES_PASS_TRANSPARENT_SIDE` flag
- **THEN** the projectile SHALL have a 50% chance (via `SimRng`) of passing through the line

#### Scenario: Projectile polygon tracking
- **WHEN** a projectile crosses a polygon boundary through a passable line
- **THEN** the projectile's `current_polygon` SHALL be updated to the adjacent polygon

### Requirement: Damage calculation and application
The system SHALL calculate damage amounts using `DamageDefinition`: `base` + random(0, `random`) scaled by `scale`. There are 24+ damage types, each with a type index. Entities SHALL have immunities (bitmask of types that deal zero damage) and weaknesses (bitmask of types that deal double damage). The system SHALL emit `SimEvent::EntityDamaged` events that include the target entity, damage amount, and damage type. When an entity's health reaches zero, the system SHALL emit `SimEvent::EntityKilled`.

#### Scenario: Normal damage to unresistant target
- **WHEN** a projectile with base damage 20, random 10, scale 1.0 hits a monster with no immunities
- **THEN** the monster SHALL take between 20 and 30 damage

#### Scenario: Immune to damage type
- **WHEN** a projectile's damage type matches a bit in the monster's `immunities` bitmask
- **THEN** the monster SHALL take zero damage from that projectile

#### Scenario: Weak to damage type
- **WHEN** a projectile's damage type matches a bit in the monster's `weaknesses` bitmask
- **THEN** the damage SHALL be doubled before application

#### Scenario: Area-of-effect damage on detonation
- **WHEN** a projectile with `area_of_effect > 0` detonates at a point
- **THEN** all entities within the `area_of_effect` radius SHALL take damage scaled linearly by distance (full damage at center, zero at edge), using `calculate_aoe_damage()`

#### Scenario: Direct hit plus splash damage
- **WHEN** a projectile with `area_of_effect > 0` hits an entity directly
- **THEN** the entity SHALL receive BOTH the full direct-hit damage AND the distance-scaled AoE damage

#### Scenario: Self-damage from splash
- **WHEN** a player-fired projectile with `area_of_effect > 0` detonates near the player
- **THEN** the player SHALL take distance-scaled AoE damage (self-damage is not prevented)

### Requirement: Detonation effect spawning
The system SHALL spawn visual effect entities when a projectile detonates. On detonation, the system SHALL spawn an `Effect` entity at the impact point using `ProjectileDefinition.detonation_effect` as the definition index (if >= 0). If the projectile detonates while submerged in a liquid media, the system SHALL use `media_detonation_effect` instead. The system SHALL emit a `SimEvent::SoundTrigger` at the detonation point.

#### Scenario: Detonation spawns effect
- **WHEN** a projectile detonates at a wall, floor, ceiling, or entity
- **THEN** the system SHALL spawn an `Effect` entity at the impact point with `definition_index` from `detonation_effect` and a `ticks_remaining` based on the effect definition

#### Scenario: Detonation in liquid uses media effect
- **WHEN** a projectile detonates at a point that is below the media surface height of its polygon
- **THEN** the system SHALL use `media_detonation_effect` instead of `detonation_effect`

#### Scenario: No detonation effect defined
- **WHEN** a projectile detonates and `detonation_effect` is -1
- **THEN** no `Effect` entity SHALL be spawned, but damage and cleanup SHALL still occur

### Requirement: Contrail effect spawning
The system SHALL spawn contrail `Effect` entities at regular intervals during a projectile's flight. Every `ticks_between_contrails` ticks, if `contrail_effect >= 0` and `contrails_spawned < maximum_contrails`, the system SHALL spawn an `Effect` entity at the projectile's current position using `contrail_effect` as the definition index and increment the contrail counter.

#### Scenario: Contrail spawns at interval
- **WHEN** a projectile with `contrail_effect >= 0` has been alive for a multiple of `ticks_between_contrails` ticks
- **THEN** the system SHALL spawn an `Effect` entity at the projectile's current position

#### Scenario: Contrail cap reached
- **WHEN** a projectile has spawned `maximum_contrails` contrail effects
- **THEN** no further contrails SHALL be spawned for that projectile

#### Scenario: No contrail defined
- **WHEN** a projectile has `contrail_effect < 0`
- **THEN** no contrail effects SHALL be spawned

### Requirement: Media interaction
The system SHALL detect when a projectile crosses a liquid media surface boundary. If the projectile's `media_projectile_promotion >= 0`, the system SHALL replace the projectile with a new projectile of the promoted type at the same position. If `media_projectile_promotion < 0` and the projectile does not have the `PASSES_MEDIA_BOUNDARY` flag, the projectile SHALL detonate at the media surface, spawning the `media_detonation_effect`. Projectiles with the `PASSES_MEDIA_BOUNDARY` flag SHALL cross liquid surfaces without effect.

#### Scenario: Projectile promoted on media entry
- **WHEN** a projectile enters a liquid surface and `media_projectile_promotion >= 0`
- **THEN** the projectile SHALL be replaced with a new projectile entity of the promoted type, retaining position and velocity direction but using the new type's speed

#### Scenario: Projectile detonates at media surface
- **WHEN** a projectile enters a liquid surface, `media_projectile_promotion < 0`, and the projectile lacks `PASSES_MEDIA_BOUNDARY`
- **THEN** the projectile SHALL detonate at the media surface, spawning `media_detonation_effect`

#### Scenario: Projectile passes through media
- **WHEN** a projectile with the `PASSES_MEDIA_BOUNDARY` flag enters a liquid surface
- **THEN** the projectile SHALL continue through the surface without detonation or promotion
