## ADDED Requirements

### Requirement: Sprite billboard rendering
The system SHALL render entity sprites as camera-facing billboarded quads in the 3D scene. Each sprite quad SHALL be positioned at the entity's world position, scaled to the sprite's defined dimensions (from Shapes data), and oriented to always face the camera. Sprites SHALL be rendered in a separate render pass after level geometry, sharing the same depth buffer for correct occlusion.

#### Scenario: Monster visible through doorway
- **WHEN** a monster is positioned behind a wall with a doorway, and the player looks through the doorway
- **THEN** the monster sprite SHALL be visible through the opening and occluded by the wall geometry via depth testing

#### Scenario: Sprite behind player not rendered
- **WHEN** an entity is positioned behind the camera's view frustum
- **THEN** the entity's sprite quad SHALL be culled and not submitted to the GPU

#### Scenario: Sprite alpha blending
- **WHEN** a sprite contains transparent pixels (color index 0 in Marathon's palette)
- **THEN** the transparent pixels SHALL be discarded (alpha test) so level geometry behind is visible

### Requirement: Shapes data lookup for sprite frames
The system SHALL resolve entity sprite frames from Shapes data using the entity's collection index, sequence index, and frame index as reported by marathon-sim. For each frame, the system SHALL extract the bitmap data, x/y offsets (for sprite registration), and world-space dimensions. The system SHALL build a texture atlas or texture array from loaded sprite bitmaps.

#### Scenario: Item sprite lookup
- **WHEN** marathon-sim reports an item entity with collection 17, sequence 0, frame 0
- **THEN** the system SHALL look up collection 17, sequence 0, frame 0 in the Shapes data and render the corresponding bitmap

#### Scenario: Monster animation frame
- **WHEN** marathon-sim reports a monster in its walking animation at frame 3 of 8
- **THEN** the system SHALL display frame 3 of the walking sequence for that monster's collection

#### Scenario: Missing sprite data handled gracefully
- **WHEN** an entity references a collection/sequence/frame combination not present in Shapes data
- **THEN** the system SHALL skip rendering that entity and log a warning (not crash)

### Requirement: Multi-angle sprite selection for monsters
The system SHALL select the appropriate viewing-angle sprite for monsters based on the angle between the camera's position and the monster's facing direction. Marathon monsters have up to 8 rotational views (front, front-left, left, back-left, back, back-right, right, front-right). The system SHALL compute the relative angle and select the nearest view.

#### Scenario: Monster facing camera
- **WHEN** the player views a monster from directly in front (relative angle ~0°)
- **THEN** the system SHALL display the monster's front-facing sprite (view 0)

#### Scenario: Monster facing away
- **WHEN** the player views a monster from directly behind (relative angle ~180°)
- **THEN** the system SHALL display the monster's back-facing sprite (view 4)

#### Scenario: Monster with fewer than 8 views
- **WHEN** a monster's sequence has only 5 views instead of 8
- **THEN** the system SHALL select the nearest available view by quantizing the angle to the available view count

### Requirement: Projectile and effect rendering
The system SHALL render projectiles and visual effects (explosions, energy discharges) as sprites. Projectiles SHALL use their assigned collection/sequence from physics data. Effects SHALL use their assigned collection/sequence and advance through animation frames based on the effect's age.

#### Scenario: Projectile in flight
- **WHEN** a projectile entity is active with a position and sprite assignment
- **THEN** the system SHALL render the projectile's sprite at its world position as a billboarded quad

#### Scenario: Explosion effect
- **WHEN** an explosion effect entity is spawned at a position
- **THEN** the system SHALL render the explosion sprite, advancing through animation frames until the effect expires

### Requirement: Scenery object rendering
The system SHALL render scenery objects (decorative elements placed in levels) as sprites at their map-defined positions. Scenery objects are static (non-simulated) and their sprite data comes from the map's object placement data combined with Shapes collections.

#### Scenario: Scenery lamp in level
- **WHEN** the map defines a scenery object (lamp) at position (2048, 3072, 512)
- **THEN** the system SHALL render the lamp's sprite at that world position, visible from all angles
