## ADDED Requirements

### Requirement: Window and GPU initialization
The system SHALL create a window using winit and initialize a wgpu device, queue, and surface. The system SHALL request a device with default limits and select the best available backend (Vulkan, Metal, or DX12). The swap chain format SHALL be the surface's preferred format.

#### Scenario: Successful initialization
- **WHEN** the application starts with valid GPU drivers
- **THEN** a window appears and wgpu initializes successfully with a renderable surface

#### Scenario: Window resize
- **WHEN** the user resizes the window
- **THEN** the surface is reconfigured to the new dimensions and the depth buffer is recreated

### Requirement: Depth buffer
The system SHALL create and maintain a depth buffer (Depth32Float format) matching the surface dimensions. The depth buffer SHALL be used in the render pass to ensure correct front-to-back rendering without manual visibility sorting.

#### Scenario: Overlapping geometry
- **WHEN** two polygons overlap in screen space (e.g., a wall behind a wall)
- **THEN** the nearer surface occludes the farther one via depth testing

### Requirement: Render pipeline configuration
The system SHALL create a wgpu render pipeline with: vertex shader that reads positions and UVs, transforms by view-projection matrix, and passes polygon index to fragment stage; fragment shader that samples the texture array and applies transfer mode effects. Back-face culling SHALL be enabled. The depth test SHALL use Less comparison.

#### Scenario: Pipeline draws textured geometry
- **WHEN** a frame is rendered
- **THEN** the pipeline produces textured 3D geometry with correct depth ordering and back-face culling

### Requirement: Camera system
The system SHALL implement a free-fly camera with 6 degrees of freedom. WASD keys move the camera forward/backward/left/right relative to its facing direction. Mouse movement rotates yaw (horizontal) and pitch (vertical). The pitch SHALL be clamped to prevent flipping. The camera SHALL provide a view matrix and a perspective projection matrix with configurable FOV (default 90 degrees), near plane (0.1), and far plane (1000.0).

#### Scenario: Move forward
- **WHEN** the user holds W
- **THEN** the camera translates forward along its look direction at a constant speed

#### Scenario: Mouse look
- **WHEN** the user moves the mouse right
- **THEN** the camera yaw rotates right, updating the view direction

#### Scenario: Pitch clamp
- **WHEN** the user moves the mouse to look straight up
- **THEN** the pitch is clamped to slightly less than 90 degrees to prevent gimbal issues

### Requirement: Frame loop
The system SHALL run a frame loop that: polls winit events, updates camera from input, updates platform/media animation state, writes per-polygon uniform data to the GPU, and submits render commands. The loop SHALL use winit's event loop with `ControlFlow::Poll` for continuous rendering.

#### Scenario: Steady frame rendering
- **WHEN** the application is running with a loaded level
- **THEN** frames are rendered continuously with updated camera and animation state

### Requirement: Level loading from scenario files
The system SHALL accept command-line arguments specifying the map WAD file path and shapes WAD file path. On startup, the system SHALL load both files and present a list of available levels (from WAD entries). The user SHALL be able to select a level to render.

#### Scenario: Load scenario from command line
- **WHEN** the user runs `marathon-viewer --map path/to/Map.sceA --shapes path/to/Shapes.shpA`
- **THEN** the system loads both files and lists available levels by name

#### Scenario: Level selection
- **WHEN** multiple levels are available
- **THEN** the system loads the first level by default and provides a key binding to cycle through levels

### Requirement: Level switching
The system SHALL support switching between levels at runtime. When switching, the system SHALL: unload current GPU resources (vertex buffers, texture arrays, bind groups), parse the new level's map data, rebuild meshes and textures, and reset the camera to the level's starting position (first polygon's center).

#### Scenario: Switch to next level
- **WHEN** the user presses the level-switch key
- **THEN** the current level's resources are freed, the next level is loaded, and rendering continues

### Requirement: Per-polygon storage buffer
The system SHALL maintain a GPU storage buffer with one entry per polygon containing: current floor height, current ceiling height, floor light intensity, ceiling light intensity, floor transfer mode ID, ceiling transfer mode ID, floor texture offset, ceiling texture offset, media height, and media transfer mode. This buffer SHALL be updated each frame for animated polygons and bound to the render pipeline.

#### Scenario: Shader reads polygon data
- **WHEN** the vertex/fragment shader processes a vertex with polygon_index=42
- **THEN** it reads entry 42 from the storage buffer to get current heights, light, and transfer mode data

### Requirement: Basic lighting
The system SHALL apply Marathon light intensities to surfaces. Each polygon references light source indices for floor and ceiling. The system SHALL evaluate light intensity (from StaticLightData or OldLightData) and pass it to the fragment shader as a multiplier on the texture color. Animated lights SHALL be evaluated on the CPU each frame.

#### Scenario: Bright and dark areas
- **WHEN** polygon A has light intensity 1.0 and polygon B has light intensity 0.3
- **THEN** polygon A's surfaces appear at full brightness and polygon B's surfaces appear dimmed to 30%

#### Scenario: Animated light
- **WHEN** a light has a pulsating function
- **THEN** the intensity value in the storage buffer changes each frame according to the light function, causing visible brightness changes
