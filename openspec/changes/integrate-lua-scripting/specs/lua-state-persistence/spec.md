## ADDED Requirements

### Requirement: Serialize solo Lua state for save
The system SHALL serialize the solo script VM's global state when saving the game. Serialization SHALL traverse all global variables and serialize primitive values (numbers, strings, booleans, nil) and tables (including nested tables). Functions, coroutines, userdata, and threads SHALL be skipped during serialization (replaced with nil on deserialization). The serialized bytes SHALL be included in the `SimSnapshot` alongside ECS state.

#### Scenario: Serialize primitive globals
- **WHEN** the solo script has set `quest_stage = 3` and `player_name = "Bob"` and a game save is triggered
- **THEN** the serialized Lua state SHALL include both variables, and after deserialization both SHALL have their original values

#### Scenario: Serialize nested tables
- **WHEN** the solo script has set `inventory = {keys = {red = true, blue = false}, gold = 42}`
- **THEN** the serialized Lua state SHALL preserve the full table structure including nesting

#### Scenario: Functions skipped in serialization
- **WHEN** the solo script has set `my_callback = function() end`
- **THEN** the serialized state SHALL skip the function, and after deserialization `my_callback` SHALL be nil

#### Scenario: Circular table references
- **WHEN** the solo script creates tables with circular references (`a = {}; b = {a}; a[1] = b`)
- **THEN** the serializer SHALL handle circular references without infinite loops (either by detecting cycles or by limiting depth)

### Requirement: Deserialize Lua state on load
The system SHALL restore the solo script VM's global state when loading a saved game. After restoring ECS state from `SimSnapshot`, the system SHALL create a new solo VM, load the script source, execute it (to re-define functions), and then overlay the deserialized global variables on top of the VM's state. This ensures both function definitions (from script source) and persistent data (from save) are present.

#### Scenario: Load saved game restores Lua state
- **WHEN** a saved game is loaded that includes serialized Lua state with `quest_stage = 3`
- **THEN** after level load and VM initialization, the solo script's `quest_stage` global SHALL be `3`

#### Scenario: Functions re-defined from source
- **WHEN** a saved game is loaded and the solo script source defines `function idle() ... end`
- **THEN** the `idle` function SHALL be available (from executing the script source), even though functions were not serialized

#### Scenario: No Lua state in save
- **WHEN** a saved game is loaded that has no serialized Lua state (saved before Lua was implemented, or no solo script was active)
- **THEN** the system SHALL initialize the solo VM normally without applying any deserialized state

### Requirement: Store serialized Lua state in SimSnapshot
The `SimSnapshot` struct SHALL be extended with an optional `lua_state: Option<Vec<u8>>` field containing the serialized Lua global state bytes. The field SHALL be `None` when no solo script is active. The serialized bytes SHALL use a compact binary format (not Lua source code).

#### Scenario: Snapshot includes Lua state
- **WHEN** `SimWorld::snapshot()` is called with an active solo script
- **THEN** the resulting `SimSnapshot.lua_state` SHALL be `Some(bytes)` containing the serialized global state

#### Scenario: Snapshot without Lua
- **WHEN** `SimWorld::snapshot()` is called with no active solo script
- **THEN** the resulting `SimSnapshot.lua_state` SHALL be `None`

### Requirement: WAD tag compatibility for Lua state
The serialized Lua state bytes SHALL be compatible with storage in the `WadTag::LuaState` (tag `slua`) format for save files. The system SHALL be able to write Lua state bytes to a WAD save file using the existing `slua` tag, and read them back on load.

#### Scenario: Round-trip through WAD save
- **WHEN** a game is saved with Lua state, written to a WAD file with the `slua` tag, then loaded back
- **THEN** the deserialized Lua state SHALL match the original state (for all serializable types)

### Requirement: State serialization performance
The system SHALL serialize Lua state within the save operation's time budget. For typical scenario scripts (Istoria-scale with ~100 global variables including nested tables), serialization SHALL complete in under 10ms. The serialized size SHALL be proportional to the number of global variables and their content.

#### Scenario: Moderate state serialization time
- **WHEN** the solo script has 100 global variables including 20 nested tables
- **THEN** serialization SHALL complete in under 10ms on a modern CPU
