## 1. Sim API: Expose Weapon and Entity Data

- [x] 1.1 Add `player_weapon_info()` method to SimWorld in tick.rs that returns `Option<(usize, u16, u16)>` (definition_index, primary_ammo, secondary_ammo) by reading from WeaponInventory
- [x] 1.2 Add `nearby_entities()` method to SimWorld in tick.rs that returns `Vec<(f32, f32, u8)>` (relative_x, relative_z, entity_type) for entities within sensor range, capped at 16 results sorted by distance

## 2. Web Layer: Expand update_hud to Pass New Data

- [x] 2.1 Update `update_hud()` in render.rs to accept and pass weapon info (definition_index, primary_ammo, secondary_ammo) to DOM elements
- [x] 2.2 Update `update_hud()` in render.rs to accept and pass nearby entity data to the motion sensor canvas via JS interop
- [x] 2.3 Call `player_weapon_info()` and `nearby_entities()` from the render loop and pass results to `update_hud()`

## 3. HUD HTML/CSS: Three-Column Opaque Panel

- [x] 3.1 Replace the existing 48px HUD div in index.html with a ~128px opaque three-column CSS Grid layout (left: motion sensor, center: vitals, right: weapon info)
- [x] 3.2 Add retro styling: monospace font, dark background (#1a1a1a), segmented bar appearance via CSS repeating-linear-gradient, subtle panel borders
- [x] 3.3 Resize the 3D canvas to end above the HUD: set canvas height to `calc(100vh - 128px)` so viewport does not overlap the panel

## 4. Motion Sensor: Canvas 2D Radar

- [x] 4.1 Add a `<canvas>` element in the left HUD column for the motion sensor circle
- [x] 4.2 Implement JS function `updateMotionSensor(playerYaw, entities)` that draws the radar circle background, crosshair, and entity dots positioned relative to player facing
- [x] 4.3 Wire WASM entity data from render.rs to the motion sensor JS function each frame

## 5. Weapon Display: Name and Ammo

- [x] 5.1 Add weapon name and ammo count DOM elements in the right HUD column
- [x] 5.2 Add JS weapon name lookup array mapping definition_index to weapon name strings
- [x] 5.3 Wire WASM weapon info from render.rs to update weapon name and ammo DOM elements each frame
