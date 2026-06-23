## 1. Weapon Overlay Positioning

- [x] 1.1 Extend `WeaponRenderState` in `marathon-sim/src/tick.rs` with `vertical_position: f32` and `horizontal_position: f32` fields. In `player_weapon_state()`, populate them from the current weapon definition's `idle_height / FIXED_ONE` and `idle_width / FIXED_ONE` via `PhysicsTables`. Weapon definitions store `idle_height` and `idle_width` as fixed-point values (FIXED_ONE = 65536); divide to get normalized floats. For fists: idle_height ≈ 1.067, idle_width = 0.5.

- [x] 1.2 Update `resolve_entity_sprite()` in `marathon-web/src/sprites.rs` to return shape world bounds alongside bitmap index. Change return type from `Option<(u32, f32, f32)>` to `Option<(u32, f32, f32, f32, f32)>` returning `(bitmap_index, world_left, world_right, world_top, world_bottom)` as f32 values divided by 1024.0 (WORLD_ONE). Read these from `low_level.world_left`, `low_level.world_right`, `low_level.world_top`, `low_level.world_bottom` (i16 fields on `LowLevelShape`).

- [x] 1.3 Rewrite `WeaponOverlayRenderer::render()` in `marathon-web/src/sprites.rs` to implement `position_sprite_axis` for `_position_center` mode. Replace the fixed NDC quad with computed screen coordinates: `origin_x = screen_width * horizontal_position`, `x0 = origin_x + world_left * screen_height / 1.0`, `x1 = origin_x + world_right * screen_height / 1.0`. For Y axis: `origin_y = screen_height * vertical_position`, `y0 = origin_y + (-world_top) * screen_height / 1.0`, `y1 = origin_y + (-world_bottom) * screen_height / 1.0`. Note the Y axis uses `screen_height` for both scale and screen dimensions, and world_top/bottom are negated. Convert pixel coordinates to NDC: `ndc_x = (px / screen_dim) * 2.0 - 1.0`. Update the function signature to accept `vertical_position`, `horizontal_position`, `world_left`, `world_right`, `world_top`, `world_bottom` instead of `sprite_width`/`sprite_height`.

- [x] 1.4 Update the weapon overlay call site in `marathon-web/src/render.rs` (around line 225-290). When resolving `weapon_sprite`, call the updated `resolve_entity_sprite` to get world bounds, and pass `vertical_position`/`horizontal_position` from `WeaponRenderState` plus the world bounds to the updated `render()` method.

## 2. Sprite Billboard Asymmetric Anchoring

- [x] 2.1 Replace `width: f32, height: f32` fields in `SpriteDrawCall` (in `marathon-web/src/sprites.rs`) with `world_left: f32, world_right: f32, world_top: f32, world_bottom: f32` (all in world units, pre-divided by 1024.0).

- [x] 2.2 Update `SpriteDrawCall` construction in `marathon-web/src/render.rs` (around line 218-222) where entity sprites are built. Use the new world bounds from the updated `resolve_entity_sprite` return value instead of symmetric width/height.

- [x] 2.3 Rewrite billboard quad construction in `SpriteRenderer::render()` (`marathon-web/src/sprites.rs` around lines 378-414). Replace symmetric centering with asymmetric bounds: `bl = center + cam_right * world_left`, `br = center + cam_right * world_right`, `tl.y = center.y - world_top` (world_top is negative = above), `bl.y = center.y - world_bottom` (world_bottom is positive = below). Combine horizontal (cam_right) and vertical (world Y) offsets for all four corners: `bl = Vec3(center + cam_right * world_left, center.y - world_bottom)`, `tl = Vec3(center + cam_right * world_left, center.y - world_top)`, `br = Vec3(center + cam_right * world_right, center.y - world_bottom)`, `tr = Vec3(center + cam_right * world_right, center.y - world_top)`.

## 3. Action Key Dispatch Infrastructure

- [x] 3.1 Add supporting data structures for action key targeting. In `marathon-sim/src/world.rs`, extend `MapGeometry` with: `pub polygon_types: Vec<i16>` (polygon type from map data, where `_polygon_is_platform = 5`), `pub polygon_permutations: Vec<i16>` (platform index for platform polygons), `pub line_side_indices: Vec<(Option<usize>, Option<usize>)>` (clockwise and counterclockwise side indices per line). Populate these from `MapData` during `MapGeometry` construction in the level load path.

- [x] 3.2 Add a `ControlPanels` resource type in `marathon-sim/src/world_mechanics/panels.rs`. Define it as `#[derive(Resource, Default)] pub struct ControlPanels(pub Vec<ControlPanel>)`. Extend `PanelAction` with `ActivateTaggedPlatforms { tag: i16 }` variant. During `SimState` level load, iterate `MapData.sides`: for each side with control panel flag set (`flags & 0x0008 != 0`) and `control_panel_type >= 0`, create a `ControlPanel` with the side's `line_index`, `side` (0 or 1 based on which side of the line), and a `PanelAction` mapped from `control_panel_type`: 4→`ToggleLight{light_index: permutation}`, 5→`ActivatePlatform{platform_index: permutation}`, 6→`ActivateTaggedPlatforms{tag: permutation}`, 9→`ActivateTerminal{terminal_index: permutation}`. Skip types 0-3 and 7. Insert the `ControlPanels` resource into the ECS world.

- [x] 3.3 Implement `find_action_key_target()` function in a new file `marathon-sim/src/world_mechanics/action_key.rs`. The function takes player position (`Vec2`), player facing angle (`f32`), current polygon index (`usize`), `&MapGeometry`, and `&ControlPanels`. Algorithm: (1) compute destination point = pos + Vec2(facing.cos(), facing.sin()) * 3.0; (2) starting from current polygon, find the edge the ray crosses using cross-product tests against each polygon edge; (3) at each crossed line, check if adjacent polygon is a platform type (`polygon_types[adj] == 5`) and is a door — return `Target::Platform(polygon_permutations[adj])`; (4) check if line has a control panel side (look up `line_side_indices` then check `ControlPanels` for matching `line_index`) within 1.5 WU distance — return `Target::Panel(panel_index)`; (5) continue to adjacent polygon; (6) return `Target::None` if nothing found. Define `enum ActionTarget { None, Platform(usize), Panel(usize) }`.

- [x] 3.4 Add `pub mod action_key;` to `marathon-sim/src/world_mechanics/mod.rs` and re-export the function and types.

## 4. Action Key Tick Integration

- [x] 4.1 In `marathon-sim/src/tick.rs`, add action key processing after `update_platforms()` and before `update_items()`. When `input.action_flags & ActionFlags::ACTION != 0`, call `find_action_key_target()` with the player's position, facing, and current polygon. On `Target::Platform(idx)`: find the platform entity with matching `polygon_index` and call platform activation logic (set state to Extending if AtRest, reverse if moving and reversible). On `Target::Panel(idx)`: look up the panel in `ControlPanels` and execute its action.

- [x] 4.2 Implement panel action execution within the tick loop or as a helper function. For `ActivatePlatform { platform_index }`: find the platform entity by polygon_index matching and toggle its state. For `ToggleLight { light_index }`: toggle the light's active state in the Light component. For `ActivateTaggedPlatforms { tag }`: iterate all platforms, activate those whose tag matches. For `ActivateTerminal { terminal_index }`: push a `SimEvent::TerminalActivated(terminal_index)` to the event queue.

- [x] 4.3 Add `PLATFORM_ACTIVATE_ON_ACTION_KEY` check to platform activation from the action key path. When the action key targets a platform, verify the platform's `activation_flags & PLATFORM_ACTIVATE_ON_ACTION_KEY != 0` before activating. Also verify the platform is a door and is a legal target (not already activated once if `PLATFORM_ACTIVATES_ONLY_ONCE` is set).

## 5. Verification

- [x] 5.1 Build the project in Docker (`docker run --rm -v $(pwd):/app -w /app rust:1.82-slim cargo build --workspace`) and verify no compilation errors.

- [x] 5.2 Visually verify weapon overlay: deploy to web, load a level with fists equipped, confirm only the fist knuckles are visible at the bottom of the screen (not the full arm/body sprite). Compare against original Marathon screenshot.

- [x] 5.3 Visually verify sprite anchoring: check that dropped weapons and items on the ground appear correctly positioned (not floating or sinking), and that asymmetric sprites like scenery objects are visually anchored at their origin point.

- [x] 5.4 Functionally verify action key: load a level with manual doors (e.g., Arrival), walk up to a door, press Space, and confirm the door opens. Find a light switch panel and confirm it toggles the light.
