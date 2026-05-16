## MODIFIED Requirements

### Requirement: Composite HUD as 2D overlay on 3D scene
The system SHALL render the HUD as a wgpu render pass that writes to the same framebuffer as the 3D scene, composited on top. The HUD render pass SHALL execute after the 3D scene render pass completes. HUD elements SHALL support alpha transparency. The first-person weapon overlay SHALL be rendered as a screen-space sprite using `position_sprite_axis` positioning logic: the weapon's `vertical_position` and `horizontal_position` values (normalized 0.0-1.0+ range from the simulation) SHALL determine the sprite origin on screen, and the shape's `world_left`, `world_right`, `world_top`, `world_bottom` bounds SHALL determine how the sprite extends from that origin. The positioning mode SHALL be `_position_center`, where the position value maps to a screen coordinate (position * screen_dimension), and the world bounds offset the sprite edges from that origin. The weapon sprite SHALL be clipped to the viewport, so portions extending below the screen bottom are not visible.

#### Scenario: HUD over gameplay
- **WHEN** a frame is rendered during the Playing state
- **THEN** the 3D scene SHALL render first, followed by the HUD overlay pass which composites HUD elements on top

#### Scenario: Transparent HUD regions
- **WHEN** a HUD element has transparent pixels in its source sprite
- **THEN** the 3D scene SHALL be visible through those transparent regions

#### Scenario: Fist weapon at idle position
- **WHEN** the player has fists equipped at idle and the weapon's vertical_position is approximately 1.067 (FIXED_ONE + FIXED_ONE/15 normalized)
- **THEN** the weapon sprite origin SHALL be placed at ~107% of screen height from the top, so only the top portion of the sprite (the fist knuckles) is visible above the screen bottom edge

#### Scenario: Pistol weapon at idle position
- **WHEN** the player has the pistol equipped at idle with its defined idle_height
- **THEN** the weapon sprite SHALL be positioned according to that weapon's idle_height, with the correct proportion of the sprite visible

#### Scenario: Weapon horizontal centering
- **WHEN** the weapon's horizontal_position is 0.5 (FIXED_ONE_HALF normalized) and the shape's world_left and world_right are asymmetric
- **THEN** the weapon sprite SHALL be horizontally centered at 50% of screen width, with the left and right edges offset by the shape's world_left and world_right bounds scaled to screen dimensions

## ADDED Requirements

### Requirement: Weapon overlay uses simulation positioning data
The system SHALL read `vertical_position`, `horizontal_position`, and shape world bounds from the simulation's `WeaponRenderState` each frame. The weapon overlay renderer SHALL use these values to compute screen-space placement via `position_sprite_axis` math rather than using fixed NDC coordinates. The shape's `world_left`, `world_right`, `world_top`, `world_bottom` values (in world units, divided by WORLD_ONE=1024) SHALL be passed from the shape data alongside the bitmap index.

#### Scenario: Simulation provides weapon positioning
- **WHEN** the renderer requests the current weapon state from the simulation
- **THEN** the returned state SHALL include vertical_position, horizontal_position as normalized floats, and the renderer SHALL use the shape's world bounds from the shapes file to compute the screen rectangle

#### Scenario: No weapon equipped
- **WHEN** the simulation reports no weapon equipped
- **THEN** no weapon overlay SHALL be rendered
