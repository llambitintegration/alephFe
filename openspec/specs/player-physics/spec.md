## ADDED Requirements

### Requirement: Apply movement from action flags
The system SHALL translate `ActionFlags` into player velocity changes using the player's `PhysicsConstants`. Forward/backward flags SHALL accelerate along the facing direction. Strafe flags SHALL accelerate perpendicular to the facing direction. Turn flags SHALL apply angular velocity. Velocity SHALL be clamped to the physics-defined maximum values.

#### Scenario: Forward movement
- **WHEN** `MOVE_FORWARD` is set and the player is on the ground
- **THEN** the player's velocity along the facing direction SHALL increase by `acceleration` per tick, up to `maximum_forward_velocity`

#### Scenario: Backward movement
- **WHEN** `MOVE_BACKWARD` is set
- **THEN** the player's velocity along the negative facing direction SHALL increase, up to `maximum_backward_velocity`

#### Scenario: Strafing
- **WHEN** `STRAFE_LEFT` is set
- **THEN** the player's velocity perpendicular to the facing direction (left) SHALL increase, up to `maximum_perpendicular_velocity`

#### Scenario: Deceleration when no input
- **WHEN** no movement flags are set and the player has velocity
- **THEN** the player's velocity SHALL decrease by `deceleration` per tick toward zero

### Requirement: Apply gravity
The system SHALL apply gravitational acceleration to the player when airborne (not standing on a floor). The player's vertical velocity SHALL increase downward by `gravitational_acceleration` per tick, up to `terminal_velocity`. When the player contacts a floor, vertical velocity SHALL be zeroed.

#### Scenario: Player falls off ledge
- **WHEN** the player walks past a polygon edge where the adjacent polygon's floor is lower
- **THEN** the player SHALL accelerate downward at `gravitational_acceleration` per tick

#### Scenario: Landing on floor
- **WHEN** the player's vertical position reaches the floor height of the current polygon
- **THEN** vertical velocity SHALL be set to zero and the player SHALL be grounded

### Requirement: Collision with walls
The system SHALL prevent the player from moving through solid walls (lines with the `SOLID` flag and no passable side). When movement would cross a solid line, the player's position SHALL be adjusted to remain on the valid side with velocity projected along the wall (sliding collision).

#### Scenario: Walk into solid wall
- **WHEN** the player moves toward a solid line
- **THEN** the player SHALL stop at the wall and velocity SHALL be projected parallel to the wall surface (slide along it)

#### Scenario: Walk through passable line
- **WHEN** the player moves toward a line that has a transparent side and sufficient height clearance
- **THEN** the player SHALL cross into the adjacent polygon

### Requirement: Step climbing
The system SHALL allow the player to step up onto ledges where the floor height difference is within `step_delta`. When the player crosses a line where the adjacent polygon's floor is higher by no more than `step_delta`, the player's vertical position SHALL be set to the higher floor.

#### Scenario: Step up small ledge
- **WHEN** the player crosses into a polygon whose floor is 0.25 WU higher and `step_delta` is 0.5 WU
- **THEN** the player's Z position SHALL be set to the higher floor height

#### Scenario: Cannot step up tall ledge
- **WHEN** the player encounters a floor height difference greater than `step_delta`
- **THEN** the line SHALL act as a solid wall and the player SHALL not cross

### Requirement: Ceiling collision
The system SHALL prevent the player from moving into spaces where the ceiling-to-floor gap is less than the player's `height`. If jumping or riding a platform would push the player into the ceiling, vertical velocity SHALL be zeroed and position clamped.

#### Scenario: Low ceiling blocks entry
- **WHEN** the player tries to cross into a polygon where ceiling - floor < player height
- **THEN** the player SHALL be blocked as if by a solid wall

### Requirement: Media submersion effects
The system SHALL detect when the player is submerged in a media surface (polygon's `media_index` references a `MediaData` whose height exceeds the player's feet position). When submerged, the system SHALL apply drag to movement velocity and deplete oxygen at a fixed rate. When oxygen reaches zero, the system SHALL apply drowning damage per tick.

#### Scenario: Player enters water
- **WHEN** the player's feet position is below the water media surface height
- **THEN** movement velocity SHALL be reduced by a drag factor and oxygen SHALL decrease per tick

#### Scenario: Oxygen depleted
- **WHEN** oxygen reaches zero while submerged
- **THEN** the player SHALL take drowning damage each tick

#### Scenario: Surface from water
- **WHEN** the player's feet position rises above the water surface
- **THEN** oxygen SHALL begin recharging toward maximum

### Requirement: Track player state
The system SHALL maintain player state components: position (Vec3), velocity (Vec3), facing angle (f32 radians), vertical look angle, health (i16), shield (i16), oxygen (i16), current polygon index, and grounded flag.

#### Scenario: Health clamped to valid range
- **WHEN** the player's health would exceed the maximum (via pickup)
- **THEN** health SHALL be clamped to the maximum value

#### Scenario: Polygon index updated on movement
- **WHEN** the player crosses from polygon 5 into polygon 8
- **THEN** the player's current polygon index SHALL update to 8
