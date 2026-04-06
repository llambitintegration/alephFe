## MODIFIED Requirements

### Requirement: Camera system
The system SHALL implement a first-person camera driven by the player's simulation state. The camera position SHALL be set to the player's world position plus an eye-height offset. The camera yaw SHALL match the player's facing angle. The camera pitch SHALL match the player's look angle. The camera SHALL provide a view matrix and a perspective projection matrix with FOV matching Marathon's original field of view (approximately 90 degrees horizontal equivalent), near plane 0.1, and far plane 1000.0. The camera SHALL support interpolation between tick states for smooth rendering above the simulation tick rate.

#### Scenario: Camera at player spawn
- **WHEN** a level is loaded and the player spawns at position (1024, 2048, 0) facing east
- **THEN** the camera SHALL be positioned at (1024, 2048, eye_height) looking east

#### Scenario: Camera follows simulation
- **WHEN** the simulation advances and the player moves to a new position
- **THEN** the camera SHALL update to the player's new position and facing after the tick

#### Scenario: Interpolated camera between ticks
- **WHEN** the render frame occurs between two simulation ticks
- **THEN** the camera position and facing SHALL be linearly interpolated between the previous and current tick states

#### Scenario: Pitch clamp
- **WHEN** the player looks up beyond Marathon's maximum look angle
- **THEN** the pitch SHALL be clamped to the engine's maximum vertical look range

### Requirement: Frame loop
The system SHALL run a frame loop that: polls winit events, receives camera state from the sim-render bridge (interpolated player position and facing), updates per-polygon uniform data (including platform and media state from the simulation), submits level geometry render commands, and then submits entity sprite render commands. The loop SHALL use winit's event loop with `ControlFlow::Poll` for continuous rendering. The frame loop SHALL NOT directly process input for camera control — all camera state comes from the simulation via the bridge.

#### Scenario: Steady frame rendering during gameplay
- **WHEN** the application is in the Playing state with a loaded level
- **THEN** frames SHALL be rendered continuously with camera state derived from the simulation

#### Scenario: Rendering while paused
- **WHEN** the game is paused
- **THEN** the frame loop SHALL continue rendering but the camera SHALL remain at its last simulation position

## ADDED Requirements

### Requirement: Entity sprite render pass
The system SHALL support a second render pass after level geometry for rendering entity sprites. This pass SHALL share the depth buffer from the level geometry pass (for correct occlusion) but SHALL use a separate pipeline configured for alpha-tested billboarded quads. The level rendering system SHALL expose an interface for the entity rendering system to submit sprite draw commands into this pass.

#### Scenario: Sprites occluded by walls
- **WHEN** entity sprites are rendered after level geometry
- **THEN** sprites behind walls SHALL be correctly occluded by the shared depth buffer

#### Scenario: Sprites in front of geometry
- **WHEN** a sprite is closer to the camera than the wall behind it
- **THEN** the sprite SHALL be visible and SHALL write to the depth buffer
