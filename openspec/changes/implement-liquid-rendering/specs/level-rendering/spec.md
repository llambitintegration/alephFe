## MODIFIED Requirements

### Requirement: Render pipeline configuration
The system SHALL create two wgpu render pipelines for level geometry: an **opaque pipeline** and a **media pipeline**. The opaque pipeline SHALL have back-face culling, depth test (Less comparison), and depth write enabled. The media pipeline SHALL share the same shaders and bind group layouts but SHALL have alpha blending enabled (source: SrcAlpha, destination: OneMinusSrcAlpha), depth test enabled (Less comparison), depth write disabled, and back-face culling enabled. Both pipelines SHALL be created at initialization and reused each frame.

#### Scenario: Opaque pipeline draws solid geometry
- **WHEN** the opaque draw sub-pass executes
- **THEN** the opaque pipeline produces textured 3D geometry with correct depth ordering, depth writes, and back-face culling

#### Scenario: Media pipeline draws transparent geometry
- **WHEN** the media draw sub-pass executes
- **THEN** the media pipeline produces alpha-blended textured geometry that is depth-tested but does not write to the depth buffer

### Requirement: Frame loop
The system SHALL run a frame loop that: polls winit events, updates camera from input, updates platform/media animation state, writes per-polygon uniform data to the GPU, and submits render commands. The loop SHALL use winit's event loop with `ControlFlow::Poll` for continuous rendering. The render commands SHALL consist of three sub-passes within a single render pass: (1) opaque geometry draw, (2) media surface draw with alpha blending, (3) underwater tint quad if camera is submerged. Entity sprites SHALL render after the tint quad.

#### Scenario: Steady frame rendering with media
- **WHEN** the application is running with a loaded level containing media polygons
- **THEN** frames are rendered continuously with opaque geometry first, then transparent media surfaces, then underwater tint if applicable, then sprites

### Requirement: Per-polygon storage buffer
The system SHALL maintain a GPU storage buffer with one entry per polygon containing: current floor height, current ceiling height, floor light intensity, ceiling light intensity, floor transfer mode ID, ceiling transfer mode ID, floor texture offset, ceiling texture offset, media height, media transfer mode, media type (u32), media light intensity (f32), media current dx (f32), and media current dy (f32). This buffer SHALL be updated each frame for animated polygons and bound to the render pipeline.

#### Scenario: Shader reads polygon data with media fields
- **WHEN** the vertex/fragment shader processes a media vertex with polygon_index=42
- **THEN** it reads entry 42 from the storage buffer to get current media_height, media_type, media_light, media_current_dx, and media_current_dy

#### Scenario: Media height updated from light
- **WHEN** a polygon's media has light_index=7 and the light's current intensity changes from 0.3 to 0.6
- **THEN** the storage buffer's media_height for that polygon SHALL be updated to `low + 0.6 * (high - low)` before the next frame's draw call

### Requirement: Underwater tint sub-pass
The system SHALL render a fullscreen tint quad as a third sub-pass when the camera is submerged in media. This sub-pass SHALL use a dedicated tint pipeline with alpha blending enabled, depth test disabled, and depth write disabled. The tint color SHALL be passed via a uniform buffer updated each frame. The tint quad SHALL render after the media surface sub-pass and before entity sprite rendering.

#### Scenario: Underwater tint renders
- **WHEN** the camera is below the media surface height in its current polygon
- **THEN** a fullscreen blue-tinted quad (for water) SHALL be composited over the scene before sprites are drawn

#### Scenario: No tint when above media
- **WHEN** the camera is above all media surfaces
- **THEN** no tint quad SHALL be rendered and sprites SHALL draw immediately after the media sub-pass
