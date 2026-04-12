## Context

Media polygons (water, lava, goo, sewage, jjaro) are parsed by marathon-formats and simulated by marathon-sim, but never rendered visibly. The `build_media_surface()` function in each crate's `mesh.rs` emits triangulated geometry at the media's static baked height, and the GPU storage buffer carries `media_height` and `media_transfer_mode` per polygon -- but the shader treats media triangles identically to opaque floors. There is no transparency, no draw-order separation, no animated height readback from the light state machine, and no visual distinction between media types.

The result: liquid areas appear as opaque floor-textured planes (or invisible voids when the texture is missing), the player can walk through lava with no visual feedback, and submerging produces no screen effect. Marathon's liquids are a core environmental mechanic -- they gate exploration (rising water), create hazards (lava pits), and provide atmosphere (sewage currents). Levels that depend on liquids are unplayable without visible, animated, transparent media surfaces and submersion feedback.

### Current State

- **marathon-formats**: `MediaData` fully parsed (type, light_index, current_direction/magnitude, low/high, texture, transfer_mode, origin).
- **marathon-sim**: `compute_media_height(media, light_intensity)` interpolates correctly between bounds. `media_deals_damage()`, `media_drag_factor()` exist. The `Media` component stores polygon_index, media_type, height bounds, light_index, current_direction/magnitude.
- **mesh.rs (all crates)**: `build_media_surface()` emits fan-triangulated geometry at `media.height` (static). Media vertices are indistinguishable from opaque floor vertices in the GPU pipeline.
- **render.rs (all crates)**: `PolygonGpuData` includes `media_height` and `media_transfer_mode` fields uploaded to the storage buffer each frame. A single `draw_indexed()` call draws all geometry (opaque + media) in one pass with depth-write enabled.
- **shader.wgsl**: `PolygonData` struct has `media_height` and `media_transfer_mode` but neither field is read by the vertex or fragment shader.

## Goals / Non-Goals

**Goals:**
- Render media surfaces as transparent, animated polygons distinct from opaque level geometry
- Split the render pass so media draws after opaque geometry with alpha blending and depth-write disabled
- Animate media surface height from the light state machine each tick (already computed by `compute_media_height()`)
- Apply per-type visual properties: alpha, tint color, wobble ripple effect
- Scroll media surface UVs based on `current_direction` and `current_magnitude` to simulate flow
- Tint the screen when the camera is submerged, with color and density varying by media type
- Spawn splash sprites when projectiles cross media surface boundaries

**Non-Goals:**
- Order-independent transparency (simple back-to-front within a polygon is sufficient; media surfaces rarely overlap)
- Depth peeling or multi-layer transparency
- Underwater fog with distance attenuation (screen tint is sufficient for first pass)
- Caustic light patterns on submerged floors
- Refraction distortion when looking through liquid from above
- Sound attenuation or ambient underwater audio (separate change)

## Decisions

### 1. Two-pass rendering: opaque geometry then media surfaces

**Decision**: Split the single `draw_indexed()` call into two sub-passes within the same render pass. Pass 1 draws opaque geometry (walls, floors, ceilings) with depth-write enabled. Pass 2 draws media surfaces with alpha blending enabled and depth-write disabled.

**Rationale**: This is the simplest approach that achieves correct transparency compositing. Media surfaces need to be depth-tested against opaque geometry (so they are occluded by walls in front of the camera) but must not write to the depth buffer (so floors visible through transparent liquid are not clipped). Marathon's original draw order (walls -> ceilings -> media -> floors) achieves the same effect through BSP ordering; our depth-buffer approach with two passes is the GPU equivalent.

**Alternative considered**: Separate wgpu render passes with shared depth buffer. Rejected because two passes add unnecessary command buffer overhead; two draw calls within a single pass share the same depth attachment naturally.

**Implementation**: `LevelMesh` returns an `opaque_index_count` marking the boundary between opaque and media indices. The mesh builder emits all opaque geometry first, then all media geometry. Render pass 1 draws `0..opaque_index_count`; pass 2 switches to the alpha-blend pipeline and draws `opaque_index_count..total_index_count`. For marathon-web, the same split applies per batch -- each `DrawBatch` carries an `is_media` flag, and media batches are drawn in the second sub-pass.

### 2. Media height driven by vertex shader storage buffer readback

**Decision**: Media vertices continue to be emitted at a baked Y position by the mesh builder, but the vertex shader overrides Y for media vertices by reading `media_height` from the per-polygon storage buffer.

**Rationale**: This is already the pattern used for platform vertices -- the mesh is static, and the storage buffer is updated each frame with current heights. Media height animation is driven by the light state machine (`compute_media_height()`), which already runs on the CPU each tick. Uploading the new height to the storage buffer is the same path used for platform heights. No vertex buffer rebuild is needed.

**Implementation**: Add a vertex attribute or use the existing `texture_descriptor` to flag media vertices (e.g., a sentinel bit in the descriptor's high bits, or a new `flags` field in the Vertex struct). The vertex shader checks this flag and, if set, replaces `position.y` with `polygon_data.media_height`.

### 3. Per-type visual properties as shader constants

**Decision**: Define media visual properties (base alpha, tint RGBA, emissive flag) as constants in the shader, indexed by media type. Add `media_type` to `PolygonGpuData` so the fragment shader can look up the correct properties.

**Rationale**: There are only 5 media types with fixed visual properties. Encoding them as shader constants avoids an additional GPU buffer and keeps the data path simple. The `media_type` field (0-4) is a single u32 added to the per-polygon storage buffer.

**Visual properties per type:**
| Type | Alpha | Tint RGBA | Emissive |
|------|-------|-----------|----------|
| Water (0) | 0.55 | (0.1, 0.3, 0.8, 1.0) | No |
| Lava (1) | 0.90 | (1.0, 0.4, 0.1, 1.0) | Yes |
| Goo (2) | 0.65 | (0.2, 0.7, 0.1, 1.0) | No |
| Sewage (3) | 0.75 | (0.5, 0.4, 0.2, 1.0) | No |
| Jjaro (4) | 0.60 | (0.4, 0.2, 0.8, 1.0) | No |

### 4. Wobble transfer mode as default for media, plus UV scroll from current

**Decision**: Apply wobble transfer mode to all media surfaces regardless of their declared `transfer_mode`, then additionally scroll UVs based on `current_direction` and `current_magnitude` to simulate liquid flow.

**Rationale**: Marathon's original engine applies wobble to media by default. The UV scroll from current direction is additive -- it shifts the texture origin over time proportional to the current vector. Both effects compose naturally: wobble distorts the UVs periodically, and current scroll shifts the base offset linearly.

**Implementation**: In the fragment shader, when rendering a media surface, apply the wobble UV distortion first, then add a linear UV offset computed as `current_direction * current_magnitude * elapsed_time`. Add `media_current_dx` and `media_current_dy` to `PolygonGpuData` (the direction decomposed into XZ components multiplied by magnitude).

### 5. Fullscreen tint quad for underwater effect

**Decision**: When the camera is submerged, render a fullscreen quad with the media type's tint color at a low alpha (0.25-0.40) as a third sub-pass after media surfaces but before sprites.

**Rationale**: A fullscreen colored quad is the simplest possible underwater effect and closely matches Marathon's software renderer fader effect. It requires only a trivial shader (constant color output) and a single draw call. No post-processing pipeline or framebuffer copy is needed.

**Implementation**: Add a `is_camera_submerged` bool and `submerged_media_type` u32 to the camera uniform or a new small uniform buffer. The submersion check runs on the CPU: determine which polygon the camera is in, look up that polygon's media_index, and compare camera Y against the current media height. If submerged, draw the tint quad. The tint quad pipeline uses alpha blending with no depth test.

**Tint colors per type:**
| Type | Tint RGBA |
|------|-----------|
| Water | (0.1, 0.2, 0.6, 0.30) |
| Lava | (0.6, 0.1, 0.0, 0.40) |
| Goo | (0.1, 0.5, 0.1, 0.35) |
| Sewage | (0.3, 0.4, 0.1, 0.30) |
| Jjaro | (0.3, 0.1, 0.5, 0.30) |

### 6. Splash effects via existing SpriteRenderer

**Decision**: When the sim detects a projectile crossing a media surface boundary, emit a `SimEvent::MediaDetonation { position, effect_type }`. The render layer maps this to a `SpriteDrawCall` using the media's detonation effect shape descriptor and passes it to the existing `SpriteRenderer`.

**Rationale**: The `SpriteRenderer` already handles billboarded quads. Media detonation effects are just short-lived sprites at a specific world position. The sim already has `media_detonation_effect` in `ProjectileDefinition`. Wiring the event through the existing sprite pipeline avoids any new rendering infrastructure.

**Implementation**: In marathon-sim, when a projectile's Z position crosses a polygon's media height during movement, emit the event with the appropriate splash effect index (small/medium/large based on projectile type). The render layer receives the event, creates a temporary `SpriteDrawCall` with a short lifetime (6-10 ticks), and adds it to the sprite batch.

### 7. Vertex flagging via high bit in texture_descriptor

**Decision**: Use bit 31 of the existing `texture_descriptor` u32 in the Vertex struct to flag media surface vertices, rather than adding a new vertex attribute.

**Rationale**: Adding a new vertex attribute changes the vertex layout, requiring pipeline recreation and buffer format changes across all three crates. The `texture_descriptor` is a `ShapeDescriptor` that uses at most 16 bits (5 bits collection + 8 bits bitmap + 3 bits CLUT). Bit 31 is safely unused. The vertex shader masks bit 31 to detect media, and masks it off before using the descriptor for texture lookup.

**Alternative considered**: A separate `flags` u32 attribute. Rejected because it increases vertex size by 4 bytes for every vertex (not just media vertices) and requires layout changes. The bit-flag approach is zero-cost in vertex size.

## Risks / Trade-offs

- **[Risk] Alpha blending artifacts with overlapping media**: If two media polygons overlap (rare in Marathon maps), the back-to-front ordering may be incorrect for some camera angles. Mitigation: Marathon maps almost never have overlapping media. If artifacts appear, sort media polygons by distance to camera before drawing.
- **[Risk] WebGL2 alpha blending performance**: The marathon-web crate uses WebGL2 via wgpu's GL backend. Alpha blending is supported but may be slower on mobile GPUs. Mitigation: Media surfaces are typically a small fraction of total geometry; the additional blending cost is negligible.
- **[Risk] Vertex descriptor bit collision**: If a future shapes file uses more than 16 bits in the descriptor, the media flag bit could conflict. Mitigation: Marathon's format is fixed at 16-bit descriptors; this will not change.
- **[Trade-off] Wobble forced on all media**: Some media entries declare a non-wobble transfer_mode. We override to wobble for visual consistency. If a level author specifically needs non-wobble media, this would need a configuration option later.
- **[Trade-off] Fullscreen tint vs. proper fog**: The underwater tint is a flat color overlay, not distance-based fog. This is less realistic than Alephone's OpenGL fog but matches the software renderer's fader approach and is much simpler to implement. Distance-based fog can be added as a follow-up enhancement.
