## Context

The Rust Marathon engine renders the 3D world visually but three systems diverge from the original C++ Aleph One engine behavior:

1. **Weapon overlay**: The current `WeaponOverlayRenderer` places the entire weapon sprite above the HUD bar using fixed NDC coordinates (`ndc_half_w=0.50`, bottom anchored at `-1.0 + hud_ndc`). The C++ engine uses `position_sprite_axis()` with `_position_center` mode, where `idle_height ≈ FIXED_ONE + FIXED_ONE/15` (1.067) places the sprite origin at ~107% of screen height — clipping most of the sprite below the viewport so only the business end is visible.

2. **Action key dispatch**: `ActionFlags::ACTION` (bit 10) is defined and mapped to Space, but the tick loop never reads it. The C++ engine calls `update_action_key()` every tick, which ray-casts via `find_action_key_target()` through polygons to find platforms (doors) and control panels. The Rust `panels.rs` module has `ControlPanel` structs and `can_activate_panel()` but they are never spawned from map data or checked in the tick loop.

3. **Sprite billboards**: `resolve_entity_sprite()` computes `width = abs(world_right - world_left)` and `height = abs(world_bottom - world_top)`, losing the asymmetric offset from the sprite origin. The billboard then extends symmetrically from center and fully upward from position. The C++ engine projects `world_left`/`world_right`/`world_top`/`world_bottom` individually from the object origin, preserving asymmetry.

## Goals / Non-Goals

**Goals:**
- First-person weapon sprites clip correctly, matching C++ `position_sprite_axis` behavior for `_position_center` mode
- Pressing Space activates doors (platforms with `ACTIVATE_ON_ACTION_KEY`) and control panels via polygon-traversal ray-cast
- Control panels spawn from map side data and trigger platform/light/terminal actions
- World sprite billboards use asymmetric world bounds for correct anchoring

**Non-Goals:**
- `_position_low` / `_position_high` positioning modes (only dual-wield weapons use these; fists/pistol/etc. all use `_position_center`)
- Weapon bob animation, kick recoil, or state-based height transitions (these can be added later; focus is on correct idle positioning)
- Transfer mode rendering (invisibility, static, teleport fold effects)
- Recharge panels (oxygen/shield refueling requires continuous activation state tracking)
- Pattern buffer (save game) panels
- Computer terminal panel UI (terminal display is a separate system; we emit the event only)

## Decisions

### 1. Reimplement `position_sprite_axis` for `_position_center` only

**Decision**: Translate the C++ `position_sprite_axis` function to Rust, but only the `_position_center` branch. Both vertical and horizontal positioning use this mode for all standard weapons.

**Rationale**: The `_position_low`/`_position_high` modes are only used for dual-wield sliding animations. Supporting `_position_center` covers idle, firing, recovering, lowering, and raising states. This keeps the implementation focused.

**Algorithm** (C++ → Rust translation for `_position_center`):
```
// X axis: scale_width = screen_height, screen_width = screen_width
origin_x = (screen_width * horizontal_position) / FIXED_ONE
x0 = origin_x + (world_left * screen_height) / WORLD_ONE
x1 = origin_x + (world_right * screen_height) / WORLD_ONE

// Y axis: scale_width = screen_height, screen_width = screen_height
origin_y = (screen_height * vertical_position) / FIXED_ONE
y0 = origin_y + (-world_top * screen_height) / WORLD_ONE
y1 = origin_y + (-world_bottom * screen_height) / WORLD_ONE
```

Convert to NDC by mapping `[0, screen_dim]` → `[-1, +1]`.

**Alternative considered**: Keep fixed NDC placement and just adjust the offset constant. Rejected because different weapons have different idle_heights and world bounds; a hardcoded offset would only fix fists.

### 2. Extend `WeaponRenderState` with positioning data

**Decision**: Add `vertical_position: f32` and `horizontal_position: f32` fields to `WeaponRenderState` (as normalized 0..1 floats, pre-divided by FIXED_ONE). Initially set to `idle_height / FIXED_ONE` and `idle_width / FIXED_ONE` from the weapon definition in `PhysicsTables`.

**Rationale**: The renderer needs these values to compute `position_sprite_axis`. The sim already has access to weapon definitions via `PhysicsTables`. Passing normalized floats avoids leaking fixed-point representation into the rendering layer.

**Alternative considered**: Hard-coding per-weapon offsets in the renderer. Rejected because the sim is the authority on weapon state and will need to animate these values for raise/lower/kick transitions later.

### 3. Polygon-traversal ray-cast for action key targeting

**Decision**: Implement `find_action_key_target()` in `marathon-sim` using the existing `MapGeometry` adjacency data. The algorithm walks polygons along a 2D ray from player position in the facing direction, checking each crossed line for platforms and control panels.

**Approach**:
1. Construct 2D ray: `destination = player_pos + facing_direction * MAX_ACTIVATION_RANGE` (3.0 world units)
2. Starting from player's current polygon, find which line the ray crosses leaving the polygon (cross-product edge test against each polygon edge)
3. At each crossed line:
   - Check if adjacent polygon is a platform with `PLATFORM_IS_DOOR` → return platform target
   - Check if line side has control panel within reach distance (1.5 WU) → return panel target
4. Continue to adjacent polygon until no more crossings or range exceeded

**Data needed**: `MapGeometry` already has `polygon_vertices`, `polygon_adjacency`, `line_endpoints`, `line_solid`. We need to add: per-side control panel data, per-polygon platform mapping, and line-to-side mapping.

**Alternative considered**: Simple distance check to nearest panel/platform (like item pickup). Rejected because the C++ engine requires facing direction and line-of-sight — a player shouldn't activate a door behind them.

### 4. Spawn control panels from map side data at level load

**Decision**: During `SimState::new()` (or equivalent level load), iterate `MapData.sides` and collect sides where `control_panel_type >= 0` into a `Vec<ControlPanel>` resource. Map the `control_panel_type` and `control_panel_permutation` fields to `PanelAction` variants.

**Panel type mapping**:
| `control_panel_type` | `PanelAction` |
|---|---|
| 4 (`_panel_is_light_switch`) | `ToggleLight { light_index: permutation }` |
| 5 (`_panel_is_platform_switch`) | `ActivatePlatform { platform_index: permutation }` |
| 6 (`_panel_is_tag_switch`) | `ActivateTaggedPlatforms { tag: permutation }` |
| 9 (`_panel_is_computer_terminal`) | `ActivateTerminal { terminal_index: permutation }` |
| 0-3 (recharge panels), 7 (pattern buffer) | Skipped (non-goal) |

**Alternative considered**: Full ECS entity per panel. Rejected as overkill — panels are static data queried by the ray-cast, not independently updated each tick.

### 5. Pass asymmetric world bounds through SpriteDrawCall

**Decision**: Replace `width: f32, height: f32` in `SpriteDrawCall` with `world_left: f32, world_right: f32, world_top: f32, world_bottom: f32` (all in world units, divided by 1024). Reconstruct the billboard quad using these offsets from the entity position instead of symmetric centering.

**Billboard construction**:
```
// Object position is the origin; bounds are offsets from it
left_edge  = center + cam_right * world_left   // world_left is negative
right_edge = center + cam_right * world_right   // world_right is positive
bottom     = center.y + world_bottom            // world_bottom is positive (below)
top        = center.y + world_top               // world_top is negative (above)
```

Note: In C++, `world_top` is negative (above origin) and `world_bottom` is positive (below origin). The billboard correctly positions the sprite relative to the object origin, not centered on it.

**Alternative considered**: Keep symmetric width/height and add a separate `origin_offset` field. Rejected because it's more complex and loses the direct correspondence with the shape data.

## Risks / Trade-offs

- **[Risk] Weapon positioning only covers idle state initially** → Acceptable: the weapon will render at the correct idle position immediately. Raise/lower/kick animations can be added incrementally by having the sim modify `vertical_position`/`horizontal_position` based on weapon state machine transitions. The rendering code won't need to change.

- **[Risk] Ray-cast may miss targets if polygon adjacency data is incomplete** → Mitigation: `MapGeometry.polygon_adjacency` is already used by player physics for movement collision. If it works for movement, it works for ray-casting. Add debug logging for ray-cast misses during testing.

- **[Risk] Control panel type mapping is partial (skips recharge/pattern buffer)** → Acceptable for this change. The most visible interactions (doors, switches, terminals) are covered. Recharge panels need continuous state tracking and pattern buffers need save system integration — both are independent features.

- **[Risk] Asymmetric sprite bounds may look wrong if world_left/right/top/bottom values are incorrectly parsed** → Mitigation: `LowLevelShape` already has these as `i16` fields in `marathon-formats`. Verify with a known sprite (e.g., BOB collection) that the bounds produce correct visual dimensions.
