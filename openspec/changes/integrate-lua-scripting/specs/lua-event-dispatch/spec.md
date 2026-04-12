## ADDED Requirements

### Requirement: Dispatch solo script events from simulation tick
The system SHALL fire Lua callback functions in the solo script VM at defined points within `SimWorld::tick()`. Events SHALL be dispatched between simulation system phases so that the ECS is in a consistent state when Lua code executes. The dispatch ordering within the tick SHALL be:

1. Player physics
2. **Lua dispatch: player movement events** (`start_refueling`, `end_refueling`)
3. Monster AI
4. Weapon/combat
5. Projectile physics
6. **Lua dispatch: projectile events** (`projectile_created`, `projectile_detonated`, `projectile_switch`)
7. Damage resolution
8. **Lua dispatch: damage/kill events** (`monster_damaged`, `monster_killed`, `player_damaged`, `player_killed`)
9. World mechanics (platforms, lights, media, items)
10. **Lua dispatch: world events** (`platform_activated`, `light_activated`, `tag_switch_activated`, `got_item`, `terminal_enter`, `terminal_exit`, `pattern_buffer`)
11. Cleanup
12. **Lua dispatch: `idle()`**

#### Scenario: Idle fires every tick
- **WHEN** `SimWorld::tick()` completes a full tick cycle with a solo script loaded
- **THEN** the system SHALL call the Lua function `idle()` at the end of the tick, after all simulation systems and other event dispatches have completed

#### Scenario: Monster killed event fires after damage resolution
- **WHEN** a monster's health reaches zero during damage resolution
- **THEN** the system SHALL call `monster_killed(monster, aggressor, projectile)` in the solo script VM, where `monster` is a Monster UserData, `aggressor` is a Player or Monster UserData (or nil), and `projectile` is a Projectile UserData (or nil)

#### Scenario: Player damaged event with arguments
- **WHEN** the player takes 25 points of damage of type 3 from a monster's projectile
- **THEN** the system SHALL call `player_damaged(player, aggressor, damage_type, damage_amount, projectile)` with appropriate UserData and numeric arguments

#### Scenario: Platform activated event
- **WHEN** a platform begins moving (transitions from AtRest to Extending)
- **THEN** the system SHALL call `platform_activated(polygon)` where `polygon` is a Polygon UserData for the platform's polygon

#### Scenario: No solo script loaded
- **WHEN** `SimWorld::tick()` runs with no solo script loaded in the LuaScriptEngine
- **THEN** all event dispatch points SHALL be no-ops with negligible overhead

### Requirement: Dispatch init and cleanup callbacks
The system SHALL call `init()` in the solo script VM after the script source is loaded and all APIs are registered, but before the first simulation tick. The system SHALL call `cleanup()` before destroying the solo VM on level unload.

#### Scenario: Init called on script load
- **WHEN** a solo script is loaded that defines a global function `init`
- **THEN** `init()` SHALL be called exactly once, before any `idle()` or event callbacks

#### Scenario: Cleanup called on level unload
- **WHEN** the level is unloaded and the solo script defines a global function `cleanup`
- **THEN** `cleanup()` SHALL be called exactly once before the Lua VM is destroyed

#### Scenario: Init not defined
- **WHEN** a solo script does not define an `init` function
- **THEN** the system SHALL silently skip the init dispatch with no error

### Requirement: Dispatch item pickup events
The system SHALL call `got_item(type, player)` in the solo script VM when a player picks up an item entity. The `type` argument SHALL be the item type index. The `player` argument SHALL be the Player UserData.

#### Scenario: Player picks up item
- **WHEN** the player collides with an item entity and picks it up
- **THEN** the system SHALL call `got_item(item_type, player)` before the item entity is despawned

### Requirement: Dispatch terminal events
The system SHALL call `terminal_enter(terminal, player)` when a player activates a terminal, and `terminal_exit(terminal, player)` when the player exits the terminal view. The `terminal` argument SHALL be a Terminal UserData. The `player` argument SHALL be a Player UserData.

#### Scenario: Terminal enter
- **WHEN** the player activates a terminal
- **THEN** the system SHALL call `terminal_enter(terminal, player)` with the terminal's UserData

#### Scenario: Terminal exit
- **WHEN** the player exits a terminal
- **THEN** the system SHALL call `terminal_exit(terminal, player)` with the terminal's UserData

### Requirement: Dispatch projectile creation events
The system SHALL call `projectile_created(projectile)` in the solo script VM when a new projectile entity is spawned during the combat/weapon phase. The `projectile` argument SHALL be a Projectile UserData.

#### Scenario: Weapon fires projectile
- **WHEN** the player fires a weapon that creates a projectile entity
- **THEN** the system SHALL call `projectile_created(projectile)` with a Projectile UserData for the new entity

### Requirement: Dispatch projectile detonation events
The system SHALL call `projectile_detonated(type, owner, polygon, position)` when a projectile is destroyed (hits a wall, floor, ceiling, or entity). The `type` argument SHALL be the projectile type index. The `owner` argument SHALL be a Player or Monster UserData (or nil). The `polygon` argument SHALL be a Polygon UserData for the detonation location. The `position` SHALL be a table with x, y, z fields.

#### Scenario: Projectile hits wall
- **WHEN** a projectile collides with a wall and is destroyed
- **THEN** the system SHALL call `projectile_detonated(type, owner, polygon, {x=..., y=..., z=...})`

### Requirement: Dispatch light and switch events
The system SHALL call `light_activated(light)` when a light's active state changes (toggled by switch or script). The system SHALL call `tag_switch_activated(tag)` when a tag switch is triggered by the player.

#### Scenario: Light switch toggled
- **WHEN** the player activates a light switch
- **THEN** the system SHALL call `light_activated(light)` with a Light UserData

#### Scenario: Tag switch activated
- **WHEN** the player activates a tag switch with tag index 5
- **THEN** the system SHALL call `tag_switch_activated(5)`

### Requirement: Collect simulation events for Lua dispatch
The system SHALL extend `SimEvents` (or use a parallel event collection mechanism) to collect fine-grained events during each simulation system phase that are needed for Lua dispatch. Events SHALL include: entity damage with source attribution, entity death with source attribution, projectile creation, projectile detonation with location, platform state changes, light state changes, switch activations, item pickups, terminal interactions. Each event SHALL carry enough information to construct the Lua callback arguments.

#### Scenario: Damage event carries source info
- **WHEN** monster A damages monster B for 10 points of type 2
- **THEN** the collected event SHALL record: victim entity, aggressor entity, damage type, damage amount, and optionally the projectile entity

#### Scenario: Multiple events in one tick
- **WHEN** three monsters are killed in a single tick
- **THEN** the system SHALL collect three separate `EntityKilled` events, and the Lua dispatch SHALL call `monster_killed` three times in sequence
