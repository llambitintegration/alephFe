## MODIFIED Requirements

### Requirement: Composite HUD as 2D overlay on 3D scene
The system SHALL render the HUD as a wgpu render pass that writes to the swapchain surface, composited on top of the post-processed scene (after fader effects have been applied). The HUD render pass SHALL execute after both the 3D scene render pass and the fader post-process pass complete. HUD elements SHALL NOT be affected by active faders, matching Marathon 2/Infinity behavior where HUD is drawn on top of faded gameplay. HUD elements SHALL support alpha transparency.

#### Scenario: HUD over faded gameplay
- **WHEN** a frame is rendered with an active damage flash fader
- **THEN** the 3D scene SHALL render first, the fader pass SHALL apply the red tint, and the HUD overlay SHALL render on top without the red tint affecting HUD elements

#### Scenario: HUD over gameplay without faders
- **WHEN** a frame is rendered during the Playing state with no active faders
- **THEN** the 3D scene SHALL render first, followed by the HUD overlay pass which composites HUD elements on top

#### Scenario: Transparent HUD regions over faded scene
- **WHEN** a HUD element has transparent pixels and a fader is active
- **THEN** the faded 3D scene (with fader applied) SHALL be visible through those transparent regions
