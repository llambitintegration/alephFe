## 1. Crate Setup and lua-rs Integration

- [ ] 1.1 Create `marathon-lua` crate directory with `Cargo.toml`, add to workspace members in root `Cargo.toml`
- [ ] 1.2 Add dependencies: `lua-rs` (git or crates.io), `marathon-sim`, `marathon-formats`, `log`, `thiserror`, `serde`
- [ ] 1.3 Create `src/lib.rs` with public API stubs: `LuaScriptEngine`, `LuaScriptSources`, `HudDrawCommand`
- [ ] 1.4 Verify `lua-rs` compiles and can create/execute a trivial Lua script in a unit test
- [ ] 1.5 Verify `marathon-lua` compiles for `wasm32-unknown-unknown` target (Docker cross-compile smoke test)
- [ ] 1.6 Add `marathon-lua` dependency to `marathon-game/Cargo.toml` and `marathon-web/Cargo.toml`

## 2. VM Lifecycle

- [ ] 2.1 Implement `vm.rs`: `create_lua_vm()` that creates a `lua_rs::Lua` instance and loads the standard library
- [ ] 2.2 Implement sandboxing: disable `io.open`, `os.execute`, and other dangerous standard library functions
- [ ] 2.3 Implement `context.rs`: `ScriptContext` enum (Solo, Hud, Stats) controlling which APIs are registered per VM
- [ ] 2.4 Implement `LuaScriptEngine::new(sources: &LuaScriptSources)` that creates up to three VMs based on available sources
- [ ] 2.5 Implement script loading: execute source code string in the VM, handle and log parse/runtime errors
- [ ] 2.6 Implement `LuaScriptEngine::destroy()` that drops all VM instances and releases memory
- [ ] 2.7 Implement `call_function(vm, name, args)` helper that calls a named Lua global function, logging errors and silently returning if the function is not defined
- [ ] 2.8 Unit test: create VM, load script defining `function idle() return 42 end`, verify function exists
- [ ] 2.9 Unit test: script with syntax error logs error but does not panic
- [ ] 2.10 Unit test: call undefined function is a silent no-op

## 3. Game Object UserData Types

- [ ] 3.1 Implement `objects/player.rs`: Player UserData with entity handle, field getters (health, shield, oxygen, position, facing, polygon, yaw, pitch), field setters (health, shield, oxygen, position, yaw, pitch, teleport_to_polygon)
- [ ] 3.2 Implement `objects/monster.rs`: Monster UserData with entity handle, field getters (type, vitality, position, polygon, facing, action, mode), field setters (vitality, position, facing, action, mode), methods (kill, damage, accelerate, play_sound)
- [ ] 3.3 Implement `objects/projectile.rs`: Projectile UserData with entity handle, field getters (type, position, polygon, facing, owner, damage_type), field setters (position, facing)
- [ ] 3.4 Implement `objects/polygon.rs`: Polygon UserData with index, field getters (floor_height, ceiling_height, type, media, permutation, vertex_count, adjacent_polygon_count, platform, visible_on_automap), field setters (floor_height, ceiling_height, type, media, permutation), sub-collection accessors (adjacent_polygons, vertices, lines, sides)
- [ ] 3.5 Implement `objects/line.rs`: Line UserData with index, field getters (endpoints, length, solid, transparent, cw_polygon, ccw_polygon), field setters (solid, transparent)
- [ ] 3.6 Implement `objects/side.rs`: Side UserData with index, field getters (type, textures, lightsources, transfer_modes, polygon, line), field setters (textures, lightsources, transfer_modes)
- [ ] 3.7 Implement `objects/platform.rs`: Platform UserData with entity handle, field getters (polygon, floor_height, ceiling_height, speed, is_active, is_extending, is_contracting), field setters (speed, floor_height, ceiling_height), methods (activate, deactivate)
- [ ] 3.8 Implement `objects/light.rs`: Light UserData with entity handle, field getters (index, active, intensity, phase), field setters (active, intensity)
- [ ] 3.9 Implement `objects/media.rs`: Media UserData with entity handle, field getters (type, height, light, current_direction, current_magnitude), field setters (height)
- [ ] 3.10 Implement `objects/level.rs`: Level singleton UserData, field getters (name, index, difficulty, game_type, environment_code, fog fields), field setters for fog fields (gated by SoloLuaWriteAccess::FOG)
- [ ] 3.11 Implement `objects/game.rs`: Game singleton UserData, field getters (ticks, version, difficulty, type, scoring_mode, kill_limit, time_remaining)
- [ ] 3.12 Implement `objects/terminal.rs`: Terminal UserData with index, field getters (index, text)
- [ ] 3.13 Implement `objects/item.rs`: Item UserData with entity handle, field getters (type, position, polygon), field setters (position)
- [ ] 3.14 Implement `objects/effect.rs`: Effect UserData with entity handle, field getters (type, position, polygon), field setters (position)
- [ ] 3.15 Implement `objects/collections.rs`: Global collection accessors (Players, Monsters, Projectiles, Polygons, Lines, Sides, Platforms, Lights, Media, Items, Effects) with length operator, index access, and iterator protocol
- [ ] 3.16 Implement unit conversion helpers: Marathon world units (1024 = 1 WU) for positions, 512-unit angle circle for yaw/facing, and inverse conversions for setters
- [ ] 3.17 Unit test: Player field read round-trip (set Health component, read player.health in Lua, verify match)
- [ ] 3.18 Unit test: Player field write round-trip (set player.health in Lua, read Health component, verify match)
- [ ] 3.19 Unit test: Monster field accessors (vitality, type, action, position)
- [ ] 3.20 Unit test: Polygon field accessors (floor_height, ceiling_height read/write)
- [ ] 3.21 Unit test: Collection accessor length and index access (Players[0], Monsters[i], #Polygons)
- [ ] 3.22 Unit test: Collection iterator protocol (for m in Monsters() do ... end counts correctly)
- [ ] 3.23 Unit test: Unit conversion correctness (position and angle round-trips)
- [ ] 3.24 Unit test: HUD VM rejects writes to game objects (player.health = X raises error)

## 4. Event Dispatch

- [ ] 4.1 Implement `dispatch.rs`: `LuaEventDispatcher` that takes event data and calls named Lua functions with marshalled arguments
- [ ] 4.2 Implement event collection: extend `SimEvents` with fine-grained event variants needed for Lua dispatch (MonsterDamaged, ProjectileCreated, ProjectileDetonated, PlatformStateChanged, LightStateChanged, SwitchActivated, ItemPickedUp, TerminalInteraction)
- [ ] 4.3 Implement `dispatch_player_events()`: fires `start_refueling`, `end_refueling` after player physics
- [ ] 4.4 Implement `dispatch_projectile_events()`: fires `projectile_created`, `projectile_detonated`, `projectile_switch` after projectile physics
- [ ] 4.5 Implement `dispatch_damage_events()`: fires `monster_damaged`, `monster_killed`, `player_damaged`, `player_killed` after damage resolution
- [ ] 4.6 Implement `dispatch_world_events()`: fires `platform_activated`, `light_activated`, `tag_switch_activated`, `got_item`, `terminal_enter`, `terminal_exit`, `pattern_buffer` after world mechanics
- [ ] 4.7 Implement `dispatch_idle()`: fires `idle()` at end of every tick
- [ ] 4.8 Implement `dispatch_init()` and `dispatch_cleanup()`: fires `init()` after script load and `cleanup()` before VM destroy
- [ ] 4.9 Wire dispatch callouts into `SimWorld::tick()`: insert Lua dispatch calls between system phases at the correct ordering points
- [ ] 4.10 Modify `SimWorld::new()` to accept optional `LuaScriptSources` parameter and initialize `LuaScriptEngine`
- [ ] 4.11 Unit test: `idle()` callback fires every tick (counter in Lua increments)
- [ ] 4.12 Unit test: `init()` fires once on load, before first `idle()`
- [ ] 4.13 Unit test: `monster_killed` fires with correct arguments when monster health reaches zero
- [ ] 4.14 Unit test: dispatch is no-op when no solo script is loaded (no performance regression)

## 5. HUD API

- [ ] 5.1 Implement `hud.rs`: `HudDrawCommand` enum with variants FillRect, FrameRect, DrawText, DrawShape, SetClipRect, ClearClipRect
- [ ] 5.2 Implement Screen global UserData: register `fill_rect`, `frame_rect`, `draw_text`, `draw_shape`, `world_to_screen`, `clip_rect`, `unclip_rect`, `width`, `height` methods
- [ ] 5.3 Implement `Screen.colors` table with named color constants (white, black, red, green, blue, yellow, light_gray, dark_gray)
- [ ] 5.4 Implement `Screen.fonts` table with font constants (interface, computer, computer_large, title)
- [ ] 5.5 Implement `LuaScriptEngine::dispatch_hud_draw()` that calls `draw()` in the HUD VM and returns the draw command buffer
- [ ] 5.6 Implement `LuaScriptEngine::hud_draw_commands()` accessor that returns `Vec<HudDrawCommand>`
- [ ] 5.7 Wire HUD draw dispatch into `marathon-game` render loop: call `dispatch_hud_draw()` after 3D scene render, before frame present
- [ ] 5.8 Wire HUD draw dispatch into `marathon-web` render loop: same as marathon-game
- [ ] 5.9 Implement HUD draw command renderer in `marathon-game/src/render.rs`: translate FillRect, FrameRect, DrawText, DrawShape to wgpu draw calls in the HUD overlay pass
- [ ] 5.10 Implement HUD draw command renderer in `marathon-web/src/render.rs`: same translation for web build
- [ ] 5.11 Implement `world_to_screen` projection using the current view-projection matrix
- [ ] 5.12 Unit test: Screen.fill_rect produces correct HudDrawCommand::FillRect
- [ ] 5.13 Unit test: Screen.draw_text with default font and color
- [ ] 5.14 Unit test: Screen.width/height return correct dimensions
- [ ] 5.15 Unit test: draw command buffer clears between frames

## 6. Stats API

- [ ] 6.1 Implement `stats.rs`: stats context VM registration with read-only game object access and Screen drawing API
- [ ] 6.2 Implement stats callback dispatch: `got_kill(aggressor, victim, damage_type)`, `player_damaged(victim, aggressor, damage_type, amount)`, `game_ended()`
- [ ] 6.3 Implement stats `draw()` dispatch for the post-game screen (reuses HudDrawCommand buffer mechanism)
- [ ] 6.4 Wire stats callbacks into the shell state machine: dispatch kill/damage events during gameplay, dispatch `game_ended()` on level completion/death
- [ ] 6.5 Wire stats `draw()` into the Intermission/GameOver render path
- [ ] 6.6 Unit test: stats `got_kill` callback fires and script can accumulate data in globals
- [ ] 6.7 Unit test: stats `draw()` produces draw commands using Screen API

## 7. State Persistence

- [ ] 7.1 Implement `persistence.rs`: `serialize_lua_state(vm) -> Vec<u8>` that walks the solo VM's global table and serializes all primitive values, strings, and tables to a compact binary format
- [ ] 7.2 Implement circular reference detection in the serializer (limit depth or track visited tables)
- [ ] 7.3 Implement `deserialize_lua_state(vm, bytes)` that restores serialized globals into a fresh VM (after script source has been executed)
- [ ] 7.4 Extend `SimSnapshot` with `lua_state: Option<Vec<u8>>` field (update Serialize/Deserialize derives)
- [ ] 7.5 Wire serialization into `SimWorld::snapshot()`: if LuaScriptEngine has active solo VM, serialize state into snapshot
- [ ] 7.6 Wire deserialization into `SimWorld::deserialize()`: if snapshot has lua_state bytes, restore into new solo VM after script load
- [ ] 7.7 Ensure `WadTag::LuaState` compatibility: the serialized bytes can be written to/read from WAD save files using the `slua` tag
- [ ] 7.8 Unit test: serialize/deserialize round-trip for numbers, strings, booleans, nil
- [ ] 7.9 Unit test: serialize/deserialize round-trip for nested tables
- [ ] 7.10 Unit test: functions are skipped (nil after deserialization)
- [ ] 7.11 Unit test: circular table references do not cause infinite loop
- [ ] 7.12 Unit test: SimSnapshot with lua_state round-trips through bincode

## 8. Script Source Loading

- [ ] 8.1 Implement `LuaScriptSources` struct with `solo: Option<String>`, `hud: Option<String>`, `stats: Option<String>` fields
- [ ] 8.2 Implement script collection from WAD: extract `WadTag::LuaScript` bytes from map entry, decode as UTF-8 string
- [ ] 8.3 Implement script collection from plugins: iterate active plugins, read `.lua` files from plugin directories for `solo_lua`, `hud_lua`, `stats_lua` fields
- [ ] 8.4 Implement script source resolution: if both embedded and plugin solo scripts exist, plugin takes precedence; apply `resolve_exclusive_resources()` for multi-plugin conflicts
- [ ] 8.5 Wire script collection into level loading path in `marathon-game/src/main.rs`
- [ ] 8.6 Wire script collection into level loading path in `marathon-web/src/lib.rs` (plugin files pre-loaded as byte buffers)
- [ ] 8.7 Unit test: LuaScriptSources built from WAD with embedded script
- [ ] 8.8 Unit test: plugin script overrides embedded script
- [ ] 8.9 Unit test: resolve_exclusive_resources disables conflicting solo scripts

## 9. Integration Testing

- [ ] 9.1 Integration test: load a real Marathon 2 level with embedded Lua script, run 100 ticks, verify no panics
- [ ] 9.2 Integration test: HUD script producing draw commands that the renderer can consume
- [ ] 9.3 Integration test: save/load round-trip with active solo script preserving Lua globals
- [ ] 9.4 Integration test: plugin script loading from test fixture plugin directory
- [ ] 9.5 Docker CI test: verify `marathon-lua` compiles and tests pass in Docker build environment
- [ ] 9.6 WASM smoke test: verify `marathon-lua` + `marathon-web` compile for `wasm32-unknown-unknown`
