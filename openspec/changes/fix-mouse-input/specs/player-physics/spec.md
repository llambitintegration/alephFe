## MODIFIED Requirements

### Requirement: Apply movement from action flags
The system SHALL translate `ActionFlags` and optional mouse delta values into player velocity and facing changes using the player's `PhysicsConstants`. Forward/backward flags SHALL accelerate along the facing direction. Strafe flags SHALL accelerate perpendicular to the facing direction. Turn flags SHALL apply angular velocity through the acceleration/deceleration system. When proportional mouse yaw/pitch deltas are present, they SHALL be applied directly to facing and vertical look angles (bypassing the angular velocity system). Velocity SHALL be clamped to the physics-defined maximum values.

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
