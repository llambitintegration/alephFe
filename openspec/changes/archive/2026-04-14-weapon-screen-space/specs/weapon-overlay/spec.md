## ADDED Requirements

### Requirement: Weapon sprite renders as screen-space overlay
The weapon sprite SHALL be rendered as a 2D screen-space overlay on top of the 3D scene, with depth testing disabled, so that world geometry never occludes the weapon.

#### Scenario: Weapon visible in front of wall
- **WHEN** the player is facing a wall at close range with a weapon equipped
- **THEN** the weapon sprite is fully visible and not clipped by the wall geometry

#### Scenario: Weapon does not move with camera yaw
- **WHEN** the player rotates the camera left or right
- **THEN** the weapon sprite remains fixed at the center-bottom of the viewport without lateral sliding

### Requirement: Weapon is centered horizontally at viewport bottom
The weapon sprite quad SHALL be horizontally centered in the viewport and anchored at the bottom edge, matching the original Marathon engine's weapon positioning.

#### Scenario: Weapon position on screen
- **WHEN** a weapon sprite is active and rendering
- **THEN** the weapon quad is horizontally centered (NDC x = 0) with its bottom edge at or near the viewport bottom (NDC y = -1.0)

### Requirement: Weapon sized as viewport percentage
The weapon sprite SHALL be sized relative to the viewport dimensions (approximately 35% of viewport width), not based on world-space distance or FOV. Height SHALL preserve the sprite bitmap's aspect ratio.

#### Scenario: Weapon size consistent across FOV changes
- **WHEN** the viewport is resized or the FOV changes
- **THEN** the weapon sprite maintains approximately the same viewport-relative size

### Requirement: Weapon overlay uses existing sprite textures
The weapon overlay renderer SHALL reuse the sprite texture atlas bind groups already loaded by `SpriteRenderer`, avoiding duplicate texture uploads for weapon collections.

#### Scenario: Weapon texture from loaded collection
- **WHEN** a weapon's sprite collection has been loaded by `SpriteRenderer`
- **THEN** the weapon overlay renderer uses the same GPU bind group to sample the weapon texture

### Requirement: Weapon overlay renders after main scene
The weapon overlay draw call SHALL execute after the main geometry and entity sprite render passes within the same frame, ensuring the weapon is always drawn on top.

#### Scenario: Render ordering
- **WHEN** a frame is rendered with both world geometry and a weapon equipped
- **THEN** the weapon overlay draw call occurs after the level geometry pass and entity sprite pass
