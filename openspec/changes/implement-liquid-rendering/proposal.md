## Why

Media polygons (water, lava, goo, sewage, jjaro) are parsed and simulated but never rendered. `build_media_surface()` in `mesh.rs` emits geometry at a static height and the GPU storage buffer receives `media_height` updates each tick, but the shader treats media triangles identically to floors -- no transparency, no draw-order separation, no animated height readback, and no visual distinction between media types. The result: liquid areas appear as opaque floor-textured planes (or invisible voids when the texture is missing), the player can walk through lava with no visual feedback, and submerging produces no screen effect. Marathon's liquids are a core environmental mechanic -- they gate exploration (rising water), create hazards (lava pits), and provide atmosphere (sewage currents). Without visible, animated, transparent media surfaces and submersion feedback, levels that depend on liquids are unplayable.

## What Changes

- **Separate media into its own draw pass**: Split the single `draw_indexed` call so opaque geometry (walls, floors, ceilings) renders first, then media surfaces render in a second sub-pass with alpha blending enabled and depth-write disabled. This matches Marathon's canonical draw order (walls -> ceilings -> far objects -> media -> near objects -> floors) and prevents media from z-fighting with floors or occluding sprites behind transparent liquid.
- **Animate media height in the vertex shader**: The vertex shader currently uses the baked `position.y` for media vertices. Change media vertices to read `media_height` from the per-polygon storage buffer so the surface rises and falls with the light-driven animation that `compute_media_height()` already calculates on the CPU each tick.
- **Per-type visual treatment**: Assign each media type (water/lava/goo/sewage/jjaro) a base alpha and tint color. Water is semi-transparent blue, lava is opaque orange with emissive glow, goo is semi-transparent green, sewage is murky brown, jjaro is translucent purple. Apply wobble transfer mode to all liquid surfaces for the characteristic ripple effect.
- **Underwater screen tint**: When the camera Y is below the media surface height of the player's current polygon, apply a fullscreen color overlay (type-dependent tint + slight fog) as a post-geometry pass. This requires tracking which polygon the camera is in and comparing eye height against that polygon's media height.
- **Splash effects on projectile impact**: When a projectile crosses a media surface boundary (enters or exits liquid), emit a splash sprite at the intersection point. The sim already has `media_detonation_effect` in `ProjectileDefinition`; wire this to spawn a short-lived billboard sprite via the existing `SpriteRenderer`.
- **Media-specific surface texture scrolling**: Use the media's `current_direction` and `current_magnitude` to scroll the surface texture UV coordinates, simulating liquid flow/current visually.

## Capabilities

### New Capabilities
- `liquid-surface-rendering`: Transparent, animated media surface polygons rendered in a dedicated alpha-blended draw pass with per-type visual properties (alpha, tint, ripple)
- `underwater-tint`: Fullscreen color overlay when the camera is submerged in media, with tint color and fog density varying by media type
- `media-splash-effects`: Billboard splash sprites spawned at projectile-media surface intersection points

### Modified Capabilities
- `mesh-generation`: Media surface vertices flagged for shader-driven height animation; media index ranges tracked separately from opaque geometry for split draw calls
- `level-rendering`: Render pass split into opaque geometry, media surface, and post-process sub-passes; pipeline state management for alpha blending toggle
- `transfer-modes`: Wobble mode applied to media surfaces by default; UV scroll driven by media current direction/magnitude
- `world-mechanics`: Media simulation exposes submersion state (is camera submerged, in which media type) to the rendering bridge

## Impact

- **marathon-game/src/mesh.rs** -- `build_level_mesh` returns separate opaque and media index ranges; `build_media_surface` flags vertices for shader height readback
- **marathon-game/src/render.rs** -- `GpuState::render()` splits into opaque draw + media draw with separate pipeline state; new fullscreen quad pass for underwater tint; `PolygonGpuData` extended with `media_light` and `media_type` fields
- **marathon-game/src/shader.wgsl** -- Vertex shader reads `media_height` from storage buffer for media vertices; fragment shader applies per-type alpha/tint and wobble UV
- **marathon-sim/src/world.rs** -- `SimSnapshot` or a new render-facing query exposes player submersion state (submerged bool, media type)
- **marathon-sim/src/world_mechanics/media.rs** -- Add `is_submerged(eye_height, media_height) -> bool` helper; expose media tint color constants per type
- **marathon-game/src/sprites.rs** -- Accept splash effect draw calls from sim events
- **marathon-web/src/mesh.rs** and **marathon-web/src/render.rs** -- Parallel changes for WASM build parity
- **Depends on**: Light state machine must be ticking correctly for `compute_media_height()` to animate; this is already implemented in `marathon-sim/src/world_mechanics/lights.rs`
