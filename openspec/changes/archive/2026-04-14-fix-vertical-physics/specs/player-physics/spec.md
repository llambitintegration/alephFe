## MODIFIED Requirements

### Requirement: Apply gravity
The system SHALL apply gravitational acceleration to the player when airborne (not standing on a floor). The player's vertical velocity SHALL increase downward by `gravitational_acceleration` per tick, up to `terminal_velocity`. When the player contacts a floor, vertical velocity SHALL be zeroed. When a grounded player crosses into a polygon with a lower floor height, the player's Z position SHALL be set to that polygon's floor height (snap down). When an airborne player crosses into a polygon with a lower floor, gravity SHALL continue pulling them down naturally without snapping.

#### Scenario: Player falls off ledge
- **WHEN** the player walks past a polygon edge where the adjacent polygon's floor is lower
- **THEN** the player SHALL accelerate downward at `gravitational_acceleration` per tick

#### Scenario: Landing on floor
- **WHEN** the player's vertical position reaches the floor height of the current polygon
- **THEN** vertical velocity SHALL be set to zero and the player SHALL be grounded

#### Scenario: Grounded player crosses to lower polygon
- **WHEN** a grounded player crosses from a polygon with floor height 2.0 to an adjacent polygon with floor height 1.0
- **THEN** the player's Z position SHALL be set to 1.0 (the lower floor height) and the player SHALL remain grounded

#### Scenario: Airborne player crosses to lower polygon
- **WHEN** an airborne player (already falling) crosses from a polygon with floor height 2.0 to an adjacent polygon with floor height 0.0
- **THEN** the player's Z position SHALL NOT be snapped and gravity SHALL continue to apply normally

### Requirement: Ceiling collision
The system SHALL prevent the player from moving into spaces where the ceiling-to-floor gap is less than the player's `height`. If jumping or riding a platform would push the player into the ceiling, vertical velocity SHALL be zeroed and position clamped. After grounding and step-up logic, the system SHALL clamp the player's Z position to `ceiling_height - player_height` for the current polygon, ensuring the player's head never penetrates the ceiling.

#### Scenario: Low ceiling blocks entry
- **WHEN** the player tries to cross into a polygon where ceiling - floor < player height
- **THEN** the player SHALL be blocked as if by a solid wall

#### Scenario: Upward movement clamped by ceiling
- **WHEN** the player's Z position after movement would place their head above the current polygon's ceiling height
- **THEN** the player's Z position SHALL be clamped to `ceiling_height - player_height` and vertical velocity SHALL be set to zero

## ADDED Requirements

### Requirement: Validate player spawn Z
The system SHALL validate the player's initial Z position against the polygon's floor height when spawning from map data. If the raw map Z coordinate is below the polygon's floor height, the spawn Z SHALL be set to the floor height. This ensures the player always starts on or above the floor surface.

#### Scenario: Spawn Z below floor
- **WHEN** a player is spawned from map data with Z = 0.0 in a polygon whose floor height is 0.5
- **THEN** the player's initial Z position SHALL be set to 0.5

#### Scenario: Spawn Z above floor
- **WHEN** a player is spawned from map data with Z = 1.0 in a polygon whose floor height is 0.5
- **THEN** the player's initial Z position SHALL remain 1.0 (gravity will handle descent if needed)
