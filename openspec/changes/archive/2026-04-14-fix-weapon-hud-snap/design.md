## Context

The weapon overlay renderer (`WeaponOverlayRenderer::render()` in `marathon-web/src/sprites.rs`) draws a screen-space quad in NDC coordinates with `bottom = -1.0`, anchoring the weapon sprite at the very bottom of the WebGPU canvas. The HTML HUD panel (`#hud` in `index.html`) is a 128px-tall opaque element positioned fixed at the bottom of the viewport with `z-index: 6`, sitting on top of the canvas. This means the bottom ~128px of the weapon sprite is hidden behind the HUD, making the weapon appear disconnected.

The `viewport_width` and `viewport_height` parameters are already passed into `WeaponOverlayRenderer::render()`, so computing the HUD offset requires no API changes.

## Goals / Non-Goals

**Goals:**
- Shift the weapon sprite's NDC bottom anchor up so the visible portion sits flush against the top edge of the HUD panel
- Keep the fix minimal -- single expression change in the NDC quad calculation

**Non-Goals:**
- Making the HUD height dynamically configurable or reading it from the DOM at runtime (the 128px height is a known constant matching the CSS)
- Changing the weapon sprite's scale or aspect ratio
- Rendering the weapon sprite into the HUD panel itself
- Refactoring the overlay pipeline

## Decisions

**Decision: Compute HUD offset in NDC space using a constant**

The HUD height is 128px, defined in CSS. In NDC, the full viewport spans [-1, 1] (height = 2.0 in NDC units). The fraction of viewport covered by the HUD is `128.0 / viewport_height`. Converting to NDC offset: `hud_ndc = 2.0 * 128.0 / viewport_height`. The weapon bottom becomes `bottom = -1.0 + hud_ndc`.

Alternative considered: reading the HUD height from the DOM via JavaScript interop. Rejected because the HUD height is a fixed design constant, and JS interop adds unnecessary complexity and per-frame overhead for a static value.

Alternative considered: using a Rust constant `HUD_HEIGHT_PX: f32 = 128.0`. This is the approach -- it keeps the value self-documenting and easy to update if the HUD height ever changes.

## Risks / Trade-offs

- [Risk] HUD height changes in CSS without updating the Rust constant → weapon misalignment. Mitigation: add a comment in both locations referencing each other.
- [Risk] At very small viewport heights (< 256px), the HUD offset could push the weapon mostly off-screen. Mitigation: acceptable -- Marathon targets normal display resolutions; extremely small viewports are not a supported use case.
