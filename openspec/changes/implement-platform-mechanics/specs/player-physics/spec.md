## MODIFIED Requirements

### Requirement: Collision with walls
The system SHALL prevent the player from moving through solid walls (lines with the `SOLID` flag and no passable side). When movement would cross a solid line, the player's position SHALL be adjusted to remain on the valid side with velocity projected along the wall (sliding collision). The floor and ceiling heights used for passability checks SHALL reflect the current platform positions as stored in `MapGeometry`, which are updated each tick by the world mechanics phase. This means a door platform that has opened (ceiling raised) SHALL allow passage, and a door that has closed (ceiling lowered) SHALL block passage.

#### Scenario: Walk through open door
- **WHEN** a door platform has fully extended (ceiling raised to maximum) and the player moves toward the door line
- **THEN** the player SHALL pass through the line because the ceiling clearance now exceeds the player's height

#### Scenario: Blocked by closed door
- **WHEN** a door platform is at rest (ceiling at minimum, blocking the opening) and the player moves toward the door line
- **THEN** the player SHALL be blocked because the ceiling-to-floor clearance is less than the player's height

#### Scenario: Elevator raises player
- **WHEN** the player is standing on a FromFloor platform that is Extending
- **THEN** the player's Z position SHALL rise with the floor (grounded on the rising floor via the existing gravity/floor-snap logic reading updated MapGeometry heights)

#### Scenario: Crusher pushes player down
- **WHEN** the player is on a FromCeiling platform polygon and the ceiling descends
- **THEN** if the ceiling reaches the player's head (position.z + height > ceiling), the crush check SHALL detect it and either damage the player or reverse the platform

### Requirement: Step climbing
The system SHALL allow the player to step up onto ledges where the floor height difference is within `step_delta`. Floor heights used for step checks SHALL reflect platform positions from `MapGeometry`. A platform that has raised a floor creates a new step-up opportunity; a platform that has lowered a floor creates a new drop.

#### Scenario: Step onto raised platform
- **WHEN** a FromFloor platform has raised its floor by 0.25 WU and the player walks toward the platform polygon edge
- **THEN** the player SHALL step up onto the platform because 0.25 WU is within `step_delta`

#### Scenario: Cannot step onto high platform
- **WHEN** a FromFloor platform has raised its floor by 1.5 WU and `step_delta` is 0.5 WU
- **THEN** the player SHALL be blocked at the polygon boundary
