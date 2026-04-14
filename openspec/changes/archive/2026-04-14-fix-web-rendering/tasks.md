## 1. Fix Player Physics in Sim

- [x] 1.1 Wire player_physics_system into SimWorld::tick() so ActionFlags drive player movement, turning, gravity, and collision each tick
- [x] 1.2 Add unit tests verifying that tick() with MOVE_FORWARD flag changes player Position, and tick() with LOOK_RIGHT flag changes player Facing
- [x] 1.3 Verify existing player physics tests still pass after wiring into tick()

## 2. Fix Camera Coordinate Mapping

- [x] 2.1 Fix render.rs camera position: change from Vec3(pos.x, pos.y + EYE_HEIGHT, pos.z) to Vec3(pos.x, pos.z + EYE_HEIGHT, pos.y) to match mesh coordinate system
- [x] 2.2 Fix render.rs camera to read VerticalLook from sim for pitch (add player_vertical_look() query to SimWorld if needed)
- [x] 2.3 Fix sprite rendering camera position to use the same coordinate swap (pos.x, pos.z + EYE_HEIGHT, pos.y)
- [x] 2.4 Add unit test for coordinate mapping: given sim position (9.4, 18.8, 0.0), camera should be at (9.4, 0.66, 18.8)

## 3. Fix Web Input Pipeline

- [x] 3.1 Verify pointer lock activates on canvas click (add console.log in click handler for debugging if needed)
- [x] 3.2 Verify keyboard events fire on canvas (add logging in keydown handler to confirm events reach WASM)
- [x] 3.3 Add mouse sensitivity scaling: scale raw movementX/Y by a configurable factor before accumulating into mouse_dx/dy (current raw pixels are too sensitive)
- [x] 3.4 Add e2e test: simulate click on canvas, verify pointer lock request fires
- [x] 3.5 Remove diagnostic logging added during debugging (physics parsed, light range, batch info, etc.)

## 4. HTML HUD Overlay

- [x] 4.1 Add HUD DOM elements to index.html: health bar, shield bar, oxygen meter containers positioned at bottom of viewport
- [x] 4.2 Add CSS styling for HUD bars (health green/yellow/red tiers, shield blue/cyan, oxygen blue)
- [x] 4.3 Expose player_health(), player_shield(), player_oxygen() from SimWorld via wasm-bindgen in lib.rs
- [x] 4.4 Add HUD update function in render loop that reads sim state and updates DOM elements, throttled to 10fps
- [x] 4.5 Hide oxygen meter when oxygen is at maximum (normal atmosphere)

## 5. Weapon View Model

- [x] 5.1 Add weapon_state query to SimWorld returning current weapon collection, shape, and animation frame
- [x] 5.2 Create weapon sprite render pass in render.rs: render a screen-space textured quad at bottom-center using the existing SpriteRenderer pipeline
- [x] 5.3 Load weapon sprite textures from shapes file using the same CLUT conversion as entity sprites
- [x] 5.4 Update weapon sprite each frame based on sim weapon animation state

## 6. Automap

- [x] 6.1 Add a second canvas element (#automap-canvas) to index.html, styled as a semi-transparent overlay, hidden by default
- [x] 6.2 Add Tab key handler to toggle automap canvas visibility
- [x] 6.3 Implement automap rendering: draw polygon edges as 2D lines on the canvas using Canvas 2D API, centered on player position
- [x] 6.4 Draw player position marker (arrow) oriented by facing angle on the automap

## 7. Testing and Verification

- [x] 7.1 Run full Docker build+test suite (cargo test for all crates) and verify all tests pass
- [x] 7.2 Run e2e test suite (docker compose e2e) and verify all 16 tests pass
- [x] 7.3 Add e2e test: verify no console errors after game loads and runs for 5 seconds
- [x] 7.4 Deploy to marathon.llambit.io and manually verify: player can move with WASD, look with mouse, HUD displays, weapon visible, automap toggles with Tab
