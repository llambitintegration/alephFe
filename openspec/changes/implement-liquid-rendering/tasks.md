## 1. Mesh Builder: Separate Opaque and Media Index Ranges

- [ ] 1.1 Add `opaque_index_count: u32` field to `LevelMesh` struct in marathon-game/src/mesh.rs, marathon-viewer/src/mesh.rs, and marathon-web/src/mesh.rs
- [ ] 1.2 Refactor `build_level_mesh()` in all three crates to emit opaque geometry (floors, ceilings, walls) first, then media surfaces, recording the index count boundary as `opaque_index_count`
- [ ] 1.3 In `build_media_surface()`, set bit 31 on the `texture_descriptor` field of each media vertex (OR with 0x80000000) to flag it as a media surface vertex
- [ ] 1.4 Add unit test: given a level with 2 opaque polygons and 1 media polygon, verify `opaque_index_count` equals the opaque triangle index count and total indices include media triangles after that boundary
- [ ] 1.5 Add unit test: verify media vertices have bit 31 set on texture_descriptor, and opaque vertices do not

## 2. PolygonGpuData: Add Media Fields

- [ ] 2.1 Add `media_type: u32`, `media_light: f32`, `media_current_dx: f32`, `media_current_dy: f32` fields to `PolygonGpuData` in marathon-game/src/render.rs, marathon-viewer/src/render.rs, and marathon-web/src/render.rs
- [ ] 2.2 Populate the new fields during per-polygon storage buffer construction: media_type from `MediaData.media_type`, media_light from the media's linked light intensity, media_current_dx/dy decomposed from direction and magnitude
- [ ] 2.3 Update the shader's `PolygonData` struct in shader.wgsl to match the new layout (add media_type, media_light, media_current_dx, media_current_dy)
- [ ] 2.4 Add unit test: verify PolygonGpuData size matches expected byte count after adding new fields (must be aligned for GPU storage buffer)

## 3. Shader: Media Vertex Detection and Height Override

- [ ] 3.1 In the vertex shader, detect media vertices by testing bit 31 of texture_descriptor; mask it off (AND with 0x7FFFFFFF) before texture lookup
- [ ] 3.2 When a media vertex is detected, replace position.y with `polygon_data.media_height` from the storage buffer
- [ ] 3.3 In the fragment shader, when a media vertex is detected, apply per-type visual properties: look up alpha and tint color from media_type constant arrays, multiply texture sample by tint, set output alpha to the type's base alpha
- [ ] 3.4 For lava (media_type=1), bypass light intensity dimming (emissive)
- [ ] 3.5 Apply these shader changes identically to marathon-game/src/shader.wgsl, marathon-viewer/src/shader.wgsl, and marathon-web/src/shader.wgsl

## 4. Shader: Wobble and Flow UV for Media Surfaces

- [ ] 4.1 In the fragment shader, when processing a media vertex, force wobble UV distortion regardless of the declared transfer_mode (apply sinusoidal UV offset based on world position and elapsed_time)
- [ ] 4.2 After wobble, add linear UV scroll offset: `uv += vec2(media_current_dx, media_current_dy) * elapsed_time` read from the polygon storage buffer
- [ ] 4.3 Verify wobble + flow compose correctly: wobble provides periodic ripple, flow provides steady directional drift

## 5. Render Pass: Alpha-Blended Media Sub-Pass

- [ ] 5.1 Create a second wgpu render pipeline (media_pipeline) in marathon-game/src/render.rs with alpha blending (src_factor: SrcAlpha, dst_factor: OneMinusSrcAlpha), depth test enabled (Less), depth write disabled, back-face culling enabled
- [ ] 5.2 In the render() method, draw opaque geometry with the existing pipeline using index range 0..opaque_index_count, then switch to media_pipeline and draw opaque_index_count..total_index_count
- [ ] 5.3 Apply the same two-pipeline split to marathon-viewer/src/render.rs
- [ ] 5.4 Apply the same two-pipeline split to marathon-web/src/render.rs (using batched draw calls: media batches drawn with media pipeline)
- [ ] 5.5 Add `is_media: bool` flag to `DrawBatch` in marathon-web/src/mesh.rs so the web renderer can distinguish media batches from opaque batches

## 6. Media Height Animation: Light-Driven Updates

- [ ] 6.1 In the per-frame storage buffer update path (render.rs in each crate), compute `media_height` using `compute_media_height(media, light_intensity)` where light_intensity comes from evaluating the media's linked light at the current tick
- [ ] 6.2 Replace the static `media.height as f32 / 1024.0` lookup with the dynamically computed height
- [ ] 6.3 Add unit test: given a media with light_index=5, low=0.0, high=2.0, and light intensity 0.5, verify media_height in PolygonGpuData equals 1.0

## 7. Underwater Tint Overlay

- [ ] 7.1 Add `is_camera_submerged: u32` and `submersion_tint: [f32; 4]` to the camera uniform struct (or a new small uniform buffer)
- [ ] 7.2 Implement camera submersion check in each crate's render.rs: determine which polygon the camera is in (from sim state), look up media_index, compare camera Y against media_height
- [ ] 7.3 Create the tint pipeline: fullscreen triangle vertex shader (no vertex buffer, 3 hardcoded vertices), fragment shader that outputs constant color from uniform. Alpha blending enabled, depth test disabled, depth write disabled
- [ ] 7.4 In the render pass, after the media sub-pass and before sprite rendering, conditionally draw the tint quad if is_camera_submerged is true
- [ ] 7.5 Apply tint pipeline to all three crates (marathon-game, marathon-viewer, marathon-web)

## 8. Sim: Submersion Query and Media Detonation Events

- [ ] 8.1 Add `is_submerged(eye_height: f32, polygon_media_height: f32) -> bool` helper to marathon-sim/src/world_mechanics/media.rs
- [ ] 8.2 Add `media_tint_color(media_type: i16) -> [f32; 4]` function to marathon-sim/src/world_mechanics/media.rs returning the per-type underwater tint RGBA
- [ ] 8.3 Expose a `player_submersion_state()` query on SimWorld (or SimSnapshot) returning Option<(bool, i16)> -- (submerged, media_type)
- [ ] 8.4 Add `SimEvent::MediaDetonation { position: Vec3, media_type: i16, effect_size: u8 }` variant
- [ ] 8.5 In the projectile movement system, detect when projectile Z crosses a polygon's media_height and emit MediaDetonation events
- [ ] 8.6 Add unit tests: is_submerged returns true when eye below media height, false when above; media_tint_color returns correct RGBA per type

## 9. Splash Sprite Rendering

- [ ] 9.1 In marathon-game/src/render.rs, receive MediaDetonation events from the sim event queue and create temporary SpriteDrawCall entries with an 8-tick lifetime
- [ ] 9.2 Maintain a Vec of active splash sprites; decrement lifetime each tick and remove expired entries
- [ ] 9.3 Add splash sprites to the sprite batch alongside entity sprites before passing to SpriteRenderer::render()
- [ ] 9.4 Apply the same splash sprite handling to marathon-web/src/render.rs

## 10. Testing and Verification

- [ ] 10.1 Run full Docker build+test suite (cargo test for all crates) and verify all existing tests pass with the new changes
- [ ] 10.2 Manually test in marathon-viewer: load a level with water (e.g., Waterloo Waterpark) and verify transparent blue water surface is visible, animates with wobble, and the floor is visible beneath
- [ ] 10.3 Manually test underwater tint: fly camera below water surface and verify blue screen tint appears
- [ ] 10.4 Manually test in marathon-web: verify media surfaces render correctly in the browser with WebGL2 backend
- [ ] 10.5 Verify lava renders as near-opaque orange with emissive glow (no light dimming)
- [ ] 10.6 Run e2e test suite and verify no regressions in existing tests
