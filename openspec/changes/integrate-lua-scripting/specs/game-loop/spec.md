## MODIFIED Requirements

### Requirement: Construct simulation world from map and physics data
The system SHALL construct a `SimWorld` from a `MapData`, `PhysicsData`, and optional `LuaScriptSources` loaded via marathon-formats. Construction SHALL spawn ECS entities for all map objects (monsters, items, players) at their initial positions, initialize platform state from `StaticPlatformData`, initialize light state from `StaticLightData`, and initialize media state from `MediaData`. The deterministic PRNG SHALL be seeded from a provided seed value. When `LuaScriptSources` is provided, the system SHALL initialize a `LuaScriptEngine` as a resource accessible during the tick. After VM initialization and script loading, the system SHALL call `init()` in the solo script VM.

#### Scenario: Construct world with Lua scripts
- **WHEN** `SimWorld::new()` is called with valid `MapData`, `PhysicsData`, and `LuaScriptSources` containing a solo script
- **THEN** the world SHALL contain all map entities, a `LuaScriptEngine` resource with an active solo VM, and `init()` SHALL have been called in the solo script

#### Scenario: Construct world without Lua scripts
- **WHEN** `SimWorld::new()` is called with `LuaScriptSources` where all script types are `None`
- **THEN** the world SHALL be constructed identically to the current behavior, with a no-op `LuaScriptEngine`

### Requirement: Advance simulation by one tick with Lua dispatch
The system SHALL advance the simulation state by exactly one tick (1/30th of a second) when `tick()` is called with the current frame's `ActionFlags`. All systems SHALL execute in the defined order with Lua event dispatch callouts inserted between phases: input processing, player physics, **[Lua: player events]**, monster AI, weapon/combat, projectile physics, **[Lua: projectile events]**, damage resolution, **[Lua: damage/kill events]**, world mechanics, **[Lua: world events]**, cleanup, **[Lua: idle]**. When no Lua script is loaded, the dispatch callouts SHALL be no-ops.

#### Scenario: Tick with Lua dispatch
- **WHEN** `tick()` is called with a solo script loaded and a monster is killed during damage resolution
- **THEN** the system SHALL call `monster_killed()` in the Lua VM after damage resolution but before world mechanics, and SHALL call `idle()` at the end of the tick

#### Scenario: Tick without Lua
- **WHEN** `tick()` is called with no Lua scripts loaded
- **THEN** the simulation SHALL advance identically to the current behavior with no Lua overhead

### Requirement: Serialize and deserialize simulation state with Lua
The system SHALL support serializing the complete simulation state including Lua global state to bytes via serde, and deserializing back to a functional `SimWorld`. `SimSnapshot` SHALL include an optional `lua_state: Option<Vec<u8>>` field. On serialization, if a solo script is active, the Lua VM's global state SHALL be serialized into this field. On deserialization, after rebuilding the ECS and reloading the solo script source, the deserialized Lua globals SHALL be restored into the new VM.

#### Scenario: Round-trip serialization with Lua state
- **WHEN** a `SimWorld` with an active solo script (where `quest_stage = 3`) is serialized after 100 ticks, then deserialized
- **THEN** the deserialized world SHALL have `quest_stage = 3` in the solo VM's globals, and all ECS state SHALL match

#### Scenario: Round-trip serialization without Lua
- **WHEN** a `SimWorld` with no Lua scripts is serialized and deserialized
- **THEN** the result SHALL be identical to the current behavior, with `lua_state` being `None` in the snapshot
