## Why

The weapon sprite is rendered as a world-space billboard (positioned 0.5 world units ahead of the camera), which causes it to slide when turning, get occluded by world geometry via the depth buffer, and scale incorrectly with FOV. The original Marathon engine renders weapon sprites as 2D screen-space overlays at fixed coordinates, and we should match that behavior.

## What Changes

- Remove weapon sprite from the world-space `sprites` Vec in `render.rs`
- Add a screen-space weapon overlay rendering path with depth testing disabled
- Render the weapon quad using an orthographic projection mapped to viewport coordinates
- Size and position the weapon using viewport percentages (centered horizontally, anchored at bottom), referencing `idle_height`/`idle_width` from `WeaponDefinition`
- Reuse existing sprite texture atlas infrastructure (same collections/bind groups)

## Capabilities

### New Capabilities
- `weapon-overlay`: Screen-space weapon sprite rendering as a 2D overlay with no depth testing, orthographic projection, and viewport-relative sizing/positioning

### Modified Capabilities

## Impact

- `marathon-web/src/render.rs` — weapon block in `frame()` changes from world-space sprite push to a separate overlay draw call after the main render pass
- `marathon-web/src/sprites.rs` — new method or small helper for screen-space quad generation (or a dedicated lightweight overlay pipeline)
- `marathon-web/src/sprite_shader.wgsl` — may need a variant or uniform toggle for orthographic/screen-space mode
- No simulation or format changes; `WeaponRenderState` and `WeaponDefinition` are consumed as-is
