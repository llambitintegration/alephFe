## MODIFIED Requirements

### Requirement: Composite HUD as 2D overlay on 3D scene
The HUD render pass and overhead map render pass SHALL coexist without conflict. Both render as 2D overlays on the 3D scene. The HUD SHALL render before the overhead map in the pass ordering. When the overhead map is visible, the HUD elements SHALL remain accessible beneath the semi-transparent map overlay. On web, the HUD uses HTML/CSS DOM elements and the overhead map uses a separate Canvas 2D element, so they occupy independent z-index layers. On desktop, both are wgpu render passes writing to the same swapchain surface with alpha blending, executing in sequence.

#### Scenario: HUD visible under overhead map
- **WHEN** the overhead map overlay is visible
- **THEN** the HUD elements (health bar, shield bar, oxygen meter) SHALL still be rendered, visible beneath the semi-transparent map overlay

#### Scenario: HUD and map do not conflict on web
- **WHEN** both the HUD and overhead map are visible in the web build
- **THEN** the HUD DOM elements and the automap canvas SHALL be on separate z-index layers with the automap canvas above the HUD but below pointer-events

#### Scenario: HUD and map do not conflict on desktop
- **WHEN** both the HUD and overhead map are visible in the desktop build
- **THEN** the HUD render pass SHALL complete before the overhead map render pass begins, and both SHALL use alpha blending without interfering with each other's output
