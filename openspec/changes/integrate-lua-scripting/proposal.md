## Why

Aleph One's Lua scripting system is the foundation of modern Marathon community content. Scenarios like Istoria (full RPG system built in solo Lua), Apotheosis X (custom HUD overlays), and dozens of others depend on three script types: solo scripts (gameplay hooks for events like `monster_killed`, `player_damaged`, `idle`, `projectile_created`, etc.), HUD scripts (custom UI rendering via the Screen drawing API), and stats scripts (end-of-game statistics). Without Lua support, the Rust engine cannot run any of these scenarios -- it is limited to vanilla Marathon levels.

The Lua API exposes roughly 15 game object types (Players, Monsters, Projectiles, Polygons, Lines, Sides, Platforms, Lights, Level, Game, etc.) with hundreds of readable and writable fields plus methods. Scripts make heavy use of `string.format`, pattern matching, and the Lua standard library. Embedded Lua source is stored in WAD files via the `LUAS` tag (already parsed as `WadTag::LuaScript` in marathon-formats) and referenced by plugins via `PluginMetadata`'s `solo_lua`, `hud_lua`, and `stats_lua` fields.

The key technical decision is to use `lua-rs` (CppCXY/lua-rs), a pure Rust implementation of Lua 5.5. This is required because marathon-web targets `wasm32-unknown-unknown` with `wasm-bindgen`, and `mlua` (the standard Rust Lua binding wrapping C Lua) only supports `wasm32-unknown-emscripten`, which is incompatible with our web build pipeline. A pure Rust implementation compiles to `wasm32-unknown-unknown` without any C toolchain or Emscripten dependency.

Risk: `lua-rs` is young (44 stars, single maintainer as of 2026). Mitigation: fork early into the project organization, run the Aleph One Lua test corpus against it, and contribute fixes upstream. Fallback: use `mlua` for native builds + Piccolo or wasmoon for WASM (split implementation), accepting the maintenance cost of two Lua backends.

This is the largest integration effort in the project -- approximately 15 UserData types with hundreds of field accessors, three distinct script execution contexts with different APIs, an event dispatch system wired into the simulation tick, and a HUD drawing API bridged to the renderer.

## What Changes

- **New `marathon-lua` crate**: A workspace crate that owns the Lua VM lifecycle, all UserData type definitions, event dispatch, and the script execution contexts. Depends on `lua-rs` for the VM and on `marathon-sim` for ECS access.
- **Lua VM lifecycle management**: Initialize a Lua state per script type (solo, HUD, stats) during level load. Load embedded scripts from `WadTag::LuaScript` data and plugin-referenced `.lua` files. Destroy states on level unload.
- **Game object UserData types**: Implement Lua-accessible wrappers for the ~15 Aleph One object types: `Player`, `Monster`, `Projectile`, `Polygon`, `Line`, `Side`, `Platform`, `Light`, `Media`, `Level`, `Game`, `Terminal`, `Item`, `Effect`, and collection accessors (`Players`, `Monsters`, `Polygons`, etc.). Each type exposes readable/writable fields that map to ECS components (e.g., `monster.vitality` reads/writes `Health`, `polygon.floor_height` reads/writes floor height in `MapGeometry`).
- **Event dispatch into solo scripts**: After each simulation system phase in `SimWorld::tick()`, fire the corresponding Lua callbacks: `idle()` every tick, `monster_killed(monster, aggressor, projectile)` on death, `player_damaged(player, aggressor, damage_type, damage_amount)` on damage, `projectile_created(projectile)` on spawn, `platform_activated(polygon)` on platform trigger, `light_activated(light)` on switch, `terminal_enter(terminal, player)` / `terminal_exit(terminal, player)` on terminal use, and others.
- **HUD script execution context**: Call `draw()` once per frame from the render loop, providing the Screen drawing API (`Screen.fill_rect`, `Screen.draw_text`, `Screen.draw_shape`, `Screen.world_to_screen`, color/font constants). HUD scripts read game state but do not write it.
- **Stats script execution context**: Call stats callbacks at game end (`got_kill`, `player_damaged`, `game_ended`). Stats scripts accumulate data but do not modify game state.
- **Lua state serialization for save/load**: Serialize Lua global state via the `slua` WAD tag (`WadTag::LuaState`) so that solo script state persists across save/load cycles. This is critical for scenarios like Istoria that maintain RPG inventory and quest state in Lua globals.
- **Wire dispatch into `SimWorld::tick()`**: Extend the tick system ordering to call into `marathon-lua` at the correct points -- after damage resolution for `player_damaged`/`monster_killed`, after projectile creation for `projectile_created`, after world mechanics for `platform_activated`/`light_activated`, and at tick end for `idle`.
- **Script source loading from plugins**: Extend the scenario loading path to collect Lua script sources from `PluginMetadata` fields (`solo_lua`, `hud_lua`, `stats_lua`) and from embedded `WadTag::LuaScript` data, respecting plugin load order and the `SoloLuaWriteAccess` exclusivity rules already implemented in `resolve_exclusive_resources()`.

## Capabilities

### New Capabilities
- `lua-vm`: Lua 5.5 virtual machine lifecycle -- creating, configuring, loading scripts, executing functions, and destroying Lua states. Pure Rust implementation via `lua-rs` targeting both native and `wasm32-unknown-unknown`.
- `lua-game-objects`: UserData type definitions for all Aleph One Lua-accessible game objects (Player, Monster, Projectile, Polygon, Line, Side, Platform, Light, Media, Level, Game, etc.) with field get/set accessors mapped to ECS components.
- `lua-event-dispatch`: Event dispatch system that fires Lua callbacks from simulation events -- wired into `SimWorld::tick()` at the correct system phases.
- `lua-hud-api`: HUD script execution context with the Screen drawing API, called once per frame from the render loop.
- `lua-stats-api`: Stats script execution context with end-of-game data collection callbacks.
- `lua-state-persistence`: Serialization and deserialization of Lua global state for save/load via the `slua` WAD tag.

### Modified Capabilities
- `game-loop`: `SimWorld::tick()` system ordering extended with Lua event dispatch callouts between existing system phases. `SimWorld::new()` accepts optional Lua script sources and initializes the Lua VM.
- `game-shell`: Level loading path collects and provides Lua script sources from WAD tags and plugin metadata to the sim layer. Level unload destroys Lua states.
- `hud-rendering`: HUD render pass calls into the Lua HUD draw context each frame when a HUD script is loaded, and renders the drawing commands produced by the script.

## Impact

- **New crate**: `marathon-lua` added to workspace `Cargo.toml` members, depends on `lua-rs`, `marathon-sim`, `marathon-formats`
- **marathon-sim/src/tick.rs**: `SimWorld::tick()` gains Lua dispatch callout points; `SimWorld::new()` accepts script configuration
- **marathon-sim/src/world.rs**: `SimWorld` holds an optional `LuaScriptEngine` resource; `SimSnapshot` extended with Lua state bytes; `SimEvents` extended with Lua-relevant event variants (or Lua hooks consume existing `SimEvent`s)
- **marathon-sim/src/components.rs**: No new components expected -- Lua reads/writes existing components via UserData accessors
- **marathon-formats/src/tags.rs**: Already defines `WadTag::LuaScript` and `WadTag::LuaState` -- no changes needed
- **marathon-formats/src/plugin.rs**: Already parses `solo_lua`, `hud_lua`, `stats_lua`, `SoloLuaWriteAccess` -- no changes needed
- **marathon-game/src/main.rs**: Level load path passes Lua sources to `SimWorld::new()`; HUD render loop calls `marathon-lua` HUD context
- **marathon-web/src/lib.rs**: Same wiring as marathon-game for the web build; confirms `lua-rs` compiles to `wasm32-unknown-unknown`
- **marathon-web/src/render.rs**: HUD overlay render pass extended to execute Lua HUD draw commands
- **Cargo.lock**: New dependency on `lua-rs` (pure Rust, no C dependencies, no build.rs complications for WASM)
- **Test strategy**: Unit tests for each UserData type (field access round-trips), integration tests executing real Aleph One Lua scripts from community scenarios against a `SimWorld`, WASM compilation smoke test
