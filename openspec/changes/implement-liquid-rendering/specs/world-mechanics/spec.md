## MODIFIED Requirements

### Requirement: Media simulation
The system SHALL simulate liquid media (water, lava, goo, sewage, jjaro) in polygons that reference a `MediaData` entry. Media height SHALL be derived from the light intensity of the media's associated light: as the light intensity varies, the liquid surface rises and falls between the media's defined low and high heights. Media SHALL apply current flow forces to entities standing in the liquid. The system SHALL expose the current media height, media type, and flow vector for each media-containing polygon to the rendering layer via the per-polygon storage buffer update path.

#### Scenario: Rising water
- **WHEN** the media's associated light intensity increases
- **THEN** the water surface height SHALL rise proportionally between the low and high bounds, and the updated height SHALL be written to the per-polygon storage buffer for GPU readback

#### Scenario: Lava damage
- **WHEN** the player is submerged in lava media
- **THEN** the player SHALL take environmental damage each tick based on the lava damage definition

#### Scenario: Media current pushes entity
- **WHEN** a player or monster stands in media with a defined current direction and magnitude
- **THEN** an external velocity SHALL be applied to the entity in the current direction

### Requirement: Camera submersion query
The system SHALL provide a query to determine whether the camera (player eye position) is submerged in media. The query SHALL accept the camera's world position and the polygon index the camera is in, and SHALL return a submersion result containing: a boolean indicating whether the camera is submerged, and the media type if submerged. This query SHALL be used by the render layer each frame to determine whether to draw the underwater tint overlay.

#### Scenario: Player wading in water
- **WHEN** the player is in a polygon with water media at height 1.5, and the player's eye height is 0.66
- **THEN** the submersion query SHALL return submerged=true, media_type=water

#### Scenario: Player above water surface
- **WHEN** the player is in a polygon with water media at height 0.3, and the player's eye height is 0.66
- **THEN** the submersion query SHALL return submerged=false

#### Scenario: Player in dry polygon
- **WHEN** the player is in a polygon with no media (media_index=-1)
- **THEN** the submersion query SHALL return submerged=false

### Requirement: Media detonation event
The system SHALL detect when a projectile crosses a media surface boundary during movement and emit a `SimEvent::MediaDetonation` event. The event SHALL include the world position of the intersection, the media type, and the detonation effect size. The detection SHALL compare the projectile's previous and current Z positions against the containing polygon's current media_height each tick.

#### Scenario: Projectile impacts water
- **WHEN** a projectile moves from above to below the water surface in one tick
- **THEN** the system SHALL emit a MediaDetonation event with the intersection position at media_height and the appropriate splash effect size

#### Scenario: Projectile emerges from liquid
- **WHEN** a projectile moves from below to above a media surface
- **THEN** the system SHALL emit a MediaDetonation event with the large emergence effect variant
