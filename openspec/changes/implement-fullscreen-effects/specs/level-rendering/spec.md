## MODIFIED Requirements

### Requirement: Frame loop
The system SHALL run a frame loop that: polls winit events, receives camera state from the sim-render bridge, updates per-polygon uniform data, submits level geometry render commands, submits entity sprite render commands, executes the fader post-process pass, and then submits HUD render commands. The 3D scene (level geometry and entity sprites) SHALL render to an intermediate color attachment when faders are active. The fader post-process pass SHALL read this intermediate texture and output to the swapchain surface. The HUD render pass SHALL execute after the fader pass, writing to the swapchain. When no faders are active, the system MAY render the 3D scene directly to the swapchain and skip the fader pass as an optimization.

#### Scenario: Frame with active fader
- **WHEN** a frame is rendered while a damage flash fader is active
- **THEN** the level geometry and sprites SHALL render to the intermediate texture, the fader pass SHALL blend the red tint onto the swapchain, and the HUD SHALL render on top without being affected by the fader

#### Scenario: Frame with no active faders
- **WHEN** a frame is rendered with no active faders
- **THEN** the system SHALL either blit the intermediate texture to the swapchain or render directly to the swapchain, followed by the HUD pass

#### Scenario: Steady frame rendering during gameplay
- **WHEN** the application is in the Playing state with a loaded level
- **THEN** frames SHALL be rendered continuously with camera state derived from the simulation

#### Scenario: Rendering while paused
- **WHEN** the game is paused
- **THEN** the frame loop SHALL continue rendering but the camera SHALL remain at its last simulation position

### Requirement: Intermediate render target for post-processing
The system SHALL allocate an intermediate color texture at the same resolution as the swapchain surface. This texture SHALL be used as the color attachment for the level geometry and entity sprite render passes when post-processing is needed. The texture SHALL be recreated when the surface is resized. The texture format SHALL match the swapchain surface format.

#### Scenario: Surface resize recreates intermediate texture
- **WHEN** the window is resized
- **THEN** the intermediate render target SHALL be recreated at the new resolution

#### Scenario: Intermediate texture used as shader input
- **WHEN** the fader post-process pass executes
- **THEN** the intermediate texture SHALL be bound as a texture input to the fader fragment shader
