## Context

Marathon-web renders levels using a single vertex buffer with per-vertex transfer mode and light values. The `PolygonInfo` struct currently only stores `floor_light` and `floor_transfer_mode`, so ceiling and wall surfaces incorrectly reuse the floor's values. The landscape shader (transfer_mode=9) computes UV from camera yaw/pitch uniforms, giving every fragment the same UV -- producing a flat colored rectangle instead of a sky dome.

## Goals / Non-Goals

**Goals:**
- Ceiling surfaces use their own transfer mode and light values from the map data
- Wall surfaces use the side-specific transfer mode from their side data
- Landscape transfer mode renders a proper sky dome using per-fragment direction vectors
- CameraUniform provides camera_position for the per-fragment calculation

**Non-Goals:**
- Fixing landscape rendering in marathon-viewer or marathon-game (same bug exists there, separate change)
- Adding landscape_mode variants (horizontal vs vertical) -- only standard landscape for now
- Animated landscape scrolling or offsets

## Decisions

**1. Add ceiling fields to PolygonInfo rather than a separate struct**

PolygonInfo gains `ceiling_transfer_mode: u32` and `ceiling_light: f32`. This keeps the mesh builder simple -- one info struct per polygon, two sets of surface fields. Alternative: separate FloorInfo/CeilingInfo structs. Rejected because it would complicate the call sites for minimal benefit.

**2. Per-fragment landscape UV via camera_position uniform**

The shader needs `world_pos` (already available from vertex output) and `camera_position` (new uniform). The direction vector `normalize(world_pos - camera_position)` gives azimuth via `atan2(dir.z, dir.x)` and elevation via `asin(dir.y)`. Alternative: pass direction as a varying from vertex shader. Rejected because interpolating direction vectors across large polygons would produce incorrect results.

**3. camera_position replaces _padding in CameraUniform**

CameraUniform currently has `[view_proj(16), camera_yaw(1), camera_pitch(1), elapsed_time(1), _padding(1)]` = 20 floats = 80 bytes. We replace `_padding` with nothing and append `camera_position(3) + _padding2(1)` = 24 floats = 96 bytes. This maintains 16-byte alignment. Both shader.wgsl and sprite_shader.wgsl CameraUniform structs must be updated to match.

**4. Wall transfer mode from side data**

Wall vertices currently use `info.floor_transfer_mode`. Instead, the wall-building functions will read `side.primary_transfer_mode` (or secondary/transparent as appropriate) from the map's side data. This is a straightforward data-plumbing fix.

## Risks / Trade-offs

- [Uniform size increase] The CameraUniform grows from 80 to 96 bytes. Negligible GPU impact. No mitigation needed.
- [WGSL struct alignment] vec3 in WGSL has 16-byte alignment. We use `camera_position: vec3<f32>` followed by a padding float to stay aligned. Verified by keeping total struct size a multiple of 16 bytes.
- [Side transfer mode availability] Some sides may not have transfer mode data in the map. Mitigation: default to 0 (normal) when side data is missing, which matches current behavior.
