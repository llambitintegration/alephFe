## ADDED Requirements

### Requirement: Weapon firing mechanics
The system SHALL implement weapon firing for the player based on `WeaponDefinition` and `TriggerDefinition` from physics data. When `FIRE_PRIMARY` or `FIRE_SECONDARY` action flags are set, the equipped weapon's corresponding trigger SHALL fire if the weapon is ready (not in reload/recovery). Firing SHALL consume ammunition, apply recoil, and spawn a projectile of the trigger's `projectile_type`. Firing SHALL respect `ticks_per_round` (minimum time between shots) and `recovery_ticks`.

#### Scenario: Fire primary trigger
- **WHEN** `FIRE_PRIMARY` is set, the weapon is ready, and ammunition is available
- **THEN** the system SHALL spawn a projectile, decrement ammunition by 1, and enter the firing cooldown for `ticks_per_round` ticks

#### Scenario: Fire with empty magazine
- **WHEN** `FIRE_PRIMARY` is set but the magazine is empty
- **THEN** the system SHALL play the click sound and begin auto-reload if ammunition reserves exist

#### Scenario: Burst fire
- **WHEN** a trigger has `burst_count` > 0
- **THEN** a single fire action SHALL spawn `burst_count` projectiles in rapid succession with the defined `theta_error` spread

### Requirement: Weapon switching and inventory
The system SHALL maintain the player's weapon inventory as a list of held weapons. `CYCLE_WEAPON_FWD` and `CYCLE_WEAPON_BACK` action flags SHALL cycle through available weapons. Weapon switching SHALL take `ready_ticks` before the new weapon can fire. The system SHALL support dual-wielded weapons (weapon class `twofisted_pistol`).

#### Scenario: Cycle to next weapon
- **WHEN** `CYCLE_WEAPON_FWD` is set
- **THEN** the system SHALL switch to the next weapon in the inventory and begin the `ready_ticks` transition

#### Scenario: Dual-wielded weapon
- **WHEN** the player holds two pistols (twofisted class)
- **THEN** `FIRE_PRIMARY` SHALL fire the right weapon and `FIRE_SECONDARY` SHALL fire the left weapon, each with independent ammunition tracking

### Requirement: Ammunition management
The system SHALL track ammunition separately per weapon trigger. Magazines have a capacity of `rounds_per_magazine`. When a magazine is depleted, the system SHALL auto-reload from reserve ammunition over `loading_ticks` + `finish_loading_ticks`. Ammunition pickups SHALL add to reserves.

#### Scenario: Magazine depletes and reloads
- **WHEN** the last round in a magazine is fired and reserves exist
- **THEN** the weapon SHALL enter reload state for the defined loading duration, then the magazine SHALL be refilled from reserves

#### Scenario: No reserves remaining
- **WHEN** the magazine is empty and no reserve ammunition exists
- **THEN** the weapon SHALL remain empty and fire attempts SHALL produce click sounds

### Requirement: Projectile creation and movement
The system SHALL spawn projectiles as ECS entities with position, velocity, damage definition, and lifetime. Projectile initial velocity SHALL be along the player's (or monster's) facing direction at the speed defined in `ProjectileDefinition`, offset by `dx`/`dz` from the trigger definition. Projectiles SHALL move along their velocity vector each tick. Projectiles with the `affected_by_gravity` flag SHALL have gravity applied.

#### Scenario: Projectile created at firing position
- **WHEN** a weapon trigger fires
- **THEN** a projectile entity SHALL be spawned at the player's weapon offset position, traveling along the facing direction at the projectile's defined speed

#### Scenario: Gravity-affected projectile arcs
- **WHEN** a projectile has the `affected_by_gravity` flag
- **THEN** the projectile's vertical velocity SHALL decrease by gravitational acceleration each tick, creating an arc trajectory

#### Scenario: Projectile exceeds maximum range
- **WHEN** a projectile has traveled beyond its `maximum_range`
- **THEN** the projectile SHALL be removed without detonation effects

### Requirement: Homing projectile tracking
The system SHALL support homing projectiles that adjust their velocity toward the nearest valid target each tick. Homing projectiles SHALL turn toward their target at a rate proportional to the projectile's turning speed. The homing behavior SHALL be indicated by a projectile flag.

#### Scenario: Homing projectile tracks target
- **WHEN** a homing projectile is in flight and a valid target exists
- **THEN** the projectile's velocity direction SHALL adjust toward the target each tick

#### Scenario: No target available
- **WHEN** a homing projectile has no valid target
- **THEN** the projectile SHALL continue in a straight line

### Requirement: Projectile collision detection
The system SHALL check projectile positions against wall lines and entity collision radii each tick. When a projectile intersects a solid line, it SHALL detonate at the intersection point. When a projectile's path intersects an entity's collision radius, it SHALL deal damage to that entity and detonate.

#### Scenario: Projectile hits wall
- **WHEN** a projectile's movement path crosses a solid line
- **THEN** the projectile SHALL detonate at the wall, spawning its `detonation_effect` and applying area-of-effect damage if `area_of_effect` > 0

#### Scenario: Projectile hits monster
- **WHEN** a projectile's path intersects a monster's collision radius
- **THEN** the projectile SHALL deal its defined damage to the monster and detonate

### Requirement: Damage calculation and application
The system SHALL calculate damage amounts using `DamageDefinition`: `base` + random(0, `random`) scaled by `scale`. There are 24+ damage types, each with a type index. Entities SHALL have immunities (bitmask of types that deal zero damage) and weaknesses (bitmask of types that deal double damage). The system SHALL emit damage events that include the source entity, target entity, damage amount, and damage type.

#### Scenario: Normal damage to unresistant target
- **WHEN** a projectile with base damage 20, random 10, scale 1.0 hits a monster with no immunities
- **THEN** the monster SHALL take between 20 and 30 damage

#### Scenario: Immune to damage type
- **WHEN** a projectile's damage type matches a bit in the monster's `immunities` bitmask
- **THEN** the monster SHALL take zero damage from that projectile

#### Scenario: Weak to damage type
- **WHEN** a projectile's damage type matches a bit in the monster's `weaknesses` bitmask
- **THEN** the damage SHALL be doubled before application

#### Scenario: Area-of-effect damage
- **WHEN** a projectile with `area_of_effect` > 0 detonates
- **THEN** all entities within the area radius SHALL take damage scaled by their distance from the detonation point
