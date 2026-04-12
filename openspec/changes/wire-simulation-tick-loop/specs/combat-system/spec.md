## MODIFIED Requirements

### Requirement: Weapon tick driven from action flags
The system SHALL call `tick_weapon()` for the player's equipped weapon each tick, passing the `FIRE_PRIMARY` and `FIRE_SECONDARY` action flags. When a weapon fires, the system SHALL spawn a projectile entity with position, velocity, and definition index derived from the weapon's trigger definition and the player's current position and facing.

#### Scenario: Player fires primary weapon
- **WHEN** `FIRE_PRIMARY` is set, the weapon is idle, and ammunition is available
- **THEN** `tick_weapon()` SHALL return true, a projectile entity SHALL be spawned traveling along the player's facing direction at the projectile's defined speed, and ammunition SHALL be decremented

#### Scenario: Player fires with empty magazine
- **WHEN** `FIRE_PRIMARY` is set but the magazine is empty and reserves exist
- **THEN** the weapon SHALL enter reloading state and no projectile SHALL be spawned

#### Scenario: Weapon cooldown prevents firing
- **WHEN** the weapon's cooldown_ticks is greater than zero
- **THEN** the cooldown SHALL decrement by 1 and no projectile SHALL be spawned regardless of action flags

### Requirement: Projectile lifecycle driven each tick
The system SHALL advance all projectile entities each tick in sequence: (1) call `advance_projectile()` to update position and accumulate distance traveled, (2) call `apply_projectile_gravity()` for gravity-affected projectiles, (3) call `apply_homing()` for homing projectiles with a valid target, (4) call `check_projectile_wall_collision()` against the current polygon's walls, (5) call `check_projectile_entity_collision()` against monsters and the player, (6) on hit, call `calculate_damage()` and `apply_damage()` to the target, (7) on hit or range exceeded, despawn the projectile entity and optionally spawn a detonation `Effect` entity.

#### Scenario: Projectile advances one tick
- **WHEN** a projectile entity exists with velocity (1.0, 0.0, 0.0)
- **THEN** after one tick the projectile's position SHALL increase by (1.0, 0.0, 0.0) and its `distance_traveled` SHALL increase by 1.0

#### Scenario: Gravity-affected projectile arcs downward
- **WHEN** a projectile has the gravity flag and is in flight
- **THEN** its Z velocity SHALL decrease by the gravitational constant each tick

#### Scenario: Homing projectile adjusts toward target
- **WHEN** a homing projectile is in flight and a monster target exists
- **THEN** the projectile's velocity direction SHALL rotate toward the target by up to the turning rate per tick

#### Scenario: Projectile hits wall and detonates
- **WHEN** a projectile's movement path crosses a solid line in its current polygon
- **THEN** the projectile SHALL be despawned, damage SHALL be applied (including AoE if defined), and a detonation `Effect` entity SHALL be spawned at the hit point

#### Scenario: Projectile hits monster and deals damage
- **WHEN** a projectile's path intersects a monster's collision radius
- **THEN** the monster's health SHALL decrease by the calculated damage amount (accounting for immunities and weaknesses), and the projectile SHALL be despawned

#### Scenario: Projectile exceeds maximum range
- **WHEN** a projectile's `distance_traveled` exceeds its definition's `maximum_range`
- **THEN** the projectile SHALL be despawned without detonation effects

#### Scenario: AoE damage on detonation
- **WHEN** a projectile with `area_of_effect` > 0 detonates
- **THEN** all entities within the AoE radius SHALL take damage scaled by distance from the detonation point via `calculate_aoe_damage()`
