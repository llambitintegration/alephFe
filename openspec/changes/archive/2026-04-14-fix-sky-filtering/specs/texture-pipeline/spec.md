## MODIFIED Requirements

### Requirement: Texture bind group management
The system SHALL create wgpu bind groups for each loaded texture array so the fragment shader can sample textures. The bind group SHALL include the texture array view, a nearest-neighbor sampler configured with repeating address mode (for pixel-art wall/floor textures), and a linear-filtering sampler configured with repeating address mode (for smooth landscape/sky textures). The bind group layout SHALL define three entries: binding 0 for the texture array, binding 1 for the nearest sampler, and binding 2 for the linear sampler.

#### Scenario: Bind group for rendering
- **WHEN** the render pipeline needs to draw surfaces from collection 5
- **THEN** a bind group exists with collection 5's texture array, a nearest-neighbor repeating sampler at binding 1, and a linear-filtering repeating sampler at binding 2

#### Scenario: Fallback bind group includes both samplers
- **WHEN** a surface references a collection that failed to load
- **THEN** the fallback bind group also contains both the nearest and linear samplers at their respective bindings
