## ADDED Requirements

### Requirement: Camera submersion detection
The system SHALL determine each frame whether the camera is submerged in media. The check SHALL identify which polygon the camera is currently in, look up that polygon's media index, and compare the camera's Y coordinate against the polygon's current `media_height`. If the camera Y is below `media_height` and the polygon has a valid media index, the camera SHALL be considered submerged. The system SHALL also track which media type the camera is submerged in.

#### Scenario: Camera below water surface
- **WHEN** the camera is in a polygon with media_index=3 (water) and media_height=1.0, and the camera Y is 0.66
- **THEN** the system SHALL report the camera as submerged in water

#### Scenario: Camera above media surface
- **WHEN** the camera is in a polygon with media_index=3 and media_height=0.5, and the camera Y is 0.66
- **THEN** the system SHALL report the camera as NOT submerged

#### Scenario: Camera in polygon with no media
- **WHEN** the camera is in a polygon with media_index=-1
- **THEN** the system SHALL report the camera as NOT submerged

### Requirement: Fullscreen underwater tint overlay
The system SHALL render a fullscreen colored quad when the camera is submerged in media. The quad SHALL be drawn after the media surface sub-pass and before entity sprites. The quad SHALL use alpha blending with no depth test and no depth write. The tint color and alpha SHALL vary by media type: water (0.1, 0.2, 0.6, 0.30), lava (0.6, 0.1, 0.0, 0.40), goo (0.1, 0.5, 0.1, 0.35), sewage (0.3, 0.4, 0.1, 0.30), jjaro (0.3, 0.1, 0.5, 0.30).

#### Scenario: Submerged in water
- **WHEN** the camera is submerged in water media
- **THEN** the system SHALL render a fullscreen quad with color (0.1, 0.2, 0.6) at alpha 0.30, tinting the entire view blue

#### Scenario: Submerged in lava
- **WHEN** the camera is submerged in lava media
- **THEN** the system SHALL render a fullscreen quad with color (0.6, 0.1, 0.0) at alpha 0.40, giving a strong red-orange tint

#### Scenario: Not submerged
- **WHEN** the camera is not submerged in any media
- **THEN** the system SHALL NOT render the underwater tint quad

### Requirement: Underwater tint pipeline
The system SHALL create a separate wgpu render pipeline for the underwater tint quad. This pipeline SHALL use a minimal vertex shader that outputs a fullscreen triangle (3 vertices, no vertex buffer), and a fragment shader that outputs a constant color from a uniform buffer. The pipeline SHALL have alpha blending enabled, depth test disabled, and depth write disabled. The tint color uniform SHALL be updated each frame based on the submersion state.

#### Scenario: Tint pipeline state
- **WHEN** the underwater tint quad is rendered
- **THEN** the pipeline SHALL blend the tint color over the existing framebuffer contents using source-alpha blending

#### Scenario: Tint pipeline does not affect depth
- **WHEN** the underwater tint quad is rendered
- **THEN** the depth buffer SHALL NOT be modified, so subsequent sprite rendering uses the correct depth values from opaque geometry
