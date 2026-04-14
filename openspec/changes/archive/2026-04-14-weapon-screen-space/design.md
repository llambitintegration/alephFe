## Context

The weapon sprite is currently rendered as a world-space billboard: it is pushed into the same `sprites` Vec as entity sprites and positioned 0.5 world units ahead of the camera. The existing `SpriteRenderer` constructs billboarded quads oriented toward the camera and renders them through the `view_proj` matrix with depth testing enabled. This causes the weapon to slide laterally during yaw rotation, get occluded by world geometry, and scale with FOV/distance rather than viewport size.

The original Marathon engine draws weapon sprites as 2D overlays at fixed screen coordinates, completely independent of the 3D scene. The weapon definition data (`idle_height`, `idle_width`) already exists in `marathon-formats/src/physics.rs`.

## Goals / Non-Goals

**Goals:**
- Render the weapon sprite as a screen-space overlay with no depth testing
- Center the weapon horizontally at the bottom of the viewport
- Size the weapon as a viewport-relative percentage (~35% viewport width), matching original Marathon behavior
- Reuse existing sprite texture atlas bind groups from `SpriteRenderer`

**Non-Goals:**
- Weapon bob animation (separate future change)
- Weapon firing animations or muzzle flash (already works via `WeaponRenderState.frame`)
- HUD integration or ammo display (handled by existing HUD system)
- Dual-wielding or weapon switching animations

## Decisions

### 1. Separate overlay pipeline with depth test disabled (vs. reusing SpriteRenderer)

**Chosen:** Create a dedicated `WeaponOverlayRenderer` with its own `wgpu::RenderPipeline` that has `depth_stencil: None` (no depth test/write). It shares the same texture bind group layout as `SpriteRenderer` so it can reuse the same loaded sprite collection bind groups.

**Alternative considered:** Reusing `SpriteRenderer::render()` in a second render pass with a different depth config. Rejected because `SpriteRenderer` owns a single pipeline with depth testing baked in; creating a second pass would still require a second pipeline, so it's cleaner to make a dedicated renderer.

**Alternative considered:** Rendering the weapon with a very small Z value (near plane trick). Rejected because it still involves world-space math and doesn't solve the billboard rotation problem.

### 2. Vertex shader uses pre-computed NDC positions (vs. orthographic uniform)

**Chosen:** Compute quad corners in NDC (normalized device coordinates) on the CPU and pass them directly as vertex positions. The vertex shader passes them through without any matrix multiplication. This avoids adding a second camera uniform buffer or orthographic projection matrix.

**Rationale:** The weapon overlay is a single quad per frame. CPU-side NDC computation is trivial and avoids shader/uniform complexity. The vertex positions are simply `(ndc_x, ndc_y, 0.0)` where the NDC range is [-1, 1].

### 3. Shader reuse with entry point variants

**Chosen:** Add `vs_overlay` entry point to `sprite_shader.wgsl` that passes position through without `view_proj` multiplication. The fragment shader `fs_sprite` is reused as-is. Both pipelines can share the same shader module.

### 4. Weapon sizing: viewport-relative percentages

The weapon quad width is set to ~35% of viewport width, and height is derived from the sprite's aspect ratio. Position is horizontally centered (`ndc_x = 0`) and vertically anchored at the bottom (`ndc_y = -1.0` for bottom edge, offset up by the quad height).

## Risks / Trade-offs

- **[Risk] Pipeline creation cost** — Adding a second pipeline increases GPU resource usage slightly. → Mitigated by the pipeline being created once at init time; the overhead is negligible.
- **[Risk] Texture bind group sharing** — The overlay pipeline must use the same bind group layout as `SpriteRenderer`. → Mitigated by explicitly sharing the `texture_bind_group_layout` via a public accessor or passing it at construction time.
- **[Trade-off] No aspect ratio preservation from weapon definition** — We use sprite bitmap aspect ratio for sizing rather than `idle_width`/`idle_height` from physics. → Acceptable for initial implementation; definition values can be incorporated later if needed.
