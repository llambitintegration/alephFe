## ADDED Requirements

### Requirement: WebGL2-compatible per-polygon dynamic data buffer

The `marathon-web` renderer SHALL maintain a GPU buffer with one entry per polygon containing at least: current floor height, current ceiling height, current media height, floor light intensity, and ceiling light intensity. The buffer SHALL be sized and bound using only features available under `wgpu::Limits::downlevel_webgl2_defaults` (uniform buffer, or a storage buffer only where the existing WebGL2 compatibility path supports it). The level vertex buffer SHALL remain immutable after load; per-polygon dynamic values SHALL NOT be baked into vertices.

#### Scenario: Buffer bound under WebGL2 limits

- **WHEN** `marathon-web` initializes on a WebGL2 adapter (including SwiftShader fallback)
- **THEN** device creation SHALL succeed with the per-polygon dynamic data buffer bound to the render pipeline, without requesting features outside `downlevel_webgl2_defaults`

#### Scenario: Vertices carry polygon index, not baked light

- **WHEN** the level mesh is built at load time
- **THEN** each vertex SHALL carry its polygon index and the per-polygon light/height SHALL be supplied via the dynamic buffer, not written into the vertex's `light` or absolute `position.y`

### Requirement: Per-frame synchronization with simulation state

The `marathon-web` frame loop SHALL, every rendered frame, read current platform floor/ceiling heights, media heights, and animated light intensities from `SimWorld` and write the corresponding per-polygon entries to the GPU dynamic data buffer before submitting render commands.

#### Scenario: Door becomes visible when opened

- **WHEN** the player activates a door platform and the sim raises its floor / lowers its barrier over subsequent ticks
- **THEN** the rendered geometry for that polygon SHALL move to match the sim height on each frame, making the door visibly open

#### Scenario: Light switch visibly changes brightness

- **WHEN** a control panel toggles a light's intensity in the sim
- **THEN** the affected polygon surfaces SHALL change brightness on screen on the next frame, without rebuilding the vertex buffer

### Requirement: Shader applies per-polygon height offset and lighting

The `marathon-web` vertex shader SHALL offset each vertex's vertical position using its polygon's current floor/ceiling/media height from the dynamic data buffer. The fragment shader SHALL multiply texture color by the polygon's current light intensity from the dynamic data buffer.

#### Scenario: Elevator platform animates via shader

- **WHEN** a polygon's floor height changes from 0 to 1024 over time in the dynamic data buffer
- **THEN** the rendered floor SHALL rise smoothly without any change to the vertex or index buffers

#### Scenario: Pulsating light renders dynamically

- **WHEN** an animated light's intensity oscillates each tick in the dynamic data buffer
- **THEN** the lit surfaces SHALL visibly brighten and dim in step with the buffer values
