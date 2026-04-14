## MODIFIED Requirements

### Requirement: Composite HUD as 2D overlay on 3D scene
The system SHALL render the HUD as a wgpu render pass that writes to the same framebuffer as the 3D scene, composited on top. The HUD render pass SHALL execute after the 3D scene render pass completes. HUD elements SHALL support alpha transparency. The weapon overlay quad SHALL be positioned in NDC so that its bottom edge aligns with the top of the HUD panel, accounting for the HUD's pixel height as a fraction of the viewport height. Specifically, the weapon NDC bottom SHALL be `-1.0 + 2.0 * HUD_HEIGHT_PX / viewport_height` where `HUD_HEIGHT_PX` is the fixed HUD panel height (128 pixels).

#### Scenario: HUD over gameplay
- **WHEN** a frame is rendered during the Playing state
- **THEN** the 3D scene SHALL render first, followed by the HUD overlay pass which composites HUD elements on top

#### Scenario: Transparent HUD regions
- **WHEN** a HUD element has transparent pixels in its source sprite
- **THEN** the 3D scene SHALL be visible through those transparent regions

#### Scenario: Weapon sprite sits flush above HUD panel
- **WHEN** the weapon overlay is rendered with a viewport height of 960 pixels and the HUD panel is 128 pixels tall
- **THEN** the weapon quad bottom in NDC SHALL be approximately -0.733 (i.e., -1.0 + 2.0 * 128.0 / 960.0) so the visible weapon sits at the top edge of the HUD

#### Scenario: Weapon position adapts to viewport height
- **WHEN** the viewport height changes (e.g., window resize from 960px to 1080px)
- **THEN** the weapon quad bottom SHALL recompute to `-1.0 + 2.0 * 128.0 / new_viewport_height`, keeping the weapon flush with the HUD at any resolution
