---
tags: [tier-3, content-pipeline, lua, scripting, vm]
status: research-complete
---

# Lua VM Integration

## Overview

Aleph One exposes a comprehensive Lua scripting API that allows scenarios and plugins to modify gameplay, render custom HUDs, and track statistics. There are three distinct script types, each with its own API surface and lifecycle. Lua scripting is the most complex integration point in the content pipeline, as it touches nearly every game subsystem.

Aleph One uses Lua 5.3, embedded via the C API. The Rust rebuild should use the `mlua` crate.

## Script Types

### 1. Solo Lua (Gameplay Scripts)

**Purpose:** Modify gameplay logic in single-player and cooperative games. Can spawn monsters, manipulate player state, respond to events, control cameras, modify level geometry, and more.

**Lifecycle:**
- Script is loaded and `init(restoring_game)` is called at level start
- `idle()` called every tick before physics
- `postidle()` called every tick after physics but before rendering
- `cleanup()` called at level end
- Script state is destroyed on level change (but `Game.restore_passed()` can pass data across levels)

**Write Access System:** Solo Lua plugins declare what subsystems they can modify via `<write_access>` elements in Plugin.xml:

| Access Level | Meaning | Exclusivity |
|---|---|---|
| `world` | Full game world access (monsters, players, polygons, etc.) | Exclusive with all other world scripts |
| `fog` | Can modify fog settings | Exclusive with other fog scripts |
| `music` | Can control music playback | Exclusive with other music scripts |
| `overlays` | Can draw overlay graphics | Exclusive with other overlay scripts |
| `ephemera` | Can create transient visual effects | Shared (multiple ephemera scripts can coexist) |
| `sound` | Can play sounds | Shared (multiple sound scripts can coexist) |

A `world` script implicitly claims `fog`, `music`, and `overlays` exclusivity. Only one script per exclusive access level can be active. Scripts with only `ephemera` and/or `sound` access can run alongside other scripts.

### 2. HUD Lua (Interface Rendering)

**Purpose:** Custom HUD rendering -- replaces the built-in HUD with Lua-drawn elements. Controls health bars, ammo displays, motion sensor, compass, inventory, and any custom overlays.

**Lifecycle:**
- `Triggers.init()` called when game session starts
- `Triggers.draw()` called every frame (screen cleared before each call)
- `Triggers.resize()` called on window size change
- `Triggers.cleanup()` called when session ends

**Exclusivity:** Only one HUD Lua script can be active at a time (last enabled plugin wins).

### 3. Stats Lua (Statistics)

**Purpose:** Track game statistics for network games. Generates end-of-game reports.

**Exclusivity:** Only one Stats Lua script can be active at a time.

### 4. Achievements Lua

**Purpose:** Achievement tracking system (added in later Aleph One versions).

### 5. Embedded Lua

**Purpose:** Lua scripts embedded directly in map files (level-specific scripts).

### 6. Network Lua (Netscript)

**Purpose:** Custom network game mode logic.

## Solo Lua API -- Complete Reference

### Triggers

Scripts define these as functions that the engine calls:

```lua
Triggers = {}

function Triggers.init(restoring_game)
  -- Level start. restoring_game is true when loading a saved game.
end

function Triggers.cleanup()
  -- Level end. Last chance to modify scores.
end

function Triggers.idle()
  -- Every tick, before physics calculations.
end

function Triggers.postidle()
  -- Every tick, after physics but before rendering.
end

function Triggers.start_refuel(class, player, side)
  -- Player activates refuel panel.
  -- class: "single shield", "double shield", "triple shield", "oxygen"
end

function Triggers.end_refuel(class, player, side)
  -- Player stops using refuel panel.
end

function Triggers.tag_switch(tag, player, side)
  -- Player toggles a tag switch. Return true to allow.
end

function Triggers.light_switch(light, player, side)
  -- Player toggles a light switch. Return true to allow.
end

function Triggers.platform_switch(polygon, player, side)
  -- Player activates platform switch. Return true to allow.
end

function Triggers.projectile_switch(projectile, side)
  -- Projectile hits a switch.
end

function Triggers.terminal_enter(terminal, player)
  -- Player starts using terminal. Return true to allow.
end

function Triggers.terminal_exit(terminal, player)
  -- Player exits terminal.
end

function Triggers.pattern_buffer(side, player)
  -- Player uses pattern buffer. Return true to allow.
end

function Triggers.got_item(type, player)
  -- Player picks up item. Return true to allow.
end

function Triggers.light_activated(light)
  -- Light state changes.
end

function Triggers.platform_activated(polygon)
  -- Platform state changes.
end

function Triggers.player_revived(player)
  -- Player revives in netgame.
end

function Triggers.player_killed(player, aggressor_player, action, projectile)
  -- Player dies.
end

function Triggers.player_damaged(victim, aggressor_player, aggressor_monster,
                                  damage_type, damage_amount, projectile)
  -- Player takes damage. Return true to allow.
end

function Triggers.monster_damaged(monster, aggressor_monster,
                                   damage_type, damage_amount, projectile)
  -- Monster takes damage. Return true to allow.
end

function Triggers.monster_killed(monster, aggressor_player, projectile)
  -- Monster dies.
end

function Triggers.item_created(item)
  -- Item placed on ground.
end

function Triggers.projectile_detonated(type, owner, polygon, x, y, z)
  -- Projectile explodes (after area-of-effect damage).
end
```

### Global Tables

#### Players
```lua
#Players          -- Player count
Players()         -- Iterator
Players[index]    -- Access by index (0-based)
Players.print(msg) -- Broadcast message to all players
```

**Player object fields:**

| Field | Type | Access | Description |
|-------|------|--------|-------------|
| `.name` | string | R | Player name |
| `.color` | mnemonic | R | Shirt color |
| `.team` | mnemonic | R | Team assignment |
| `.dead` | boolean | R | Is dead |
| `.local_` | boolean | R | Is local player |
| `.disconnected` | boolean | R | Disconnected from network |
| `.energy` / `.life` | number | RW | Suit energy (150 = full) |
| `.oxygen` | number | RW | Oxygen (max 10800) |
| `.direction` / `.yaw` | number | RW | Facing angle |
| `.elevation` / `.pitch` | number | RW | Vertical look angle |
| `.x`, `.y`, `.z` | number | R | Position |
| `.polygon` | polygon | R | Current floor polygon |
| `.monster` | monster | R | Corresponding monster object |
| `.points` | number | RW | Score |
| `.deaths` | number | RW | Non-player death count |
| `.kills[player]` | number | RW | Kill count vs specific player |
| `.items[type]` | number | RW | Inventory count |
| `.zoom_active` | boolean | R | Sniper zoom state |
| `.feet_below_media` | boolean | R | Standing in liquid |
| `.head_below_media` | boolean | R | Submerged |
| `.invincibility_duration` | number | RW | Ticks remaining |
| `.invisibility_duration` | number | RW | Ticks remaining |
| `.infravision_duration` | number | RW | Ticks remaining |
| `.extravision_duration` | number | RW | Ticks remaining |

**Player object methods:**

| Method | Description |
|--------|-------------|
| `:damage(amount [, type])` | Inflict damage |
| `:fade_screen(type)` | Trigger screen fade effect |
| `:find_action_key_target()` | Detect nearby switches/panels |
| `:find_target()` | Raycast detection |
| `:play_sound(sound, pitch)` | Play sound for this player only |
| `:position(x, y, z, polygon)` | Teleport player |
| `:print(message)` | Show HUD message |
| `:teleport(polygon)` | Teleport to polygon center |
| `:teleport_to_level(level)` | Level jump |

**Player sub-objects:**
- `.action_flags` -- Input state (action_trigger, cycle_weapons_backward/forward, left_trigger, right_trigger, toggle_map, microphone_button)
- `.compass` -- Navigation display (.beacon, .lua, .ne, .nw, .se, .sw, .x, .y; methods :all_on(), :all_off())
- `.crosshairs` -- (.active)
- `.external_velocity` -- (.i, .j, .k / .x, .y, .z)
- `.internal_velocity` -- (.forward, .perpendicular)
- `.texture_palette` -- (.highlight, .size, .slots[n])
- `.weapons` -- (.active, .current, .desired, .weapons[type])

#### Monsters
```lua
#Monsters                            -- Max monster count
Monsters()                           -- Iterator
Monsters.new(x, y, height, polygon, type) -- Create monster
```

**Monster fields:** `.action` (R), `.facing`/`.yaw` (RW), `.life`/`.vitality` (RW), `.mode` (R), `.player` (R), `.polygon` (R), `.type` (R), `.valid` (R), `.visible` (RW), `.x`/`.y`/`.z` (R), `.external_velocity` (RW), `.vertical_velocity` (RW)

**Monster methods:** `:accelerate()`, `:attack(target)`, `:damage()`, `:move_by_path(polygon)`, `:play_sound()`, `:position()`

#### Projectiles
```lua
#Projectiles
Projectiles()
Projectiles.new(x, y, z, polygon, type)
```

**Fields:** `.damage_scale` (RW), `.dz` (RW), `.elevation`/`.pitch` (RW), `.facing`/`.yaw` (RW), `.owner` (R), `.polygon` (R), `.target` (RW), `.type` (R), `.x`/`.y`/`.z` (R)

**Methods:** `:delete()`, `:play_sound()`, `:position()`

#### Items
```lua
#Items
Items()
Items.new(x, y, height, polygon, type)
```

**Fields:** `.facing` (RW), `.polygon` (R), `.type` (R), `.x`/`.y`/`.z` (R)

**Methods:** `:delete()`, `:play_sound()`, `:position()`

#### Effects
```lua
#Effects
Effects()
Effects.new(x, y, height, polygon, type)
```

**Fields:** `.facing` (RW), `.polygon` (R), `.type` (R), `.x`/`.y`/`.z` (R)

**Methods:** `:delete()`, `:play_sound()`, `:position()`

#### Polygons
```lua
#Polygons
Polygons()
```

**Fields:** `.adjacent_polygons[n]`, `.area` (R), `.ceiling` / `.floor` (sub-object with .collection, .height/.z, .light, .texture_index, .texture_x, .texture_y, .transfer_mode), `.endpoints[n]`, `.lines[n]`, `.media` (RW), `.permutation` (R), `.sides[n]`, `.type` (RW), `.x`/`.y`/`.z` (R)

**Methods:** `:adjacent_polygons()`, `:contains(x, y [, z])`, `:endpoints()`, `:find_line_crossed_leaving()`, `:lines()`, `:monsters()`, `:play_sound()`, `:sides()`

#### Lines
```lua
#Lines
Lines()
```

**Fields:** `.clockwise_polygon`/`.cw_polygon` (R), `.clockwise_side`/`.cw_side` (R), `.counterclockwise_polygon`/`.ccw_polygon` (R), `.counterclockwise_side`/`.ccw_side` (R), `.endpoints[n]` (R), `.has_transparent_side` (R), `.highest_adjacent_floor` (R), `.length` (R), `.lowest_adjacent_ceiling` (R), `.solid` (R)

#### Sides
```lua
#Sides
Sides()
Sides.new(polygon, line)
```

**Fields:** `.control_panel` (RW boolean), `.can_be_destroyed` (RW), `.light_dependent` (RW), `.only_toggled_by_weapons` (RW), `.repair` (RW), `.type` (R), `.uses_item` (R), `.primary`/`.secondary`/`.transparent` (texture sub-objects with .collection, .empty, .light, .texture_index, .texture_x, .texture_y, .transfer_mode), `.line` (R), `.polygon` (R)

**Methods:** `:play_sound()`, `:recalculate_type()`

#### Endpoints
```lua
#Endpoints
Endpoints()
```

**Fields:** `.x`, `.y` (R)

#### Platforms
```lua
#Platforms
Platforms()
```

**Fields:** `.active` (RW), `.ceiling_height` (RW), `.contracting` (R), `.door` (R), `.extending` (R), `.floor_height` (RW), `.locked` (RW), `.monster_controllable` (RW), `.player_controllable` (RW), `.polygon` (R), `.secret` (R), `.speed` (RW), `.type` (R)

#### Lights
```lua
#Lights
Lights()
Lights.new([preset])
```

**Fields:** `.active` (RW), `.tag` (RW), `.initial_phase` (RW), `.initially_active` (RW), `.intensity` (R, 0-1), `.states[state]` (sub-object with .delta_intensity, .delta_period, .intensity, .light_function, .period)

#### Tags
**Fields:** `.active` (RW)

#### Media
```lua
#Media
Media()
```

**Fields:** `.direction` (RW), `.height` (R), `.high` (RW), `.light` (RW), `.low` (RW), `.speed` (RW), `.type` (R)

#### Level
**Fields:** `.completed` (R), `.extermination`/`.exploration`/`.low_gravity`/`.magnetic`/`.rebellion`/`.repair`/`.rescue`/`.retrieval`/`.vacuum` (R boolean flags), `.name` (R), `.fog`/`.underwater_fog` (sub-object with .active, .present, .affects_landscapes, .color {.r, .g, .b}, .depth)

**Methods:** `:calculate_completion_state()`

#### Game
**Fields:** `.difficulty` (R), `.kill_limit` (R), `.monsters_replenish` (RW), `.proper_item_accounting` (RW), `.time_remaining` (R), `.scoring_mode` (RW), `.over` (W), `.ticks` (R), `.type` (R), `.version` (R)

**Methods:** `:global_random(n)`, `:local_random(n)`, `:random(n)`, `:restore_passed()`, `:restore_saved()`, `:save()`

#### Cameras
```lua
#Cameras
Cameras()
Cameras.new()
```

**Methods:** `:activate(player)`, `:clear()`, `:deactivate()`, `.path_angles:new(yaw, pitch, time)`, `.path_points:new(x, y, z, polygon, time)`

#### Additional Read-Only Tables
- `Annotations` -- Map annotations (.polygon, .text, .x, .y)
- `Goals` -- Goal points (.facing, .polygon, .id, .x, .y, .z)
- `ItemStarts` -- Item spawn points
- `MonsterStarts` -- Monster spawn points
- `PlayerStarts` -- Player spawn points
- `Terminals` -- Terminal objects
- `SoundObjects` -- Ambient sound sources

#### Music
**Methods:** `:clear()`, `:fade(duration)`, `:play(track1, ...)`, `:stop()`, `:valid(track1, ...)`

#### Utility
- `CollectionsUsed = {id1, id2, ...}` -- Request collection loading
- Custom fields on any userdata: `object._my_field = value` (underscore prefix)

### Mnemonics/Enumerations

All game constants are accessed as mnemonic strings rather than numeric IDs:

- **MonsterTypes** (47): "player", "minor tick", "major tick", "kamikaze tick", "minor compiler", "major compiler", ..., "explodavacbob"
- **ProjectileTypes** (39): "missile", "grenade", "pistol bullet", ..., "smg bullet"
- **ItemTypes** (36): "knife", "pistol", "pistol ammo", ..., "smg ammo"
- **WeaponTypes** (10): "fist", "pistol", "fusion pistol", "assault rifle", "missile launcher", "flamethrower", "alien weapon", "shotgun", "ball", "smg"
- **DamageTypes** (24): "explosion", "staff", "projectile", "absorbed", "flame", "claws", ..., "shotgun"
- **EffectTypes** (72): "rocket explosion", "rocket contrail", ..., "assimilated civilian fusion blood splash"
- **Collections** (32): "interface", "weapons in hand", "juggernaut", "tick", "explosions", ..., "cyborg"
- **Sounds** (214+): "startup", "teleport in", "teleport out", ..., "vacbob kill the player"
- **PolygonTypes** (24): "normal", "item impassable", "monster impassable", "hill", ..., "superglue"
- **PlatformTypes** (9): "spht door", "spht split door", ..., "pfhor platform"
- **GameTypes** (9): "kill monsters", "cooperative play", "capture the flag", ..., "netscript"
- **DifficultyTypes** (5): "kindergarten", "easy", "normal", "major damage", "total carnage"
- **FadeTypes** (33): "start cinematic fade in", ..., "tint gross", "tint jjaro"
- **TransferModes** (12): "normal", "pulsate", "wobble", ..., "fast wander"
- **MonsterActions** (12): "stationary", "waiting to attack again", "moving", ..., "teleporting out"
- **MonsterModes** (5): "locked", "losing lock", "lost lock", "unlocked", "running"
- **MonsterClasses** (16): "player", "bob", "madd", ..., "yeti"
- **ScoringModes** (4): "most points", "most time", "least points", "least time"
- **LightFunctions** (4): "constant", "linear", "smooth", "flicker"
- **LightPresets** (3): "normal", "strobe", "media"
- **LightStates** (6): "becoming active", "primary active", ..., "secondary inactive"
- **PlayerColors** (8): "slate", "red", "violet", "yellow", "white", "orange", "blue", "green"

## HUD Lua API -- Complete Reference

### Triggers

```lua
Triggers = {}
function Triggers.init() end      -- Session start
function Triggers.cleanup() end   -- Session end
function Triggers.resize() end    -- Window resize
function Triggers.draw() end      -- Every frame (screen cleared before call)
```

### Screen Object

**Read-only properties:** `.width`, `.height`, `.renderer`, `.map_active`, `.term_active`, `.crosshairs.active`, `.hud_size_preference`, `.term_size_preference`

**Modifiable properties:** `.field_of_view` (.horizontal, .vertical, .fix_h_not_v), `.masking_mode`, `.clip_rect`, `.world_rect`, `.map_rect`, `.term_rect`

**Drawing methods:**
- `:fill_rect(x, y, w, h, color)` -- Solid rectangle
- `:frame_rect(x, y, w, h, color, thickness)` -- Outlined rectangle
- `:clear_mask()` -- Reset masking

### Fonts
```lua
local font = Fonts.new{file="mono", size=12, style=0}
-- or: Fonts.new{id=4, size=12}     -- Resource-based (4=Monaco)
-- or: Fonts.new{interface="terminal"} -- Interface font

font.line_height  -- Read-only
font:measure_text("string")  -- Returns width, height
font:draw_text("string", x, y, {r, g, b, a})
```

### Images
```lua
local img = Images.new{resource=1000}
-- or: Images.new{path="image.png", mask="mask.png"}

img.width, img.height          -- Read-only
img.crop_rect = {x=0, y=0, width=64, height=64}
img.tint_color = {1, 1, 1, 1} -- OpenGL only
img.rotation = 45              -- OpenGL only
img:rescale(new_w, new_h)
img:draw(x, y)
```

### Shapes (Game Sprites)
```lua
local shape = Shapes.new{collection=0, texture_index=0, type="wall", color_table=0}
-- Same properties as Images: width, height, crop_rect, tint_color, rotation
shape:rescale(w, h)
shape:draw(x, y)
```

### Player (Read-Only in HUD Context)

All fields from Solo Lua are available read-only, plus:
- `.inventory_sections` -- Structured inventory data
- `.motion_sensor` -- Motion sensor blip data (.active, .blips[] with .direction, .distance, .intensity, .type)
- `.weapons` -- Detailed weapon/ammo display info
- `.velocity` -- (.forward, .perpendicular, .vertical)
- `.respawn_duration` -- Ticks until revival (nil if alive)

### Game (Read-Only)
Same as Solo Lua, plus `.players[]` array with `.active`, `.color`, `.team`, `.kills`, `.ranking`, `.local_`, `.name`.

### Lighting
- `.ambient_light` (0-1), `.weapon_flash`, `.liquid_fader`, `.damage_fader`

### Colors
Tables with `.r`/`.red`/`[1]`, `.g`/`.green`/`[2]`, `.b`/`.blue`/`[3]`, `.a`/`.alpha`/`[4]` (alpha defaults to 1).

### HUD-Specific Mnemonics
- `SensorBlipTypes`: "friend", "alien", "hostile player"
- `MaskingModes`: "disabled", "enabled", "drawing", "erasing"
- `RendererTypes`: "software", "opengl", "shader"
- `SizePreferences`: "normal", "double", "largest"
- `InventorySections`: "weapons", "ammunition", "powerups", "items", "balls"
- `InterfaceFonts`, `InterfaceRects`, `InterfaceColors`

## C++ Integration Architecture (Original)

From `lua_script.h`, the C++ integration uses:

**Script type enum:**
```cpp
enum ScriptType {
    _embedded_lua_script,
    _lua_netscript,
    _solo_lua_script,
    _stats_lua_script,
    _achievements_lua_script
};
```

**Core functions:** `LoadLuaScript()`, `RunLuaScript()`, `CloseLuaScript()`, `ExecuteLuaString()`, `L_Error()`

**Hook dispatch:** `L_Call_Init()`, `L_Call_Cleanup()`, `L_Call_Idle()`, `L_Call_PostIdle()`, plus hooks for every trigger listed above.

**Mutability interface:** `LuaMutabilityInterface` provides a permissions system checked at runtime to enforce write access restrictions.

**Camera system:** `timed_point`, `timed_angle`, `lua_path`, `lua_camera` structs for scripted camera movement.

**State persistence:** Serialization/deserialization for saved games.

## Current State in Rust Rebuild

**Existing code references:** The Rust codebase has no Lua VM integration. The `plugin.rs` parser recognizes Lua script paths (`hud_lua`, `solo_lua`, `stats_lua`) and write access flags (`SoloLuaWriteAccess` bitflags) in Plugin.xml, but does not load or execute scripts.

**Relevant files:**
- `marathon-formats/src/plugin.rs` -- Parses Lua script references and write access
- No Lua VM, no script loading, no trigger dispatch

## Gaps and Implementation Plan

### Phase 1: Lua VM Setup
- Add `mlua` crate dependency with `lua54` and `vendored` features
- Create `marathon-scripting` crate to house the Lua integration
- Initialize Lua VM, configure sandboxing (remove os, io, debug libraries for security)

### Phase 2: Mnemonic Registry
- Build a compile-time or init-time registry mapping all mnemonic strings to numeric IDs
- Expose as global tables in Lua (e.g., `MonsterTypes["minor tick"] = 1`)

### Phase 3: Game Object Userdata
- Implement `UserData` trait for each game object type (Player, Monster, Projectile, etc.)
- Use `UserDataFields` for properties, `UserDataMethods` for methods
- Implement read/write access checking via the mutability interface

### Phase 4: Trigger Dispatch
- Define a `TriggerDispatcher` that calls named Lua functions at appropriate game events
- Wire into the game loop: idle/postidle per tick, event hooks on damage/kill/switch/etc.
- Handle return values (some triggers return boolean to allow/deny actions)

### Phase 5: HUD Lua Rendering
- Expose Screen, Fonts, Images, Shapes objects to HUD scripts
- Bridge drawing commands to the wgpu rendering pipeline
- Handle the draw-every-frame lifecycle

### Phase 6: Script Isolation and Lifecycle
- Separate Lua VMs for solo, HUD, and stats scripts (or use separate environments within one VM)
- Implement script loading from plugins and embedded map data
- Handle saved game serialization of Lua state

## Recommended Rust Crates

### Primary: `mlua`
- **Version:** Latest (actively maintained as of 2026)
- **Features needed:** `lua54`, `vendored`, `send`, `serialize` (for serde integration)
- **Key capabilities:**
  - `Lua::new()` -- Create VM
  - `lua.create_function()` -- Register Rust functions callable from Lua
  - `lua.scope()` -- Temporary callbacks without `'static` requirement
  - `UserData` trait -- Expose Rust types as Lua objects with fields and methods
  - `lua.globals().set()` -- Set global tables
  - `lua.load(code).exec()` -- Execute Lua code
  - Serde integration for serialization
- **WASM support:** Via `wasm32-unknown-emscripten` target (not `wasm32-unknown-unknown`)
- **Thread safety:** Available with `send` feature (uses reentrant mutex)

### Alternative Considered: `rlua`
- Predecessor to `mlua`, same author
- Less maintained, fewer features
- **Recommendation:** Use `mlua` instead

### Supporting Crates
- `serde` -- For Lua state serialization (saved games)
- `thiserror` -- Error types for script failures

## WASM Considerations

The marathon-web crate currently targets `wasm32-unknown-unknown` with wasm-bindgen, web-sys, and wgpu. mlua supports WASM only through `wasm32-unknown-emscripten`, which is **incompatible** with wasm-bindgen and our web rendering stack. Switching to emscripten is not viable.

**See [[lua-in-rust-options]] for the full evaluation of all Lua-in-Rust options.**
**See [[lua-wasm-architecture]] for the recommended dual-target architecture.**

Summary of findings:
1. **mlua cannot target wasm32-unknown-unknown** due to Lua C source's setjmp/longjmp dependency
2. **Piccolo** (pure Rust) works on wasm32-unknown-unknown but is missing critical stdlib (string.format, string.find/match/gsub)
3. **lua-rs** (CppCXY/lua-rs, pure Rust Lua 5.5) is the most promising option -- works on wasm32-unknown-unknown with near-complete stdlib (28/30 official tests pass), but is very new (44 stars, single maintainer)
4. **Recommended approach:** Evaluate lua-rs as a unified VM for both targets, with mlua as a native-only fallback
5. **Rhai** has excellent WASM support but is not Lua-compatible (different language entirely)
6. **Wasmoon** (Lua 5.4 via JS) is a last-resort fallback via a JS bridge from Rust WASM

## Related Notes

- [[mml-override-system]] -- MML and Lua interact (console lua flag, level scripts reference both)
- [[plugin-system-patching]] -- Plugins declare and deliver Lua scripts
- [[community-content-ecosystem]] -- Major scenarios depend heavily on Lua scripting
