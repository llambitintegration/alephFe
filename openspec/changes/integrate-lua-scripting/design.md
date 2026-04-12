## Context

The alephone-rust project implements a Marathon game engine in Rust with six workspace crates: format parsing (`marathon-formats`), simulation (`marathon-sim`), audio (`marathon-audio`), integration layer (`marathon-integration`), a playable binary (`marathon-game`), and a WASM web build (`marathon-web`). The simulation runs a deterministic 30Hz ECS tick loop using bevy_ecs, with player physics, monster AI, combat, and world mechanics. The format layer already parses Lua script sources from `WadTag::LuaScript` (tag `LUAS`) and plugin metadata including `solo_lua`, `hud_lua`, `stats_lua` fields with `SoloLuaWriteAccess` exclusivity resolution.

Aleph One's Lua scripting is the foundation of modern Marathon community content. Scenarios like Istoria (RPG system), Apotheosis X (custom HUD overlays), and dozens of others depend on three script types: solo scripts (gameplay hooks wired to simulation events), HUD scripts (custom UI rendering via a Screen drawing API), and stats scripts (end-of-game data collection). Without Lua support, the engine cannot run any community scenarios beyond vanilla levels.

This change introduces a new `marathon-lua` crate that owns the Lua VM lifecycle, all game object UserData type definitions, event dispatch, and the three script execution contexts. It is the largest integration effort in the project.

## Goals / Non-Goals

**Goals:**
- Lua 5.5 VM via `lua-rs` (pure Rust) compiling to both native and `wasm32-unknown-unknown`
- Solo script execution with 20+ event triggers wired into `SimWorld::tick()` system phases
- HUD script execution with a Screen drawing API called once per render frame
- Stats script execution with end-of-game data collection callbacks
- ~15 UserData types exposing hundreds of readable/writable fields mapped to ECS components
- Lua state serialization for save/load via the `slua` WAD tag
- Script loading from both embedded WAD data and plugin filesystem references
- Plugin load order and `SoloLuaWriteAccess` exclusivity rules respected

**Non-Goals:**
- Network script synchronization (multiplayer Lua is out of scope)
- Lua debugger or REPL integration
- Custom Lua standard library extensions beyond what Aleph One defines
- Modifying `marathon-formats` (WAD tags and plugin metadata parsing already exist)

## Decisions

### 1. Use `lua-rs` (CppCXY/lua-rs) as the Lua VM

**Decision:** Use `lua-rs`, a pure Rust implementation of Lua 5.5, as the single Lua backend for both native and WASM builds.

**Rationale:** The web build (`marathon-web`) targets `wasm32-unknown-unknown` with `wasm-bindgen`. The standard Rust Lua binding `mlua` wraps C Lua and only supports `wasm32-unknown-emscripten`, which is incompatible with the web build pipeline. A pure Rust implementation compiles to `wasm32-unknown-unknown` without any C toolchain or Emscripten dependency, giving us one codebase for both targets.

**Risk mitigation:** `lua-rs` is young (44 stars, single maintainer as of 2026). Fork early into the project organization, run the Aleph One Lua test corpus against it, and contribute fixes upstream. Fallback: split implementation using `mlua` for native + Piccolo or wasmoon for WASM.

**Alternative considered:** Using `mlua` for native builds and a separate WASM-compatible Lua for web. Rejected because maintaining two backends with subtly different behaviors would be a significant long-term cost.

### 2. New `marathon-lua` workspace crate

**Decision:** Create a new workspace member `marathon-lua` that owns all Lua-related code: VM lifecycle, UserData types, event dispatch, and script contexts.

**Rationale:** Lua scripting is a cross-cutting concern that touches simulation, rendering, and the shell. Isolating it in its own crate keeps the dependency graph clean: `marathon-lua` depends on `marathon-sim` (for ECS access) and `marathon-formats` (for script source types), while `marathon-game` and `marathon-web` depend on `marathon-lua` to wire it in. The sim crate itself does not depend on Lua -- the dispatch is called from outside the sim tick.

**Alternative considered:** Embedding Lua code in `marathon-sim`. Rejected because the sim crate is intentionally pure deterministic simulation with no rendering or scripting dependencies.

### 3. ECS-bridged UserData via entity index handles

**Decision:** Each Lua UserData type (Player, Monster, Polygon, etc.) stores a lightweight index or entity handle that resolves to ECS components at access time. Field getters read components; field setters write components. The UserData object itself holds no cached state.

**Rationale:** Marathon Lua scripts expect that reading `monster.vitality` returns the current value at the time of the read, not a stale snapshot. By resolving through the ECS on every access, we guarantee consistency. The performance cost is negligible -- Lua scripts access tens of fields per tick, not thousands.

**Implementation:** For entity-based types (Player, Monster, Projectile, Item, Effect), the UserData stores a `bevy_ecs::Entity` handle. For map-based types (Polygon, Line, Side, Platform, Light, Media), the UserData stores an index into the corresponding ECS resource vectors (e.g., `MapGeometry.floor_heights[polygon_index]`). For singleton types (Level, Game), no handle is needed -- they access world-level resources directly.

### 4. Event dispatch at tick system boundaries, not inside systems

**Decision:** Fire Lua callbacks between simulation system phases in `SimWorld::tick()`, not during system execution. The tick loop becomes: player physics -> [lua: player events] -> monster AI -> combat -> projectile physics -> damage resolution -> [lua: damage/kill events] -> world mechanics -> [lua: platform/light events] -> cleanup -> [lua: idle].

**Rationale:** Firing callbacks between phases means the ECS is in a consistent state when Lua code runs -- no partially-updated components. Lua scripts can safely read and write any component. This matches Aleph One's original behavior where Lua callbacks fire at defined points in the game loop.

**Alternative considered:** Firing events asynchronously by collecting events during the tick and dispatching after all systems complete. This would simplify the tick loop but would not match Aleph One's semantics where, for example, `player_damaged` fires between damage resolution and world mechanics, allowing scripts to modify damage before death is finalized.

### 5. HUD draw commands buffered, not immediate-mode rendered

**Decision:** The HUD Lua context's Screen API (fill_rect, draw_text, draw_shape, world_to_screen) buffers draw commands into a `Vec<HudDrawCommand>`. The renderer consumes this buffer each frame and translates commands to wgpu draw calls.

**Rationale:** HUD scripts run in Lua land without direct access to the GPU. Buffering commands decouples script execution from rendering, keeps the Lua-to-Rust interface simple (no wgpu types crossing the boundary), and allows the renderer to batch/optimize draw calls.

**Alternative considered:** Giving Lua scripts direct access to a canvas abstraction. Rejected because it would require complex lifetime management across the Lua/Rust boundary and would not compose well with the existing wgpu render pass structure.

### 6. Lua state serialization via lua-rs's serialization API

**Decision:** Serialize the Lua global state table to bytes using `lua-rs`'s built-in serialization support (or a custom walker that traverses the global table and serializes values). Store the bytes in `SimSnapshot` alongside ECS state. On load, deserialize into a fresh Lua VM's global table.

**Rationale:** Scenarios like Istoria maintain RPG inventory, quest flags, and NPC dialogue state in Lua globals. Without state serialization, save/load would lose all script state, breaking these scenarios. The `slua` WAD tag exists in the format layer for exactly this purpose.

**Limitations:** Functions, coroutines, and userdata cannot be serialized. This matches Aleph One's behavior -- only primitive types, strings, and tables are preserved across save/load.

### 7. Three separate Lua VM instances

**Decision:** Create three independent Lua states: one for solo scripts, one for HUD scripts, one for stats scripts. Each runs in its own `lua_rs::Lua` instance with a different set of available APIs.

**Rationale:** Aleph One enforces isolation between script types. Solo scripts can read and write game state; HUD scripts can only read game state and draw; stats scripts can only accumulate statistics. Separate VM instances enforce this by only registering the appropriate UserData types and functions in each.

## Architecture

### Crate structure

```
marathon-lua/
  Cargo.toml          # depends on lua-rs, marathon-sim, marathon-formats
  src/
    lib.rs            # public API: LuaScriptEngine, LuaScriptSources
    vm.rs             # VM lifecycle: create, configure, load, destroy
    context.rs        # ScriptContext enum (Solo, Hud, Stats)
    dispatch.rs       # Event dispatch: fire callbacks with arguments
    hud.rs            # HUD drawing API: Screen object, HudDrawCommand
    stats.rs          # Stats context callbacks
    persistence.rs    # Lua state serialization/deserialization
    objects/
      mod.rs          # Re-exports all UserData types
      player.rs       # Player UserData
      monster.rs      # Monster UserData
      projectile.rs   # Projectile UserData
      polygon.rs      # Polygon UserData
      line.rs         # Line UserData
      side.rs         # Side UserData
      platform.rs     # Platform UserData
      light.rs        # Light UserData
      media.rs        # Media UserData
      level.rs        # Level UserData (singleton)
      game.rs         # Game UserData (singleton)
      terminal.rs     # Terminal UserData
      item.rs         # Item UserData
      effect.rs       # Effect UserData
      collections.rs  # Collection accessors (Players, Monsters, Polygons, etc.)
```

### VM lifecycle

1. **Create:** During level load, `LuaScriptEngine::new(sources: &LuaScriptSources)` creates up to three Lua VMs based on which script types are present.
2. **Configure:** Each VM gets the Lua standard library loaded, then type-specific APIs registered (UserData types, global functions, constants).
3. **Load:** Script source code (from `WadTag::LuaScript` data or plugin `.lua` files) is executed to define callback functions.
4. **Execute:** During tick/frame, the engine calls named Lua functions with appropriate arguments.
5. **Save:** On save, serialize the solo VM's global state to bytes.
6. **Destroy:** On level unload, all three VMs are dropped.

### UserData type mapping (~15 types)

| Lua Type | ECS Source | Key Fields (read) | Key Fields (write) |
|----------|-----------|-------------------|---------------------|
| Player | `Player` entity components | position, polygon, yaw, pitch, health, shield, oxygen, items, weapons | health, shield, oxygen, position, yaw, pitch, teleport_to_polygon |
| Monster | `Monster` entity components | type, vitality, position, polygon, facing, action, mode, vertical_velocity | vitality, position, facing, action, mode |
| Projectile | `Projectile` entity components | type, position, polygon, facing, owner, damage_type | position, facing |
| Polygon | `MapGeometry` resource vectors | floor_height, ceiling_height, type, media, permutation, vertices, adjacent_polygons, lines, sides, platform | floor_height, ceiling_height, type, media, permutation, visible_on_automap |
| Line | `MapGeometry` resource | endpoints, length, solid, transparent, has_transparent_side, cw_polygon, ccw_polygon | solid, transparent |
| Side | Map side data | type, primary_texture, secondary_texture, transparent_texture, polygon, line, primary_lightsource, secondary_lightsource | primary_texture, secondary_texture, transparent_texture, primary_lightsource, secondary_lightsource |
| Platform | `Platform` entity | polygon, floor_height, ceiling_height, speed, is_active, is_extending, is_contracting | speed, floor_height, ceiling_height, activate, deactivate |
| Light | `Light` entity | index, active, intensity, phase | active, intensity |
| Media | `Media` entity | type, height, light, current_direction, current_magnitude | height |
| Level | World-level resources | name, index, map_checksum, difficulty, game_type, player_count, initial_random_seed, environment_code | fog_active, fog_color, fog_depth, underwater_fog_active, underwater_fog_color, underwater_fog_depth |
| Game | World-level resources | ticks, version, difficulty, type, scoring_mode, kill_limit, time_remaining | |
| Terminal | Map terminal data | index, text | |
| Item | `Item` entity | type, position, polygon | position |
| Effect | `Effect` entity | type, position, polygon | position |
| Players/Monsters/Polygons/etc. | Collection iterators | `#` (length), `[index]` (access), `__call` (iterator) | |

### Solo script event triggers (20+)

| Event | Trigger Point | Arguments |
|-------|--------------|-----------|
| `idle()` | End of every tick | (none) |
| `init()` | After script load | (none) |
| `cleanup()` | Before script unload | (none) |
| `start_refueling(type)` | Player enters refueling polygon | refuel type |
| `end_refueling(type)` | Player leaves refueling polygon | refuel type |
| `tag_switch_activated(tag)` | Tag switch hit | tag index |
| `light_activated(light)` | Light switch toggled | light index |
| `platform_activated(polygon)` | Platform triggered | polygon index |
| `terminal_enter(terminal, player)` | Player activates terminal | terminal, player |
| `terminal_exit(terminal, player)` | Player exits terminal | terminal, player |
| `pattern_buffer(player)` | Player uses save station | player |
| `got_item(type, player)` | Player picks up item | item type, player |
| `light_activated(light)` | Light state changes | light UserData |
| `projectile_detonated(type, owner, polygon, position)` | Projectile hits | type, owner, polygon, position |
| `projectile_switch(projectile, side)` | Projectile hits switch | projectile, side |
| `projectile_created(projectile)` | Projectile spawned | projectile |
| `monster_killed(monster, aggressor, projectile)` | Monster dies | monster, aggressor, projectile |
| `monster_damaged(monster, aggressor, damage_type, damage_amount, projectile)` | Monster takes damage | monster, aggressor, type, amount, projectile |
| `player_damaged(player, aggressor, damage_type, damage_amount, projectile)` | Player takes damage | player, aggressor, type, amount, projectile |
| `player_killed(player, aggressor, damage_type, projectile)` | Player dies | player, aggressor, type, projectile |

### HUD drawing API (Screen object)

The `Screen` global provides drawing primitives:

- `Screen.fill_rect(x, y, w, h, color)` -- Fill a rectangle
- `Screen.frame_rect(x, y, w, h, color, width)` -- Draw rectangle outline
- `Screen.draw_text(text, x, y, font, color, style)` -- Draw text string
- `Screen.draw_shape(shape, x, y)` -- Draw a Shapes bitmap
- `Screen.world_to_screen(x, y, z)` -- Project world coordinates to screen
- `Screen.clip_rect(x, y, w, h)` / `Screen.unclip_rect()` -- Set/clear clipping region
- `Screen.width()` / `Screen.height()` -- Screen dimensions

Color constants: `Screen.colors.white`, `Screen.colors.red`, etc.
Font constants: `Screen.fonts.interface`, `Screen.fonts.computer`, etc.

### Stats context

Stats scripts receive callbacks at game end:
- `got_kill(aggressor, victim, damage_type)` -- Kill recorded
- `player_damaged(victim, aggressor, damage_type, amount)` -- Damage recorded
- `game_ended()` -- Game over, finalize stats
- `draw()` -- Render stats display (uses same Screen API as HUD)

### WASM compatibility

`lua-rs` is pure Rust with no C dependencies, no `build.rs` complications, and no platform-specific code. It compiles directly to `wasm32-unknown-unknown` alongside the rest of the workspace. The `marathon-lua` crate uses `#[cfg(target_arch = "wasm32")]` only if any platform-specific adaptation is needed (e.g., file I/O for plugin loading is replaced by pre-loaded byte buffers on WASM).

## Risks / Trade-offs

**[lua-rs maturity]** -- The crate is young with a single maintainer. Aleph One scripts may exercise edge cases in the Lua 5.5 implementation. Mitigation: fork early, run the community test corpus, contribute fixes.

**[UserData type volume]** -- ~15 types with hundreds of field accessors is a large surface area. Mitigation: generate accessor boilerplate with macros; test each type with field access round-trip tests.

**[Event dispatch ordering]** -- Lua callbacks fire between system phases, which means the tick is no longer a simple sequential system chain. Mitigation: clearly document the extended tick ordering; test that callbacks fire at the correct points.

**[State serialization fidelity]** -- Lua functions and coroutines cannot be serialized. Scripts that store functions in globals will lose them on save/load. Mitigation: document this limitation (matches Aleph One behavior).

**[Performance of per-access ECS resolution]** -- Each field read/write on a UserData object queries the ECS. Mitigation: Marathon Lua scripts are lightweight (tens of accesses per tick, not thousands); profiling can identify hotspots if needed.

## Open Questions

- What is the exact `lua-rs` crate name on crates.io vs. a git dependency? May need to use a git dependency or forked version.
- Should the Lua VM execution be time-limited (e.g., instruction count timeout) to prevent infinite loops in broken scripts? Aleph One has no such limit, but a safety mechanism could be valuable.
- Should HUD draw commands support batching text draws for performance, or is naive per-command rendering sufficient? Depends on how complex community HUD scripts get.
