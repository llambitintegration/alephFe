## Why

The marathon-web WASM build renders the level but has three critical issues that make it unplayable: (1) the sim's `tick()` is a stub that never processes player input, so WASD/mouse do nothing; (2) the player spawn position uses Marathon's 2D map coordinates (x, y, z) mapped directly to 3D (x, y, z) without swapping Y/Z, placing the camera above the level; and (3) there is no HUD, weapon overlay, or any game UI beyond raw 3D geometry. These must be fixed to make the web build a functional, playable experience.

## What Changes

- **Wire up player physics in `sim.tick()`**: Connect the existing player movement/look systems so `ActionFlags` from keyboard/mouse input actually drive the player entity each tick.
- **Fix coordinate mapping in render.rs**: The camera reads `sim.player_position()` which returns `Vec3(map_x, map_y, map_z)`. The mesh builder uses `(map_x, floor_height, map_y)` for vertex positions. The camera must apply the same mapping: `Vec3(pos.x, floor_height + EYE_HEIGHT, pos.y)` instead of using the sim's raw Y as the vertical axis.
- **Add basic HUD rendering**: Render a minimal HUD overlay showing health, shield/oxygen bars, and weapon readout — either via a second wgpu render pass or an HTML/CSS overlay driven from sim state.
- **Add weapon view model**: Render the first-person weapon sprite at the bottom of the viewport, matching Marathon's weapon display.
- **Add automap toggle**: Render a 2D overhead map when the player presses the automap key, showing explored polygons and the player's position.

## Capabilities

### New Capabilities
- `web-hud`: Health, shield, oxygen bars and weapon readout overlay for the web build
- `web-weapon-view`: First-person weapon sprite rendering in the web viewport
- `web-automap`: 2D overhead automap display toggled by keypress

### Modified Capabilities
- `player-physics`: Wire up the existing player movement and look systems into `SimWorld::tick()` so input flags are actually processed
- `level-rendering`: Fix camera coordinate mapping to match mesh coordinate system (swap Y/Z from sim position)
- `input-system`: Ensure pointer lock engagement and keyboard focus work reliably in the web build

## Impact

- **marathon-sim/src/tick.rs**: The `tick()` stub must call player physics, monster AI, combat, and world mechanics systems
- **marathon-web/src/render.rs**: Camera position calculation, new HUD/weapon render passes, automap rendering
- **marathon-web/src/mesh.rs**: May need additional vertex data for automap lines
- **marathon-web/src/lib.rs**: Expose HUD state queries from sim to the render loop
- **marathon-web/src/shader.wgsl**: Possible new shader for HUD/automap orthographic rendering
- **marathon-web/static/index.html**: If using HTML overlay for HUD, add DOM elements
