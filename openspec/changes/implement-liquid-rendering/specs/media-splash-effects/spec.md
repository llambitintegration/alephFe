## ADDED Requirements

### Requirement: Projectile-media surface crossing detection
The system SHALL detect when a projectile crosses a media surface boundary during its movement step. For each projectile that moves within a polygon containing media, the system SHALL compare the projectile's previous Z position and current Z position against the polygon's current media_height. If the positions straddle the media surface (one above, one below), a crossing event has occurred.

#### Scenario: Projectile enters water
- **WHEN** a projectile moves from Z=1.5 to Z=0.3 in a polygon with media_height=0.8
- **THEN** the system SHALL detect a downward media crossing at the intersection point

#### Scenario: Projectile exits lava
- **WHEN** a projectile moves from Z=0.2 to Z=1.0 in a polygon with media_height=0.5
- **THEN** the system SHALL detect an upward media crossing (emergence) at the intersection point

#### Scenario: Projectile moves above media
- **WHEN** a projectile moves from Z=2.0 to Z=1.5 in a polygon with media_height=0.5
- **THEN** no crossing event SHALL be detected

### Requirement: Media detonation event emission
The system SHALL emit a `SimEvent::MediaDetonation` event when a projectile-media crossing is detected. The event SHALL include: the world position of the intersection point (interpolated along the projectile's movement vector at the media height), the media type, and the detonation effect size (small, medium, or large based on the projectile's `media_detonation_effect` field). For upward crossings (emergence), the system SHALL use the large emergence effect variant.

#### Scenario: Bullet hits water
- **WHEN** a small projectile (e.g., pistol round) crosses a water surface downward
- **THEN** the system SHALL emit a MediaDetonation event with the small splash effect at the intersection XZ position and Y=media_height

#### Scenario: Rocket enters lava
- **WHEN** a large projectile (e.g., rocket) crosses a lava surface downward
- **THEN** the system SHALL emit a MediaDetonation event with the large splash effect

### Requirement: Splash sprite rendering
The render layer SHALL receive `SimEvent::MediaDetonation` events and spawn short-lived billboard sprites via the existing `SpriteRenderer`. Each splash sprite SHALL be positioned at the event's world position, use the detonation effect's shape descriptor from the shapes file, and persist for 8 ticks (approximately 0.27 seconds). The sprite SHALL be added to the sprite draw call batch alongside entity sprites.

#### Scenario: Splash sprite appears
- **WHEN** a MediaDetonation event is received with position (5.0, 0.8, 3.0) and the small water splash shape
- **THEN** a billboard sprite SHALL appear at (5.0, 0.8, 3.0) using the water splash texture for 8 ticks

#### Scenario: Splash sprite expires
- **WHEN** a splash sprite has been alive for 8 ticks
- **THEN** the sprite SHALL be removed from the draw call batch and no longer rendered
