## ADDED Requirements

### Requirement: Render motion sensor as Canvas 2D radar circle
The system SHALL render a circular motion sensor on the left column of the HUD using a Canvas 2D element. The circle SHALL have a dark background with a subtle grid or crosshair overlay. The player's position SHALL be represented at the center of the circle.

#### Scenario: Motion sensor renders on HUD
- **WHEN** the HUD is visible during gameplay
- **THEN** a circular Canvas 2D element SHALL be rendered in the left column of the HUD panel

#### Scenario: Motion sensor shows center marker
- **WHEN** the motion sensor is rendered
- **THEN** a small bright dot or crosshair SHALL appear at the center representing the player

### Requirement: Display nearby entities as colored dots
The system SHALL render nearby entities as colored dots on the motion sensor, positioned relative to the player's location and facing direction. Dot color SHALL indicate entity type: red for enemies, green for allies, yellow for items. Dots SHALL be positioned proportionally to their distance from the player within sensor range.

#### Scenario: Enemy ahead within range
- **WHEN** an enemy is directly ahead of the player at 50% of sensor range
- **THEN** a red dot SHALL appear halfway between the center and the top edge of the sensor circle

#### Scenario: Entity behind and to the right
- **WHEN** an entity is behind and to the right of the player within sensor range
- **THEN** the dot SHALL appear in the lower-right quadrant of the sensor circle

#### Scenario: Entity beyond sensor range
- **WHEN** an entity is farther than the sensor maximum range
- **THEN** no dot SHALL appear for that entity

#### Scenario: Multiple entities at different positions
- **WHEN** three enemies are within range at different positions relative to the player
- **THEN** three red dots SHALL appear at positions corresponding to each enemy's relative location

### Requirement: Motion sensor rotates with player facing
The system SHALL orient entity dots on the motion sensor relative to the player's current facing direction. When the player rotates, dots SHALL reposition so that "up" on the sensor always corresponds to the direction the player is facing.

#### Scenario: Player rotates 90 degrees right
- **WHEN** the player rotates 90 degrees clockwise and an enemy was directly ahead
- **THEN** the enemy dot SHALL move to the left side of the sensor circle

#### Scenario: Player faces opposite direction
- **WHEN** the player rotates 180 degrees and an enemy was ahead
- **THEN** the enemy dot SHALL appear at the bottom of the sensor circle

### Requirement: Sim exposes nearby entity positions
The sim layer SHALL provide a method to query entity positions relative to the player within a given range. The method SHALL return each entity's relative X and Z coordinates and an entity type identifier. The query SHALL be bounded to a maximum number of results (16) to limit per-frame cost.

#### Scenario: Query with entities in range
- **WHEN** 5 enemies are within sensor range and 2 are beyond range
- **THEN** the query SHALL return 5 entries with relative positions and type=enemy

#### Scenario: Query with no entities
- **WHEN** no entities exist within sensor range
- **THEN** the query SHALL return an empty result set

#### Scenario: Query capped at maximum results
- **WHEN** 20 entities are within sensor range
- **THEN** the query SHALL return at most 16 entries, selecting the 16 nearest
