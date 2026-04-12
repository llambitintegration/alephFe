## MODIFIED Requirements

### Requirement: Apply movement from action flags
SimWorld::tick() SHALL call the player physics system each tick, translating the stored ActionFlags into position and facing updates on the Player entity. The player physics system SHALL read ActionFlags from the TickInput resource, compute velocity from PhysicsConstants (acceleration, deceleration, max speed), apply wall collision, step climbing, gravity, and update the Player's Position, Velocity, Facing, and VerticalLook components.

#### Scenario: Player moves forward when W is pressed
- **WHEN** ActionFlags contains MOVE_FORWARD and tick() is called
- **THEN** the Player entity's Position SHALL change in the direction of the Player's Facing angle by an amount determined by PhysicsConstants acceleration and max speed

#### Scenario: Player turns when mouse moves horizontally
- **WHEN** ActionFlags contains LOOK_RIGHT or LOOK_LEFT and tick() is called
- **THEN** the Player entity's Facing angle SHALL change by the turn rate defined in PhysicsConstants

#### Scenario: Player looks up/down when mouse moves vertically
- **WHEN** ActionFlags contains LOOK_UP or LOOK_DOWN and tick() is called
- **THEN** the Player entity's VerticalLook SHALL change, clamped to the allowed pitch range

#### Scenario: No movement when no input flags
- **WHEN** ActionFlags is empty and tick() is called
- **THEN** the Player entity's Position SHALL only change due to gravity/deceleration, not voluntary movement
