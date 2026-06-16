## ADDED Requirements

### Requirement: Web consumes a single render snapshot per frame

The `marathon-web` frame loop SHALL obtain its per-frame render state from one `render_snapshot()` call per rendered frame, instead of the previous set of scattered accessor calls, and SHALL produce visually identical output for a static scene.

#### Scenario: Static scene is pixel-equivalent after migration

- **WHEN** a level with no animating geometry or lights is rendered before and after the web migration to `render_snapshot()`
- **THEN** the rendered frames SHALL be pixel-equivalent within the existing visual-regression tolerance

### Requirement: Web wall heights are driven by the per-polygon data texture

The `marathon-web` renderer SHALL drive wall quad top/bottom Y from the per-polygon data texture using a height-source discriminator, mirroring the existing floor/ceiling/media discriminator mechanism, instead of baking absolute wall Y into the static vertex buffer. The level vertex and index buffers SHALL remain immutable after load and SHALL NOT be recreated per frame.

#### Scenario: Wall stretches when an adjacent platform moves

- **WHEN** a platform/door polygon changes floor or ceiling height over successive ticks
- **THEN** the side wall quads bordering that polygon SHALL stretch to follow the new heights, leaving no gap, on each frame

#### Scenario: Wall heights resolve from the source polygon

- **WHEN** the vertex shader processes a wall vertex tagged with a height-source discriminator and a source polygon index
- **THEN** the vertex's Y SHALL be resolved from that source polygon's current floor/ceiling height in the data texture, not from a baked `position.y`

#### Scenario: Vertex and index buffers are never rebuilt

- **WHEN** wall geometry animates across many frames
- **THEN** only the per-polygon data texture SHALL be updated; the vertex and index buffer handles SHALL be unchanged across frames
