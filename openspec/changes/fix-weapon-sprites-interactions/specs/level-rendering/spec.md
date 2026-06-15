## ADDED Requirements

### Requirement: Asymmetric sprite billboard anchoring
The system SHALL render world entity sprites (items, scenery, monsters, projectiles) as camera-facing billboard quads using the shape's asymmetric world bounds (`world_left`, `world_right`, `world_top`, `world_bottom`) rather than symmetric centering. The entity's world position SHALL serve as the sprite origin point. The billboard's left edge SHALL be offset by `world_left` (negative = left of origin) and right edge by `world_right` (positive = right of origin) along the camera-right vector. The billboard's top SHALL be offset by `world_top` (negative = above origin) and bottom by `world_bottom` (positive = below origin) along the world-up vector. All offsets SHALL be in world units (the raw shape values divided by WORLD_ONE = 1024).

#### Scenario: Symmetric sprite renders centered
- **WHEN** a sprite has world_left=-200, world_right=+200, world_top=-300, world_bottom=+100 (raw values)
- **THEN** the billboard SHALL extend 200/1024 WU left and right of the entity position, 300/1024 WU above and 100/1024 WU below the entity position

#### Scenario: Asymmetric sprite renders offset
- **WHEN** a sprite has world_left=-300, world_right=+100, world_top=-400, world_bottom=+50 (raw values)
- **THEN** the billboard left edge SHALL be 300/1024 WU left of the entity position and the right edge 100/1024 WU right, with the sprite visually shifted left relative to symmetric centering

#### Scenario: Sprite anchored above ground
- **WHEN** a sprite has world_top=-500, world_bottom=-50 (both negative, meaning both edges are above the origin)
- **THEN** the entire billboard SHALL render above the entity's world position, with the bottom edge 50/1024 WU above the origin

#### Scenario: SpriteDrawCall carries world bounds
- **WHEN** a sprite draw call is constructed from shape data
- **THEN** the draw call SHALL contain `world_left`, `world_right`, `world_top`, `world_bottom` as separate f32 fields (in world units) instead of a single symmetric `width` and `height`
