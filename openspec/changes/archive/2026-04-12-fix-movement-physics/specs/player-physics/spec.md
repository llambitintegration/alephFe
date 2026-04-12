## MODIFIED Requirements

### Requirement: Apply movement from action flags
The system SHALL translate `ActionFlags` and optional mouse delta values into player velocity and facing changes using the player's `PhysicsConstants`. The system SHALL track forward velocity and perpendicular velocity as independent scalar values. Forward/backward flags SHALL accelerate the forward velocity scalar along the facing direction. Strafe flags SHALL accelerate the perpendicular velocity scalar. Turn flags SHALL apply angular velocity through the acceleration/deceleration system. When proportional mouse yaw/pitch deltas are present, they SHALL be applied directly to facing and vertical look angles (bypassing the angular velocity system). Each velocity scalar SHALL be clamped to its physics-defined maximum independently. The system SHALL use the running physics constants (index 1) when available, falling back to walking (index 0).

#### Scenario: Forward movement
- **WHEN** `MOVE_FORWARD` is set and the player is on the ground
- **THEN** the player's forward velocity scalar SHALL increase by `acceleration` per tick, up to `maximum_forward_velocity`

#### Scenario: Backward movement
- **WHEN** `MOVE_BACKWARD` is set
- **THEN** the player's forward velocity scalar SHALL decrease by `acceleration` per tick, down to negative `maximum_backward_velocity`

#### Scenario: Strafing
- **WHEN** `STRAFE_LEFT` is set
- **THEN** the player's perpendicular velocity scalar SHALL change by `acceleration` per tick in the left direction, up to `maximum_perpendicular_velocity`

#### Scenario: Deceleration when no forward/backward input
- **WHEN** no forward/backward flags are set and the player has non-zero forward velocity
- **THEN** the forward velocity scalar SHALL decrease by `deceleration` per tick toward zero, independently of perpendicular velocity

#### Scenario: Deceleration when no strafe input
- **WHEN** no strafe flags are set and the player has non-zero perpendicular velocity
- **THEN** the perpendicular velocity scalar SHALL decrease by `deceleration` per tick toward zero, independently of forward velocity

#### Scenario: Direction reversal boost
- **WHEN** `MOVE_FORWARD` is set and the player's forward velocity is negative (moving backward)
- **THEN** the forward velocity scalar SHALL change by `acceleration + deceleration` per tick (both forces applied simultaneously), providing snappier direction reversal

#### Scenario: Mouse yaw applied directly to facing
- **WHEN** `mouse_yaw` is non-zero in the tick input
- **THEN** the player's facing angle SHALL change by exactly `mouse_yaw` radians, without going through the angular acceleration/deceleration system

#### Scenario: Mouse pitch applied directly to vertical look
- **WHEN** `mouse_pitch` is non-zero in the tick input
- **THEN** the player's vertical look angle SHALL change by exactly `mouse_pitch` radians, clamped to the maximum elevation limits

#### Scenario: Mouse yaw and keyboard turn compose
- **WHEN** `mouse_yaw` is non-zero AND `TURN_RIGHT` flag is set
- **THEN** the facing angle SHALL change by `mouse_yaw` radians PLUS the angular velocity contribution from the keyboard turn flag

#### Scenario: Keyboard turn without mouse
- **WHEN** `mouse_yaw` is 0.0 AND `TURN_RIGHT` flag is set
- **THEN** the angular velocity system SHALL apply acceleration as before (no behavioral change from current implementation)

#### Scenario: Running physics loaded by default
- **WHEN** the physics data contains two PhysicsConstants entries (walking at index 0, running at index 1)
- **THEN** the system SHALL use the running entry (index 1) for player physics parameters

#### Scenario: Fallback to walking physics
- **WHEN** the physics data contains only one PhysicsConstants entry (index 0)
- **THEN** the system SHALL use that entry for player physics parameters

#### Scenario: Angular constants converted to radians
- **WHEN** physics constants are loaded from Marathon data files
- **THEN** angular fields (angular_acceleration, angular_deceleration, maximum_angular_velocity, maximum_elevation) SHALL be converted from Marathon angle units (512 = full circle) to radians using the factor 2*pi/512

#### Scenario: Position computed from axis-decomposed velocity
- **WHEN** the player has forward velocity F and perpendicular velocity P with facing angle theta
- **THEN** the XY position change SHALL be computed as: dx = F*cos(theta) - P*sin(theta), dy = F*sin(theta) + P*cos(theta)

### Requirement: Track player state
The system SHALL maintain player state components: position (Vec3), forward velocity (f32), perpendicular velocity (f32), vertical velocity (f32), facing angle (f32 radians), vertical look angle, angular velocity (f32), health (i16), shield (i16), oxygen (i16), current polygon index, and grounded flag.

#### Scenario: Health clamped to valid range
- **WHEN** the player's health would exceed the maximum (via pickup)
- **THEN** health SHALL be clamped to the maximum value

#### Scenario: Polygon index updated on movement
- **WHEN** the player crosses from polygon 5 into polygon 8
- **THEN** the player's current polygon index SHALL update to 8

#### Scenario: Velocity decomposed into forward and perpendicular
- **WHEN** the player state is queried
- **THEN** forward velocity and perpendicular velocity SHALL be available as independent scalar values, not as a combined Vec2/Vec3
