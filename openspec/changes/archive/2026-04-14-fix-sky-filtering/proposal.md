## Why

Landscape/sky textures appear grainy and pixelated because all textures share a single nearest-neighbor sampler. Wall and floor textures benefit from nearest-neighbor filtering (preserving the pixel-art look), but landscape textures are low-resolution Marathon bitmaps stretched across the entire sky via per-fragment view-angle calculations. Nearest-neighbor sampling on these stretched textures produces visible blocky pixels instead of a smooth sky.

## What Changes

- Add a second wgpu sampler with `FilterMode::Linear` (bilinear filtering) for landscape textures
- Update the texture bind group layout and bind groups to include both the nearest and linear samplers
- Modify the fragment shader to select the linear sampler when `transfer_mode == TRANSFER_LANDSCAPE` and the nearest sampler for all other transfer modes
- No changes to texture creation (mip_level_count stays at 1 for now; bilinear filtering alone addresses the visible blockiness)

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `texture-pipeline`: Add a linear sampler alongside the existing nearest sampler in the texture bind group, so the fragment shader can choose filtering mode per surface
- `transfer-modes`: Landscape transfer mode selects the linear sampler for smooth sky rendering

## Impact

- `marathon-web/src/render.rs` — sampler creation, bind group layout, bind group construction
- `marathon-web/src/sprites.rs` — if sprites share the same bind group layout, may need matching layout update
- `marathon-web/src/shader.wgsl` — fragment shader sampler selection logic
- No API changes, no new dependencies, no breaking changes
