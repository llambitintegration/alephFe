## ADDED Requirements

### Requirement: Monster behavioral state machine
The system SHALL implement a state machine for each monster with states: `Idle`, `Alerted`, `Attacking`, `Moving`, `Fleeing`, `Dying`, `Dead`. State transitions SHALL be driven by events (seeing a target, taking damage, target dying, vitality reaching zero). Each state SHALL determine the monster's animation sequence and behavior.

#### Scenario: Idle monster sees player
- **WHEN** an idle monster has line-of-sight to the player within its `visual_range` and `half_visual_arc`
- **THEN** the monster SHALL transition to `Alerted` and play its activation sound

#### Scenario: Monster vitality reaches zero
- **WHEN** a monster's vitality is reduced to zero or below
- **THEN** the monster SHALL transition to `Dying` state with the appropriate death animation (hard or soft depending on damage type)

#### Scenario: Monster finishes dying
- **WHEN** a monster's death animation completes
- **THEN** the monster SHALL transition to `Dead` state and become a non-collidable corpse

### Requirement: Target acquisition
The system SHALL allow alerted monsters to select a target entity. Monsters SHALL prefer the player as a target. Monsters SHALL evaluate targets based on distance, line-of-sight, and the monster's `enemies` bitmask. If a monster takes damage from a friendly entity, it SHALL switch its target to the attacker (friendly-fire response).

#### Scenario: Monster acquires player target
- **WHEN** an alerted monster has line-of-sight to the player
- **THEN** the monster SHALL set the player as its current target

#### Scenario: Friendly fire redirects target
- **WHEN** monster A (friends with monster B) takes damage from monster B's projectile
- **THEN** monster A SHALL switch its target to monster B and play its friendly-fire sound

### Requirement: Line-of-sight check
The system SHALL determine line-of-sight between two entities by tracing a ray through the polygon adjacency graph. A line-of-sight check SHALL fail if the ray crosses any solid line (no transparent side). The check SHALL also account for the monster's `half_visual_arc` (horizontal field of view) and `half_vertical_visual_arc`.

#### Scenario: Clear line of sight
- **WHEN** no solid walls exist between a monster and the player in the polygon adjacency path
- **THEN** the line-of-sight check SHALL return true

#### Scenario: Wall blocks line of sight
- **WHEN** a solid wall exists between the monster and player
- **THEN** the line-of-sight check SHALL return false

#### Scenario: Target outside visual arc
- **WHEN** the player is behind the monster (angle exceeds `half_visual_arc`)
- **THEN** the line-of-sight check SHALL return false even if no walls intervene

### Requirement: Monster movement and pathfinding
The system SHALL move monsters toward their targets using the polygon adjacency graph for pathfinding. Monsters SHALL walk from their current polygon toward adjacent polygons that reduce the graph distance to the target's polygon. Movement speed SHALL be determined by the monster definition's `speed` field. Monsters SHALL respect collision with walls and other entities.

#### Scenario: Monster walks toward player
- **WHEN** a monster is in Moving state targeting the player
- **THEN** the monster SHALL move along the shortest polygon-graph path toward the player at its defined speed

#### Scenario: Monster blocked by wall
- **WHEN** a monster's path requires crossing a solid line with insufficient clearance (height < monster height or floor delta > maximum_ledge_delta)
- **THEN** the monster SHALL stop at the wall and attempt alternate adjacent polygons

#### Scenario: Flying monster movement
- **WHEN** a monster has the `_monster_flys` flag set
- **THEN** the monster SHALL move in 3D toward its target at `preferred_hover_height` above the floor

### Requirement: Monster attacks
The system SHALL execute monster attacks based on the monster's `melee_attack` and `ranged_attack` definitions. When in range, the monster SHALL attack at the frequency defined by `attack_frequency`. Melee attacks SHALL deal damage directly. Ranged attacks SHALL spawn a projectile of the type defined in `attack_type`.

#### Scenario: Melee attack in range
- **WHEN** a monster's target is within the melee attack's `range` and `attack_frequency` ticks have elapsed
- **THEN** the monster SHALL execute the melee attack, dealing damage to the target with the defined repetitions

#### Scenario: Ranged attack
- **WHEN** a monster's target is within the ranged attack's `range` but beyond melee range
- **THEN** the monster SHALL spawn a projectile of the defined `attack_type` aimed at the target, offset by `dx`, `dy`, `dz`, with random error up to `error`

### Requirement: Monster activation cascading
The system SHALL propagate monster activation to nearby monsters. When a monster becomes alerted, other monsters of the same class within a propagation radius that share the same `enemies` bitmask SHALL also become alerted. Activation sounds SHALL trigger sound events.

#### Scenario: Nearby monster activated by cascade
- **WHEN** monster A becomes alerted and monster B of the same class is within cascade range
- **THEN** monster B SHALL also transition to `Alerted` state

#### Scenario: Different class not cascaded
- **WHEN** monster A becomes alerted but monster C is of a different class with different enemies
- **THEN** monster C SHALL remain in its current state

### Requirement: Monster gravity and floor tracking
The system SHALL apply gravity to non-flying monsters. Monsters SHALL track the floor height of their current polygon and fall when airborne. Gravity acceleration and terminal velocity SHALL use the monster definition's `gravity` and `terminal_velocity` fields.

#### Scenario: Monster falls off ledge
- **WHEN** a monster walks past an edge where the adjacent floor is lower
- **THEN** the monster SHALL fall at its defined gravitational rate

#### Scenario: Flying monster hovers
- **WHEN** a flying monster's current height is below `preferred_hover_height`
- **THEN** the monster SHALL move upward toward the hover height
