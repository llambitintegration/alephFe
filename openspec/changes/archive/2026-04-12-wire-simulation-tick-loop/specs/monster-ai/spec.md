## MODIFIED Requirements

### Requirement: Monster AI state machine advanced each tick
The system SHALL advance every non-Dead monster's AI state machine each tick by calling `can_see_target()` to evaluate vision, then `next_state()` to compute the state transition. The system SHALL query the player's current position (updated by player physics earlier in the tick) as the primary target for vision checks. State transitions SHALL update the `MonsterState` component on the entity.

#### Scenario: Idle monster detects player
- **WHEN** an idle monster has line-of-sight to the player within `visual_range` and `half_visual_arc`
- **THEN** the monster's `MonsterState` component SHALL transition from `Idle` to `Alerted` during this tick

#### Scenario: Alerted monster with target in range attacks
- **WHEN** an alerted monster's target is within melee or ranged attack range
- **THEN** the monster's state SHALL transition to `Attacking`

#### Scenario: Alerted monster with target out of range moves
- **WHEN** an alerted monster's target exists but is beyond attack range
- **THEN** the monster's state SHALL transition to `Moving`

#### Scenario: Monster dies when vitality reaches zero
- **WHEN** a monster's health reaches zero or below (from projectile damage earlier in the tick or from a previous tick)
- **THEN** the monster's state SHALL transition to `Dying`, and on the subsequent tick to `Dead`

### Requirement: Monster movement executed each tick
The system SHALL apply movement to monsters in `Moving` state each tick. Ground monsters SHALL move toward their target's polygon using direct movement in world space at the monster definition's `speed`. Flying monsters SHALL use `compute_flying_movement()` to move in 3D toward the target at `preferred_hover_height`. Non-flying monsters SHALL have gravity applied via `apply_monster_gravity()`.

#### Scenario: Ground monster moves toward player
- **WHEN** a ground monster is in `Moving` state with a target
- **THEN** the monster's position SHALL change toward the target at the monster's defined speed, and `PolygonIndex` SHALL update if the monster crosses a polygon boundary

#### Scenario: Flying monster hovers toward target
- **WHEN** a flying monster is in `Moving` state
- **THEN** the monster SHALL move in 3D via `compute_flying_movement()`, maintaining `preferred_hover_height` above the floor

#### Scenario: Ground monster affected by gravity
- **WHEN** a non-flying monster is airborne (position Z > floor height)
- **THEN** the monster SHALL fall at its defined gravitational rate via `apply_monster_gravity()`

### Requirement: Monster attacks executed each tick
The system SHALL call `compute_monster_attack()` for monsters in `Attacking` state each tick. Melee attacks SHALL directly apply damage to the target entity via `calculate_damage()` and `apply_damage()`. Ranged attacks SHALL spawn a projectile entity at the monster's position plus the attack offset, aimed at the target with the defined error angle. The `AttackCooldown` component SHALL be set to `attack_frequency` ticks after each attack.

#### Scenario: Monster melee attack
- **WHEN** a monster in `Attacking` state is within melee range and `AttackCooldown` is zero
- **THEN** the target entity SHALL take damage computed from the melee attack definition, and the monster's `AttackCooldown` SHALL be set to `attack_frequency`

#### Scenario: Monster ranged attack spawns projectile
- **WHEN** a monster in `Attacking` state is within ranged range, beyond melee range, and `AttackCooldown` is zero
- **THEN** a projectile entity SHALL be spawned at the monster's position plus offset, traveling toward the target with the defined error, and `AttackCooldown` SHALL be set

#### Scenario: Attack cooldown prevents attack
- **WHEN** a monster is in `Attacking` state but `AttackCooldown` is greater than zero
- **THEN** the cooldown SHALL decrement by 1 and no attack SHALL occur

### Requirement: Monster activation cascading executed on alert
The system SHALL call `find_cascade_targets()` when a monster transitions from `Idle` to `Alerted`. Nearby idle monsters of the same class with the same enemies bitmask SHALL also transition to `Alerted`.

#### Scenario: Cascade alerts nearby same-class monsters
- **WHEN** monster A transitions from `Idle` to `Alerted` and monster B is idle, same class, same enemies, within cascade radius
- **THEN** monster B's `MonsterState` SHALL also transition to `Alerted` during the same tick

#### Scenario: Different class not cascaded
- **WHEN** monster A becomes alerted but nearby monster C is a different class
- **THEN** monster C SHALL remain `Idle`
