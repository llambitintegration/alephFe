## ADDED Requirements

### Requirement: Media surface alpha-blended draw pass
The system SHALL render media (liquid) surfaces in a dedicated draw sub-pass after all opaque geometry (walls, floors, ceilings). The media draw sub-pass SHALL enable alpha blending (source alpha, one-minus-source-alpha) and disable depth writes while keeping depth testing enabled (Less comparison). This ensures media surfaces are occluded by opaque geometry in front of the camera but do not prevent floors beneath transparent liquid from rendering.

#### Scenario: Transparent water over visible floor
- **WHEN** a polygon has a water media surface at height 0.5 and a floor at height 0.0
- **THEN** the floor texture SHALL be visible through the semi-transparent water surface

#### Scenario: Media occluded by wall
- **WHEN** a wall is between the camera and a media surface
- **THEN** the media surface SHALL NOT be visible (depth test against the wall passes)

#### Scenario: Media does not clip sprites behind it
- **WHEN** a sprite entity is behind a transparent media surface from the camera's perspective
- **THEN** the sprite SHALL be visible through the media surface because media does not write to the depth buffer

### Requirement: Per-type media visual properties
The system SHALL apply visual properties based on media type (water=0, lava=1, goo=2, sewage=3, jjaro=4). Each type SHALL have a base alpha transparency, a tint color multiplied with the texture sample, and an emissive flag. Water SHALL be semi-transparent blue (alpha 0.55, tint (0.1, 0.3, 0.8)). Lava SHALL be near-opaque orange with emissive glow (alpha 0.90, tint (1.0, 0.4, 0.1)). Goo SHALL be semi-transparent green (alpha 0.65, tint (0.2, 0.7, 0.1)). Sewage SHALL be murky brown (alpha 0.75, tint (0.5, 0.4, 0.2)). Jjaro SHALL be translucent purple (alpha 0.60, tint (0.4, 0.2, 0.8)).

#### Scenario: Water surface appearance
- **WHEN** a media surface has media_type=0 (water)
- **THEN** the fragment shader SHALL output the texture color multiplied by the blue tint with alpha=0.55

#### Scenario: Lava surface appearance
- **WHEN** a media surface has media_type=1 (lava)
- **THEN** the fragment shader SHALL output the texture color multiplied by the orange tint with alpha=0.90, and the emissive flag SHALL bypass light intensity dimming

#### Scenario: Unknown media type fallback
- **WHEN** a media surface has an unrecognized media_type value
- **THEN** the system SHALL use water visual properties as a fallback

### Requirement: Media vertex identification via texture descriptor bit flag
The system SHALL flag media surface vertices by setting bit 31 of the `texture_descriptor` field in the Vertex struct. The vertex shader SHALL test bit 31 to identify media vertices and mask it off (bitwise AND with 0x7FFFFFFF) before using the descriptor for texture lookup. This avoids adding a new vertex attribute.

#### Scenario: Media vertex flagged
- **WHEN** `build_media_surface()` emits a vertex with texture descriptor 0x0013
- **THEN** the vertex's `texture_descriptor` field SHALL be 0x80000013 (bit 31 set)

#### Scenario: Shader reads media flag
- **WHEN** the vertex shader processes a vertex with texture_descriptor bit 31 set
- **THEN** it SHALL treat the vertex as a media surface vertex and mask the descriptor to 0x7FFFFFFF for texture sampling

#### Scenario: Non-media vertex unaffected
- **WHEN** the vertex shader processes a vertex with texture_descriptor bit 31 clear
- **THEN** it SHALL process the vertex normally as opaque geometry

### Requirement: Shader-driven media height animation
The system SHALL animate media surface vertex Y positions in the vertex shader by reading `media_height` from the per-polygon storage buffer. When the vertex shader detects a media vertex (bit 31 flag), it SHALL replace the baked `position.y` with the `media_height` value from the polygon's storage buffer entry. The CPU SHALL update `media_height` each tick using `compute_media_height()` driven by the linked light's current intensity.

#### Scenario: Rising water animation
- **WHEN** the media's linked light intensity increases from 0.0 to 0.5 over 30 ticks
- **THEN** the media_height in the storage buffer SHALL interpolate from the low bound to the midpoint, and the vertex shader SHALL position the surface at the updated height each frame

#### Scenario: Static media (constant light)
- **WHEN** a media's linked light has constant function at intensity 0.8
- **THEN** the media_height SHALL remain at `low + 0.8 * (high - low)` and the surface SHALL not move

### Requirement: Media surface UV flow scrolling
The system SHALL scroll media surface texture UV coordinates based on the media's `current_direction` and `current_magnitude`. The UV offset SHALL increase linearly with elapsed time: `uv_offset = normalize(current_direction) * current_magnitude * elapsed_time`. The `PolygonGpuData` SHALL include `media_current_dx` and `media_current_dy` fields representing the decomposed flow vector.

#### Scenario: Water flowing east
- **WHEN** a water media has current_direction pointing east and current_magnitude=0.5
- **THEN** the surface texture SHALL scroll rightward at a rate proportional to 0.5 * elapsed_time

#### Scenario: No current
- **WHEN** a media has current_magnitude=0.0
- **THEN** the surface texture UV SHALL not scroll (only wobble distortion applies)
