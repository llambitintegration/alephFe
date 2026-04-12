## ADDED Requirements

### Requirement: Create Lua VM instances from script sources
The system SHALL create Lua 5.5 virtual machine instances using `lua-rs` (pure Rust implementation). The system SHALL support creating up to three independent VM instances per level: one for solo scripts, one for HUD scripts, and one for stats scripts. Each VM SHALL be created only if the corresponding script type has source code available. The `lua-rs` crate SHALL compile to both native targets and `wasm32-unknown-unknown` without any C toolchain dependency.

#### Scenario: Create solo VM from embedded script
- **WHEN** `LuaScriptEngine::new()` is called with `LuaScriptSources` containing solo script source code extracted from `WadTag::LuaScript`
- **THEN** the system SHALL create a Lua VM instance, load the standard library, register solo-context APIs (all UserData types with read/write access), and execute the script source to define callback functions

#### Scenario: Create HUD VM from plugin script
- **WHEN** `LuaScriptEngine::new()` is called with `LuaScriptSources` containing HUD script source code from a plugin's `hud_lua` field
- **THEN** the system SHALL create a Lua VM instance, load the standard library, register HUD-context APIs (read-only UserData types plus the Screen drawing API), and execute the script source

#### Scenario: Create stats VM from plugin script
- **WHEN** `LuaScriptEngine::new()` is called with `LuaScriptSources` containing stats script source code from a plugin's `stats_lua` field
- **THEN** the system SHALL create a Lua VM instance, load the standard library, register stats-context APIs (read-only game state plus stats accumulation functions), and execute the script source

#### Scenario: No scripts available
- **WHEN** `LuaScriptEngine::new()` is called with `LuaScriptSources` where all three script types are `None`
- **THEN** the system SHALL create a `LuaScriptEngine` with no active VMs, and all dispatch/draw calls SHALL be no-ops

#### Scenario: Script parse error
- **WHEN** script source code contains a Lua syntax error
- **THEN** the system SHALL log the error with the script type and error details, and SHALL NOT create a VM for that script type, but SHALL continue creating VMs for other script types that parse successfully

#### Scenario: WASM compilation
- **WHEN** the `marathon-lua` crate is compiled with target `wasm32-unknown-unknown`
- **THEN** the build SHALL succeed with no C toolchain required, and the resulting WASM module SHALL be able to create and execute Lua VMs

### Requirement: Configure VM with context-appropriate APIs
The system SHALL register different sets of APIs in each VM based on the script context. Solo VMs SHALL have full read/write access to all game object UserData types plus the event callback mechanism. HUD VMs SHALL have read-only access to game objects plus the Screen drawing API. Stats VMs SHALL have read-only access to game state plus stats accumulation callbacks.

#### Scenario: Solo VM has write access
- **WHEN** a solo script sets `monster.vitality = 0`
- **THEN** the write SHALL succeed and the monster's Health component in the ECS SHALL be updated to 0

#### Scenario: HUD VM has read-only game objects
- **WHEN** a HUD script reads `Players[0].health`
- **THEN** the read SHALL return the player's current health value

#### Scenario: HUD VM cannot write game state
- **WHEN** a HUD script attempts to set `Players[0].health = 999`
- **THEN** the write SHALL be rejected with a Lua error indicating that HUD scripts cannot modify game state

### Requirement: Load Lua standard library
The system SHALL load the Lua 5.5 standard library into each VM, providing `string`, `table`, `math`, `io` (limited), `os` (limited), `coroutine`, `utf8`, and `debug` libraries. The `io` and `os` libraries SHALL be sandboxed: `io.open` and `os.execute` SHALL be disabled to prevent filesystem/process access from scripts.

#### Scenario: String library available
- **WHEN** a Lua script calls `string.format("HP: %d/%d", 100, 150)`
- **THEN** the call SHALL succeed and return `"HP: 100/150"`

#### Scenario: Math library available
- **WHEN** a Lua script calls `math.sqrt(144)`
- **THEN** the call SHALL return `12.0`

#### Scenario: File I/O blocked
- **WHEN** a Lua script calls `io.open("/etc/passwd", "r")`
- **THEN** the call SHALL fail with an error indicating that file I/O is disabled

### Requirement: Destroy VM instances on level unload
The system SHALL destroy all active Lua VM instances when a level is unloaded. After destruction, the `LuaScriptEngine` SHALL release all memory held by the Lua states. Subsequent dispatch calls on the engine SHALL be no-ops.

#### Scenario: Level unload destroys VMs
- **WHEN** the shell transitions from `Playing` to `Loading` for a new level
- **THEN** the system SHALL call `LuaScriptEngine::destroy()`, dropping all Lua VM instances and releasing their memory

#### Scenario: Dispatch after destroy is no-op
- **WHEN** `engine.dispatch_idle()` is called after `engine.destroy()`
- **THEN** the call SHALL return successfully with no side effects

### Requirement: Execute named Lua functions with arguments
The system SHALL support calling named global Lua functions with typed arguments. Arguments SHALL be marshalled from Rust types to Lua values: `i32`/`f64` to Lua numbers, `&str` to Lua strings, UserData handles to Lua userdata, and `Option<T>` to value-or-nil. Return values from Lua functions SHALL be discarded (callbacks are fire-and-forget).

#### Scenario: Call function with integer argument
- **WHEN** the system calls `idle()` in the solo VM (no arguments)
- **THEN** the Lua function named `idle` SHALL execute if defined, or the call SHALL silently succeed if `idle` is not defined

#### Scenario: Call function with UserData argument
- **WHEN** the system calls `monster_killed(monster, aggressor, projectile)` with a Monster UserData, a Player UserData, and a Projectile UserData
- **THEN** the Lua function SHALL receive these as userdata arguments with accessible fields

#### Scenario: Undefined callback is a no-op
- **WHEN** the system calls a callback function name that is not defined in the script
- **THEN** the call SHALL return successfully with no error

#### Scenario: Callback runtime error
- **WHEN** a Lua callback function raises a runtime error (e.g., accessing nil field)
- **THEN** the system SHALL log the error with script type, function name, and error message, and SHALL NOT crash or halt the simulation
