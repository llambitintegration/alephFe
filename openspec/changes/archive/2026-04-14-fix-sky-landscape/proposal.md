## Why

Sky/landscape rendering is broken in marathon-web: ceiling surfaces always use the floor's transfer mode (so sky ceilings render as tiled textures instead of landscapes), and the landscape shader computes a single UV from camera angles rather than per-fragment direction, producing a flat image instead of a sky dome effect.

## What Changes

- Add `ceiling_transfer_mode` and `ceiling_light` fields to `PolygonInfo` in mesh.rs so ceiling surfaces use their own transfer mode instead of the floor's
- Fix wall vertices to use side-specific transfer modes instead of `info.floor_transfer_mode`
- Add `camera_position` (vec3) to `CameraUniform` in render.rs, shader.wgsl, and sprite_shader.wgsl
- Fix the TRANSFER_LANDSCAPE case in shader.wgsl to compute per-fragment UV from `normalize(world_pos - camera_position)` using atan2/asin for azimuth/elevation mapping

## Capabilities

### New Capabilities

### Modified Capabilities
- `transfer-modes`: The landscape transfer mode requirement changes from camera-angle-based UV to per-fragment direction-based UV, and the uniform data requirement adds camera_position
- `mesh-generation`: Ceiling and wall vertices must carry their own transfer mode and light values instead of reusing the floor's

## Impact

- `marathon-web/src/mesh.rs` - PolygonInfo struct, build_ceiling(), wall vertex construction
- `marathon-web/src/render.rs` - CameraUniform struct, camera position upload
- `marathon-web/src/shader.wgsl` - CameraUniform struct, TRANSFER_LANDSCAPE case
- `marathon-web/src/sprite_shader.wgsl` - CameraUniform struct (must stay in sync)
