## MODIFIED Requirements

### Requirement: Player physics uses platform-updated geometry
The system SHALL use `MapGeometry` floor and ceiling heights that reflect platform updates from the current tick when computing player collision and grounding. When a platform moves the floor or ceiling of a polygon, the player physics system (running after platforms in the tick order) SHALL see the updated heights for wall collision, step climbing, and floor tracking.

#### Scenario: Player rides rising platform
- **WHEN** a platform is extending upward on the player's current polygon
- **THEN** the player's Z position SHALL track the rising floor height computed by the platform update earlier in the same tick

#### Scenario: Ceiling lowers onto player
- **WHEN** a platform lowers the ceiling of the player's polygon below player head height
- **THEN** the player physics system SHALL detect the reduced clearance using the platform-updated ceiling height

### Requirement: Media current forces apply to player
The system SHALL apply media current forces as an external velocity component to the player when the player is submerged in a media surface. The current direction and magnitude from the `Media` component SHALL be added to the player's world-space velocity before collision resolution. Media drag SHALL scale the player's input-driven velocity.

#### Scenario: Player pushed by water current
- **WHEN** the player stands in water media with `current_direction` = 0.0 (east) and `current_magnitude` = 0.1
- **THEN** the player's effective velocity SHALL include an additional eastward component of 0.1 world units per tick

#### Scenario: Media drag reduces player speed
- **WHEN** the player is submerged in lava (drag factor 0.3) and moving forward
- **THEN** the player's input-driven velocity SHALL be scaled by 0.3 before collision resolution

### Requirement: Media damage applies to submerged player
The system SHALL apply environmental damage to the player each tick when submerged in a damaging media type (lava, goo, jjaro). Damage SHALL be applied to shield first, then health, using the `apply_damage()` function. The system SHALL also deplete oxygen when submerged in any media type and apply drowning damage when oxygen reaches zero.

#### Scenario: Player takes lava damage
- **WHEN** the player's Z position is below the current lava surface height
- **THEN** the player SHALL take environmental damage each tick, reducing shield and then health

#### Scenario: Oxygen depletes in water
- **WHEN** the player is submerged in water media
- **THEN** the player's oxygen SHALL decrease by a fixed amount each tick

#### Scenario: Drowning damage when oxygen zero
- **WHEN** the player's oxygen reaches zero while submerged
- **THEN** the player SHALL take drowning damage each tick until surfacing

#### Scenario: Oxygen recharges above surface
- **WHEN** the player surfaces from any media
- **THEN** the player's oxygen SHALL increase toward the maximum each tick
