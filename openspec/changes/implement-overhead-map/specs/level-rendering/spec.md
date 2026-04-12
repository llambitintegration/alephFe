## MODIFIED Requirements

### Requirement: Frame loop
The frame loop SHALL conditionally invoke the overhead map render pass after the 3D scene render pass and HUD render pass when the overhead map is visible. The overhead map render pass SHALL execute only when `OverheadMapState.visible` is true. On desktop, this is a wgpu render pass with orthographic projection, alpha blending, and no depth testing. On web, this is a Canvas 2D draw call on the overlay canvas. The overhead map pass SHALL NOT interfere with the 3D scene or HUD rendering state.

#### Scenario: Overhead map renders after 3D and HUD
- **WHEN** the overhead map is visible and a frame is rendered
- **THEN** the render order SHALL be: 3D scene pass, HUD pass, overhead map pass

#### Scenario: Overhead map hidden skips render pass
- **WHEN** the overhead map is not visible and a frame is rendered
- **THEN** the overhead map render pass SHALL be skipped entirely, incurring no GPU cost

#### Scenario: Overhead map does not affect 3D scene state
- **WHEN** the overhead map render pass completes
- **THEN** the 3D scene's depth buffer, blend state, and pipeline bindings SHALL be unaffected for the next frame
