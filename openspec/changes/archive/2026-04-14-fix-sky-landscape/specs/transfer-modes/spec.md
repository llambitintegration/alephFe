## MODIFIED Requirements

### Requirement: Landscape transfer mode
The system SHALL render surfaces with landscape transfer mode by computing a per-fragment direction vector from the camera position to the fragment's world-space position. The U coordinate SHALL be derived from `atan2(dir.z, dir.x) / (2 * PI)` (azimuth). The V coordinate SHALL be derived from `0.5 - asin(dir.y) / PI` (elevation). This produces a sky dome effect where the texture wraps spherically around the viewer based on the direction to each fragment.

#### Scenario: Sky rendering
- **WHEN** a ceiling surface has landscape transfer mode
- **THEN** the texture maps spherically based on the direction from camera to each fragment, creating a sky dome that responds to camera rotation and varies across the surface

#### Scenario: Landscape varies across surface
- **WHEN** a large ceiling polygon has landscape transfer mode and the camera is positioned below it
- **THEN** fragments at different positions on the polygon receive different UV coordinates based on the direction vector from the camera to each fragment, rather than all fragments receiving the same UV

### Requirement: Transfer mode uniform data
The system SHALL pass transfer mode parameters to the fragment shader via per-vertex attributes and the camera uniform buffer. Each vertex SHALL include a transfer mode ID (u32). The camera uniform SHALL include camera_position (vec3) for per-fragment landscape UV computation, in addition to elapsed time (f32) and camera angles. The fragment shader SHALL branch on the transfer mode ID to apply the appropriate effect.

#### Scenario: Shader receives mode data
- **WHEN** the fragment shader processes a surface with slide transfer mode
- **THEN** it reads the transfer mode ID from the vertex attribute and elapsed time from the camera uniform and applies the sliding UV offset

#### Scenario: Landscape shader uses camera position
- **WHEN** the fragment shader processes a surface with landscape transfer mode
- **THEN** it reads camera_position from the camera uniform and the fragment's world position to compute the per-fragment direction vector for UV mapping
