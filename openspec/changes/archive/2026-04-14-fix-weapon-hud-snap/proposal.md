## Why

The weapon sprite (fists, etc.) is rendered to the WebGPU canvas with its bottom anchored at NDC y = -1.0 (viewport bottom). However, the HTML HUD panel is a 128px-tall `position: fixed; bottom: 0` element with `z-index: 6` that sits on top of the canvas. This means the bottom portion of the weapon sprite is hidden behind the opaque HUD, making the weapon appear to float disconnected from the HUD rather than sitting flush against it.

## What Changes

- Adjust the weapon overlay NDC quad calculation in `WeaponOverlayRenderer::render()` to shift the weapon bottom up by the HUD's fraction of viewport height: `bottom = -1.0 + 2.0 * 128.0 / viewport_height`
- The viewport height is already passed to the render method, so no API changes are needed
- The weapon sprite will visually sit right at the top edge of the HUD panel instead of being hidden behind it

## Capabilities

### New Capabilities

### Modified Capabilities
- `hud-rendering`: The weapon overlay positioning requirement changes to account for the HUD panel height when computing the NDC bottom anchor

## Impact

- `marathon-web/src/sprites.rs` — single line change in `WeaponOverlayRenderer::render()` to compute `bottom` from viewport height and HUD height instead of hardcoding `-1.0`
