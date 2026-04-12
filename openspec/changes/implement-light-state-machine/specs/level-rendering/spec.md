## MODIFIED Requirements

### Requirement: Basic lighting
The system SHALL apply Marathon light intensities to surfaces. Each polygon references light source indices for floor and ceiling. The system SHALL read live light intensity values from the simulation's Light entities each frame and update the per-vertex or per-polygon light values in the GPU buffer. Animated lights SHALL produce visible brightness changes in real time as their intensity varies tick by tick.

#### Scenario: Bright and dark areas
- **WHEN** polygon A has light intensity 1.0 and polygon B has light intensity 0.3
- **THEN** polygon A's surfaces appear at full brightness and polygon B's surfaces appear dimmed to 30%

#### Scenario: Animated light updates GPU buffer
- **WHEN** a light's intensity changes between ticks (e.g., a smooth cycling light)
- **THEN** the renderer SHALL update the light value for all vertices belonging to polygons that reference that light, and the brightness change SHALL be visible in the next rendered frame

#### Scenario: Floor and ceiling use correct light
- **WHEN** a polygon has different floor_lightsource_index and ceiling_lightsource_index values
- **THEN** floor surfaces and lower walls SHALL use the floor light intensity, and ceiling surfaces and upper walls SHALL use the ceiling light intensity

#### Scenario: Media surface lighting
- **WHEN** a polygon contains media with an animated light
- **THEN** the media surface geometry SHALL use the floor light intensity of its polygon for brightness, and the media height SHALL update each frame from the sim's live media current_height

### Requirement: Per-polygon storage buffer
The system SHALL maintain a GPU storage buffer with one entry per polygon containing: current floor height, current ceiling height, floor light intensity, ceiling light intensity, floor transfer mode ID, ceiling transfer mode ID, floor texture offset, ceiling texture offset, media height, and media transfer mode. This buffer SHALL be updated each frame for polygons whose light values or media heights have changed.

#### Scenario: Light value refresh in buffer
- **WHEN** a light's intensity changes between frames
- **THEN** the floor_light and ceiling_light fields in the polygon storage buffer SHALL be updated for all polygons referencing that light

#### Scenario: Media height refresh in buffer
- **WHEN** a media entity's current_height changes between frames
- **THEN** the media_height field in the polygon storage buffer SHALL be updated for the media's polygon
