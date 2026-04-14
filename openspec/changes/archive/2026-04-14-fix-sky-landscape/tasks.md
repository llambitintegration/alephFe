## 1. PolygonInfo and mesh data plumbing

- [x] 1.1 Add `ceiling_transfer_mode: u32` and `ceiling_light: f32` to `PolygonInfo` in `marathon-web/src/mesh.rs`
- [x] 1.2 Update `build_ceiling()` to use `info.ceiling_transfer_mode` and `info.ceiling_light` for ceiling vertices instead of `info.floor_transfer_mode` and `info.floor_light`
- [x] 1.3 Update wall vertex construction to use the side's `primary_transfer_mode` (and secondary/transparent as appropriate) instead of `info.floor_transfer_mode`
- [x] 1.4 Update all call sites that construct `PolygonInfo` to populate `ceiling_transfer_mode` and `ceiling_light` from map data
- [x] 1.5 Update mesh.rs unit tests to set `ceiling_transfer_mode` and `ceiling_light` fields

## 2. CameraUniform expansion

- [x] 2.1 Add `camera_position: [f32; 3]` and `_padding2: f32` to `CameraUniform` in `marathon-web/src/render.rs`, update the struct to write camera position when building the uniform buffer
- [x] 2.2 Update `CameraUniform` struct in `marathon-web/src/shader.wgsl` to add `camera_position: vec3<f32>` and `_padding2: f32`
- [x] 2.3 Update `CameraUniform` struct in `marathon-web/src/sprite_shader.wgsl` to add `camera_position: vec3<f32>` and `_padding2: f32`

## 3. Per-fragment landscape shader

- [x] 3.1 Fix the `TRANSFER_LANDSCAPE` case in `marathon-web/src/shader.wgsl` to compute per-fragment UV: `dir = normalize(world_pos - camera.camera_position)`, `u = atan2(dir.z, dir.x) / (2*PI)`, `v = 0.5 - asin(dir.y) / PI`

## 4. Verification

- [x] 4.1 Build marathon-web via Docker and verify no compile errors
- [x] 4.2 Visually confirm sky rendering on a level with landscape ceilings (e.g., Waterloo Waterpark)
