## MODIFIED Requirements

### Requirement: Level loading and initialization with Lua script collection
The system SHALL load a level by: (1) parsing the map entry from the WadFile via marathon-formats, (2) collecting Lua script sources from `WadTag::LuaScript` data in the map WAD and from plugin `solo_lua`, `hud_lua`, `stats_lua` fields, (3) initializing marathon-sim with the map data, physics data, game mode, and collected Lua script sources, (4) initializing the rendering pipeline with the map geometry, textures, and entity sprite data, (5) optionally initializing marathon-audio with the map data and sound definitions, (6) transitioning to the `Playing` state. Plugin script collection SHALL respect `resolve_exclusive_resources()` exclusivity rules. All parsing and initialization SHALL complete before gameplay begins.

#### Scenario: Load level with embedded Lua script
- **WHEN** the player starts a level whose WAD entry contains a `WadTag::LuaScript` chunk
- **THEN** the system SHALL extract the Lua source bytes, include them as the solo script source in `LuaScriptSources`, and pass them to `SimWorld::new()`

#### Scenario: Load level with plugin scripts
- **WHEN** the player starts a level with two plugins: one providing `hud_lua` and one providing `solo_lua`
- **THEN** the system SHALL collect both plugin script sources (reading `.lua` files from plugin directories), apply `resolve_exclusive_resources()` to handle conflicts, and include the resolved scripts in `LuaScriptSources`

#### Scenario: Plugin solo_lua exclusivity
- **WHEN** two plugins both declare `solo_lua` with `SoloLuaWriteAccess::WORLD`
- **THEN** `resolve_exclusive_resources()` SHALL disable the earlier plugin's solo_lua, and only the last plugin's solo script SHALL be loaded

#### Scenario: Load level without scripts
- **WHEN** the player starts a level with no embedded Lua script and no plugins with Lua scripts
- **THEN** `LuaScriptSources` SHALL have all fields as `None`, and the `LuaScriptEngine` SHALL be a no-op

#### Scenario: Level load failure does not affect Lua
- **WHEN** a level's map data fails to parse
- **THEN** the system SHALL display an error and transition to `MainMenu` without attempting Lua initialization

### Requirement: Level unload destroys Lua states
The system SHALL destroy all Lua VM instances when a level is unloaded (transitioning from `Playing` to `Loading`, `Intermission`, or `MainMenu`). Before destruction, the system SHALL call `cleanup()` in the solo script VM if a solo script was active. The `LuaScriptEngine` SHALL release all memory held by Lua states.

#### Scenario: Level transition destroys Lua
- **WHEN** the player completes a level and transitions to `Intermission`
- **THEN** the system SHALL call `cleanup()` in the solo VM, then destroy all Lua VMs before loading the next level

#### Scenario: Quit to menu destroys Lua
- **WHEN** the player quits to the main menu from the pause screen
- **THEN** the system SHALL destroy all Lua VMs as part of level unload

### Requirement: Save game includes Lua state
The system SHALL include serialized Lua state in save game files when a solo script is active. When serializing the game state for a save file, the system SHALL call `LuaScriptEngine::serialize_state()` to obtain the Lua global state bytes and include them in the save data. When loading a save file, the system SHALL pass the stored Lua state bytes to the level initialization so they can be restored after VM creation.

#### Scenario: Save with active solo script
- **WHEN** the player saves the game with an active solo script that has modified globals
- **THEN** the save file SHALL include the serialized Lua state alongside the simulation snapshot

#### Scenario: Load save restores Lua state
- **WHEN** the player loads a save file that includes Lua state
- **THEN** the system SHALL reconstruct the `LuaScriptSources`, create VMs, execute script sources, and then overlay the deserialized Lua globals onto the solo VM

### Requirement: Collect Lua script sources from WAD and plugins
The system SHALL implement a `LuaScriptSources` struct that holds optional script source code for each of the three script types: `solo: Option<String>`, `hud: Option<String>`, `stats: Option<String>`. The collection process SHALL: (1) check for `WadTag::LuaScript` data in the map WAD entry and use it as the embedded solo script source, (2) iterate over active plugins in load order, collecting `solo_lua`, `hud_lua`, `stats_lua` file paths, (3) read `.lua` files from plugin directories, (4) if both embedded and plugin solo scripts exist, the plugin script takes precedence per Aleph One behavior. Multiple solo scripts from different plugins are resolved by `resolve_exclusive_resources()`.

#### Scenario: Embedded script only
- **WHEN** the WAD contains `WadTag::LuaScript` data and no plugins have Lua scripts
- **THEN** `LuaScriptSources.solo` SHALL contain the embedded script source

#### Scenario: Plugin overrides embedded
- **WHEN** the WAD contains `WadTag::LuaScript` and a plugin also provides `solo_lua`
- **THEN** `LuaScriptSources.solo` SHALL contain the plugin's solo script (plugin takes precedence)

#### Scenario: Multiple script types from plugins
- **WHEN** one plugin provides `hud_lua` and another provides `stats_lua`
- **THEN** `LuaScriptSources` SHALL have `hud` from the first plugin and `stats` from the second plugin
