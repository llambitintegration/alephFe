## MODIFIED Requirements

### Requirement: Collision with walls
The system SHALL prevent the player from moving through solid walls (lines with the `SOLID` flag and no passable side) using radius-based collision. The player's center SHALL be kept at least `radius` distance (from `PlayerPhysicsParams.radius`) from any impassable wall segment. When the player's center is closer than `radius` to a solid wall segment, the system SHALL push the player outward along the direction from the closest point on the segment to the player center until the distance equals `radius`. Velocity SHALL be projected parallel to the wall surface (sliding collision). The system SHALL iterate up to 3 times per tick to resolve multi-wall contacts (e.g., corners).

#### Scenario: Walk into solid wall with radius
- **WHEN** the player moves toward a solid line and the player center would be less than `radius` distance from the line segment
- **THEN** the player center SHALL be pushed outward to exactly `radius` distance from the wall and velocity SHALL be projected parallel to the wall surface

#### Scenario: Stand near wall without crossing
- **WHEN** the player is within `radius` distance of a solid wall but has not crossed the line
- **THEN** the player SHALL still be pushed outward to maintain `radius` distance (no crossing required to trigger collision)

#### Scenario: Walk through passable line
- **WHEN** the player moves toward a line that has a transparent side and sufficient height clearance
- **THEN** the player SHALL cross into the adjacent polygon (radius check SHALL NOT block passable lines)

#### Scenario: Corner collision
- **WHEN** the player moves into a corner where two solid walls meet
- **THEN** the system SHALL resolve both wall contacts via the multi-iteration loop, keeping the player center at least `radius` from both walls

#### Scenario: Slide along wall with radius offset
- **WHEN** the player moves diagonally into a solid wall
- **THEN** the player SHALL slide parallel to the wall at `radius` distance from it, with the velocity component perpendicular to the wall removed

## ADDED Requirements

### Requirement: Point-to-segment distance primitive
The system SHALL provide a `point_to_segment_distance` function in `collision.rs` that computes the shortest distance from a 2D point to a line segment, returning both the distance and the closest point on the segment. The closest point SHALL be computed by projecting the point onto the infinite line defined by the segment endpoints and clamping the parameter to [0, 1].

#### Scenario: Point projects onto segment interior
- **WHEN** the point's perpendicular projection falls between the segment endpoints
- **THEN** the closest point SHALL be the perpendicular foot and the distance SHALL be the perpendicular distance

#### Scenario: Point nearest to segment endpoint
- **WHEN** the point's perpendicular projection falls outside the segment (beyond an endpoint)
- **THEN** the closest point SHALL be the nearer endpoint and the distance SHALL be the distance to that endpoint
