## 1. Shader Changes

- [x] 1.1 Add `vs_overlay` vertex shader entry point to `sprite_shader.wgsl` that passes position through without `view_proj` multiplication (position is already in NDC)

## 2. Weapon Overlay Renderer

- [x] 2.1 Add `WeaponOverlayRenderer` struct to `sprites.rs` with a `wgpu::RenderPipeline` that uses `vs_overlay`/`fs_sprite`, alpha blending, and `depth_stencil: None`
- [x] 2.2 Implement `WeaponOverlayRenderer::new()` that creates the overlay pipeline, sharing the same texture bind group layout as `SpriteRenderer`
- [x] 2.3 Implement `WeaponOverlayRenderer::render()` that takes a sprite collection bind group, bitmap index, tint, and viewport dimensions, builds a screen-space quad in NDC (centered horizontally, anchored at bottom, ~35% viewport width, aspect-ratio-preserving height), and issues a draw call

## 3. Render Integration

- [x] 3.1 Add `weapon_overlay: WeaponOverlayRenderer` field to `GameState` in `render.rs`, initialize it alongside `SpriteRenderer`
- [x] 3.2 Remove the weapon sprite from the world-space `sprites` Vec (lines 219-243 in `render.rs`)
- [x] 3.3 Add a weapon overlay draw call after the main render pass ends: begin a new render pass with no depth attachment, call `weapon_overlay.render()` with the weapon's collection bind group and bitmap index

## 4. Verification

- [x] 4.1 Build and test in browser: confirm weapon is centered at bottom, does not slide with yaw, and is not occluded by walls
