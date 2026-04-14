# fix-weapon-hud-snap Tasks

## Summary
Fix first-person weapon sprite being hidden behind the HUD panel by offsetting the weapon overlay's NDC bottom coordinate to account for the HUD's pixel height.

## Tasks

- [x] Add `WeaponOverlayRenderer` to `marathon-web/src/sprites.rs` that renders the first-person weapon as a screen-space NDC quad with configurable bottom offset
- [x] Integrate `WeaponOverlayRenderer` into the render loop in `marathon-web/src/render.rs` — create it during init, call `render()` each frame with viewport height
- [x] Add HUD panel `<div>` element (128px tall, opaque background) to `marathon-web/static/index.html` positioned at bottom of viewport
- [x] Verify the build compiles: `docker build -f Dockerfile.web --target builder .`
