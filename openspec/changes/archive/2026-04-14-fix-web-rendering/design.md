## Context

The marathon-web WASM build successfully renders level geometry with correct textures and lighting after recent WebGL2 compatibility fixes. However, three issues make it unplayable:

1. **No input processing**: `SimWorld::tick()` in marathon-sim is a stub (`// TODO: Wire up actual system functions`). It stores action flags and increments a counter but never calls player physics, monster AI, or any simulation system. WASD and mouse input are captured but have zero effect.

2. **Wrong camera coordinates**: The sim stores the player position as `Vec3(map_x, map_y, map_z)` where map_x/map_y are the 2D horizontal map coordinates and map_z is the vertical height. But the mesh builder maps Marathon coordinates to 3D as `(map_x, vertical_height, map_y)`. The camera in render.rs reads `sim.player_position()` and uses `.y` as the vertical axis, but the sim's `.y` is the second horizontal axis — placing the camera ~19 world units above the level.

3. **No game UI**: The web build renders raw 3D geometry with no HUD (health/shield/oxygen), no weapon view model, and no automap.

## Goals / Non-Goals

**Goals:**
- Make the player controllable: WASD movement, mouse look, with correct collision detection
- Fix the camera to be at the correct vertical position (floor height + eye height)
- Add a basic HTML/CSS HUD overlay showing health, shield, and oxygen
- Add first-person weapon sprite rendering
- Add a basic automap toggle

**Non-Goals:**
- Full monster AI or combat (monsters can remain static for now)
- Sound or music playback
- Terminal interaction or level transitions
- Networked multiplayer
- Full inventory/motion sensor HUD (basic health/shield/oxygen only for first pass)
- Save/load functionality

## Decisions

### 1. Wire player physics into tick() directly, defer other systems

**Decision**: Implement only the `player_physics_system` call in `tick()`. Leave monster AI, combat, projectiles, and world mechanics as stubs for now.

**Rationale**: The player physics system already exists in marathon-sim (`player_physics.rs`) with movement, collision, gravity, and step climbing. Wiring it into tick() is the minimum change to make the game controllable. Other systems add complexity without addressing the core "can't move" issue.

**Alternative considered**: Wire up all systems at once. Rejected because it risks introducing bugs in untested systems and delays the critical input-to-movement fix.

### 2. Fix coordinate mapping in render.rs, not in the sim

**Decision**: Change `render.rs`'s camera position calculation from `Vec3(pos.x, pos.y + EYE_HEIGHT, pos.z)` to `Vec3(pos.x, pos.z + EYE_HEIGHT, pos.y)` to match the mesh builder's coordinate system.

**Rationale**: The sim uses Marathon's native coordinate system where (x, y) are horizontal and z is vertical. The mesh builder already converts this correctly: `world_to_f32(ep.vertex.x)` → X, `world_to_f32(floor_height)` → Y (vertical), `world_to_f32(ep.vertex.y)` → Z. The render.rs camera should use the same mapping rather than changing the sim's coordinate convention.

**Alternative considered**: Change the sim to output positions in the mesh coordinate system. Rejected because it would affect all sim consumers and violate the sim's responsibility to use Marathon-native coordinates.

### 3. HTML/CSS overlay for HUD instead of wgpu render pass

**Decision**: Use an HTML `<div>` overlay positioned on top of the canvas for the HUD, with JavaScript updating the DOM from sim state exposed via wasm-bindgen.

**Rationale**: A wgpu 2D render pass for HUD would require a new shader, orthographic projection, sprite atlas loading, and text rendering. An HTML overlay is trivial to implement (CSS styling, DOM updates) and can be iterated on quickly. It also works regardless of WebGL state.

**Alternative considered**: wgpu-based HUD rendering. Deferred to a future change — the HTML overlay gets us playable faster.

### 4. Weapon view as a screen-space textured quad

**Decision**: Render the weapon sprite as a camera-facing quad in a second render pass after the level geometry, positioned at a fixed screen-space offset (bottom center). Use the existing sprite texture pipeline.

**Rationale**: Marathon's weapon sprites are already in the shapes file. The existing `SpriteRenderer` can render billboarded quads. A screen-space weapon quad reuses this infrastructure with a fixed position relative to the camera.

### 5. Automap as a 2D canvas overlay

**Decision**: Render the automap as a separate 2D HTML canvas overlay when toggled, drawing polygon edges and the player position marker using Canvas 2D API.

**Rationale**: The automap is a 2D line drawing. Using the Canvas 2D API on a separate `<canvas>` element avoids adding complexity to the wgpu pipeline and is easy to toggle on/off.

## Risks / Trade-offs

- **[Risk] Player physics may have bugs when first wired up** → The physics system has unit tests but hasn't been run in a real game loop. Mitigate by adding integration-level tests and testing in the browser iteratively.
- **[Risk] HTML HUD may have performance issues with frequent DOM updates** → Mitigate by throttling updates to 10fps (every 3 ticks) rather than every frame. Only update changed values.
- **[Risk] Coordinate mapping fix may affect sprite rendering** → Sprites also read `sim.player_position()` for camera position. Must apply the same coordinate swap to sprite billboard calculations.
- **[Trade-off] HTML HUD is not pixel-accurate to original Marathon** → Accepted for now. A wgpu-based HUD can replace it later for visual fidelity.
