## ADDED Requirements

### Requirement: Player UserData type
The system SHALL provide a `Player` UserData type accessible in Lua scripts. The Player UserData SHALL store a `bevy_ecs::Entity` handle and resolve all field accesses through the ECS at access time. The following fields SHALL be readable: `index`, `name`, `team`, `polygon`, `x`, `y`, `z`, `yaw`, `pitch`, `health`, `shield`, `oxygen`, `dead`, `weapon`, `items`, `overlays`, `external_velocity`, `internal_velocity`, `elevation`, `direction`. The following fields SHALL be writable in solo scripts: `health`, `shield`, `oxygen`, `x`, `y`, `z`, `yaw`, `pitch`, `teleport_to_polygon`, `external_velocity`, `internal_velocity`, `elevation`, `direction`. The Player UserData SHALL also provide methods: `accelerate(direction, velocity)`, `position_to_polygon()`, `fade_screen(type, color, duration)`, `play_sound(sound, pitch)`, `find_action_key_target()`, `damage(amount, type)`.

#### Scenario: Read player health
- **WHEN** a Lua script accesses `player.health`
- **THEN** the system SHALL query the `Health` component on the player entity and return its value as a Lua number

#### Scenario: Write player health
- **WHEN** a solo Lua script sets `player.health = 100`
- **THEN** the system SHALL update the `Health` component on the player entity to 100

#### Scenario: Read player position
- **WHEN** a Lua script accesses `player.x`, `player.y`, `player.z`
- **THEN** the system SHALL return the x, y, z components of the `Position` component, converted to Marathon world units (multiply by 1024)

#### Scenario: Teleport player
- **WHEN** a solo script sets `player.teleport_to_polygon = 42`
- **THEN** the system SHALL move the player entity to the center of polygon 42, updating `Position` and `PolygonIndex` components

#### Scenario: Player items accessor
- **WHEN** a Lua script accesses `player.items`
- **THEN** the system SHALL return a table-like accessor where `player.items[item_type]` returns the count of that item type in the player's inventory

### Requirement: Monster UserData type
The system SHALL provide a `Monster` UserData type accessible in Lua scripts. The Monster UserData SHALL store a `bevy_ecs::Entity` handle. The following fields SHALL be readable: `type`, `index`, `polygon`, `x`, `y`, `z`, `yaw`, `vitality`, `action`, `mode`, `vertical_velocity`, `external_velocity`, `facing`, `visible`, `active`. The following fields SHALL be writable in solo scripts: `vitality`, `x`, `y`, `z`, `yaw`, `action`, `mode`, `external_velocity`, `facing`, `active`. The Monster UserData SHALL provide methods: `accelerate(direction, velocity)`, `position_to_polygon()`, `play_sound(sound)`, `damage(amount, type)`, `kill()`.

#### Scenario: Read monster vitality
- **WHEN** a Lua script accesses `monster.vitality`
- **THEN** the system SHALL query the `Health` component on the monster entity and return its value

#### Scenario: Write monster vitality
- **WHEN** a solo script sets `monster.vitality = 0`
- **THEN** the system SHALL set the monster's `Health` component to 0

#### Scenario: Kill monster
- **WHEN** a solo script calls `monster:kill()`
- **THEN** the system SHALL set the monster's `Health` to 0 and transition its `MonsterState` to `Dying`

#### Scenario: Read monster type
- **WHEN** a Lua script accesses `monster.type`
- **THEN** the system SHALL return the `Monster.definition_index` as a Lua number, which corresponds to the monster type in physics data

#### Scenario: Read monster action
- **WHEN** a Lua script accesses `monster.action`
- **THEN** the system SHALL return the `MonsterState` variant mapped to Aleph One's action constants (e.g., Idle=0, Alerted=1, Attacking=2)

### Requirement: Projectile UserData type
The system SHALL provide a `Projectile` UserData type accessible in Lua scripts. The Projectile UserData SHALL store a `bevy_ecs::Entity` handle. The following fields SHALL be readable: `type`, `index`, `polygon`, `x`, `y`, `z`, `yaw`, `pitch`, `owner`, `damage_type`. The following fields SHALL be writable in solo scripts: `x`, `y`, `z`, `yaw`, `pitch`.

#### Scenario: Read projectile type
- **WHEN** a Lua script accesses `projectile.type`
- **THEN** the system SHALL return the `Projectile.definition_index`

#### Scenario: Read projectile owner
- **WHEN** a Lua script accesses `projectile.owner`
- **THEN** the system SHALL return a Monster or Player UserData for the entity referenced by `ProjectileSource`, or nil if the source entity no longer exists

### Requirement: Polygon UserData type
The system SHALL provide a `Polygon` UserData type accessible in Lua scripts. The Polygon UserData SHALL store a polygon index. The following fields SHALL be readable: `index`, `floor_height`, `ceiling_height`, `type`, `media`, `permutation`, `contains_player_start`, `adjacent_polygon_count`, `vertex_count`, `visible_on_automap`, `platform`. Fields `floor_height`, `ceiling_height`, `type`, `media`, `permutation`, and `visible_on_automap` SHALL be writable in solo scripts. The Polygon UserData SHALL provide collection-style accessors: `polygon.adjacent_polygons[i]` returning adjacent Polygon UserData, `polygon.vertices[i]` returning vertex position, `polygon.lines[i]` returning Line UserData, `polygon.sides[i]` returning Side UserData.

#### Scenario: Read polygon floor height
- **WHEN** a Lua script accesses `polygon.floor_height`
- **THEN** the system SHALL return `MapGeometry.floor_heights[polygon_index]` converted to Marathon world units

#### Scenario: Write polygon floor height
- **WHEN** a solo script sets `polygon.floor_height = 0.5`
- **THEN** the system SHALL update `MapGeometry.floor_heights[polygon_index]` (converting from Marathon world units to internal f32)

#### Scenario: Read polygon adjacent count
- **WHEN** a Lua script accesses `#polygon.adjacent_polygons`
- **THEN** the system SHALL return the number of entries in `MapGeometry.polygon_adjacency[polygon_index]`

#### Scenario: Access polygon platform
- **WHEN** a Lua script accesses `polygon.platform`
- **THEN** the system SHALL return a Platform UserData if the polygon has an associated platform, or nil otherwise

#### Scenario: Iterate polygon vertices
- **WHEN** a Lua script iterates over `polygon.vertices`
- **THEN** each iteration SHALL yield the vertex position as x, y coordinates from `MapGeometry.polygon_vertices[polygon_index]`

### Requirement: Line UserData type
The system SHALL provide a `Line` UserData type accessible in Lua scripts. The Line UserData SHALL store a line index. The following fields SHALL be readable: `index`, `length`, `solid`, `transparent`, `has_transparent_side`, `cw_polygon`, `ccw_polygon`, `endpoints`. Fields `solid` and `transparent` SHALL be writable in solo scripts.

#### Scenario: Read line endpoints
- **WHEN** a Lua script accesses `line.endpoints`
- **THEN** the system SHALL return a table with two vertex positions from `MapGeometry.line_endpoints[line_index]`

#### Scenario: Read adjacent polygons
- **WHEN** a Lua script accesses `line.cw_polygon` and `line.ccw_polygon`
- **THEN** the system SHALL return Polygon UserData for the polygons on each side of the line, or nil for one-sided lines

### Requirement: Side UserData type
The system SHALL provide a `Side` UserData type accessible in Lua scripts. The Side UserData SHALL store a side index. The following fields SHALL be readable: `index`, `type`, `primary_texture`, `secondary_texture`, `transparent_texture`, `polygon`, `line`, `primary_lightsource`, `secondary_lightsource`, `transparent_lightsource`, `primary_transfer_mode`, `secondary_transfer_mode`, `transparent_transfer_mode`. Texture, lightsource, and transfer mode fields SHALL be writable in solo scripts.

#### Scenario: Read side texture
- **WHEN** a Lua script accesses `side.primary_texture`
- **THEN** the system SHALL return a table with `collection`, `texture_index`, and `shape_descriptor` fields from the map's side data

#### Scenario: Write side texture
- **WHEN** a solo script sets `side.primary_texture = {collection=1, texture_index=5}`
- **THEN** the system SHALL update the side's primary texture descriptor in the map data

### Requirement: Platform UserData type
The system SHALL provide a `Platform` UserData type accessible in Lua scripts. The Platform UserData SHALL store a `bevy_ecs::Entity` handle referencing a `Platform` component. The following fields SHALL be readable: `polygon`, `floor_height`, `ceiling_height`, `speed`, `is_active`, `is_extending`, `is_contracting`, `is_at_rest`, `is_at_extended`. Fields `speed`, `floor_height`, `ceiling_height` SHALL be writable. Methods: `activate()`, `deactivate()`, `set_floor_height(h)`, `set_ceiling_height(h)`.

#### Scenario: Read platform active state
- **WHEN** a Lua script accesses `platform.is_active`
- **THEN** the system SHALL return `true` if the platform's `PlatformState` is `Extending` or `Returning`, `false` if `AtRest` or `AtExtended`

#### Scenario: Activate platform
- **WHEN** a solo script calls `platform:activate()`
- **THEN** the system SHALL set the platform's `PlatformState` to `Extending`

### Requirement: Light UserData type
The system SHALL provide a `Light` UserData type accessible in Lua scripts. The Light UserData SHALL store a `bevy_ecs::Entity` handle referencing a `Light` component. The following fields SHALL be readable: `index`, `active`, `intensity`, `phase`. Fields `active` and `intensity` SHALL be writable in solo scripts.

#### Scenario: Read light intensity
- **WHEN** a Lua script accesses `light.intensity`
- **THEN** the system SHALL return the `Light.current_intensity` value

#### Scenario: Set light active
- **WHEN** a solo script sets `light.active = false`
- **THEN** the system SHALL set the light's intensity to its minimum value and halt its animation

### Requirement: Media UserData type
The system SHALL provide a `Media` UserData type accessible in Lua scripts. The Media UserData SHALL store a `bevy_ecs::Entity` handle referencing a `Media` component. The following fields SHALL be readable: `type`, `height`, `light`, `current_direction`, `current_magnitude`. The `height` field SHALL be writable in solo scripts.

#### Scenario: Read media height
- **WHEN** a Lua script accesses `media.height`
- **THEN** the system SHALL return `Media.current_height` converted to Marathon world units

#### Scenario: Set media height
- **WHEN** a solo script sets `media.height = 0.25`
- **THEN** the system SHALL update `Media.current_height`

### Requirement: Level UserData type (singleton)
The system SHALL provide a `Level` global UserData accessible in Lua scripts. The Level UserData SHALL access world-level resources. The following fields SHALL be readable: `name`, `index`, `map_checksum`, `difficulty`, `game_type`, `player_count`, `initial_random_seed`, `environment_code`, `fog_active`, `fog_color`, `fog_depth`, `underwater_fog_active`, `underwater_fog_color`, `underwater_fog_depth`, `map_file`. The fog fields SHALL be writable in solo scripts with `SoloLuaWriteAccess::FOG`.

#### Scenario: Read level name
- **WHEN** a Lua script accesses `Level.name`
- **THEN** the system SHALL return the current level's name string

#### Scenario: Write fog settings
- **WHEN** a solo script with FOG write access sets `Level.fog_active = true`
- **THEN** the system SHALL enable fog rendering with the specified parameters

### Requirement: Game UserData type (singleton)
The system SHALL provide a `Game` global UserData accessible in Lua scripts. The following fields SHALL be readable: `ticks`, `version`, `difficulty`, `type`, `scoring_mode`, `kill_limit`, `time_remaining`, `proper_item_accounting`, `nonlocal_overlays`, `monsters_replenish`. The `Game` object SHALL be read-only in all script contexts.

#### Scenario: Read game ticks
- **WHEN** a Lua script accesses `Game.ticks`
- **THEN** the system SHALL return the current `TickCounter.0` value

#### Scenario: Read game difficulty
- **WHEN** a Lua script accesses `Game.difficulty`
- **THEN** the system SHALL return the difficulty setting (0=Kindergarten through 4=Total Carnage)

### Requirement: Terminal UserData type
The system SHALL provide a `Terminal` UserData type accessible in Lua scripts. The Terminal UserData SHALL store a terminal index. The following fields SHALL be readable: `index`, `text`, `groups`.

#### Scenario: Read terminal index
- **WHEN** a Lua script accesses `terminal.index`
- **THEN** the system SHALL return the terminal's index in the map data

### Requirement: Item UserData type
The system SHALL provide an `Item` UserData type accessible in Lua scripts. The Item UserData SHALL store a `bevy_ecs::Entity` handle. The following fields SHALL be readable: `type`, `index`, `polygon`, `x`, `y`, `z`. Position fields SHALL be writable in solo scripts.

#### Scenario: Read item type
- **WHEN** a Lua script accesses `item.type`
- **THEN** the system SHALL return the `Item.item_type` value

### Requirement: Effect UserData type
The system SHALL provide an `Effect` UserData type accessible in Lua scripts. The Effect UserData SHALL store a `bevy_ecs::Entity` handle. The following fields SHALL be readable: `type`, `index`, `polygon`, `x`, `y`, `z`. Position fields SHALL be writable in solo scripts.

#### Scenario: Read effect position
- **WHEN** a Lua script accesses `effect.x`, `effect.y`, `effect.z`
- **THEN** the system SHALL return the `Position` component values of the effect entity

### Requirement: Collection accessor types
The system SHALL provide global collection accessors: `Players`, `Monsters`, `Projectiles`, `Polygons`, `Lines`, `Sides`, `Platforms`, `Lights`, `Media`, `Items`, `Effects`. Each collection SHALL support: length operator `#collection`, index access `collection[i]`, and iterator protocol `for item in collection() do ... end`. Indexing SHALL return the appropriate UserData type or nil for invalid indices. Collection length SHALL reflect the current number of entities/elements.

#### Scenario: Iterate all monsters
- **WHEN** a Lua script executes `for m in Monsters() do ... end`
- **THEN** the iterator SHALL yield a Monster UserData for each living monster entity in the ECS

#### Scenario: Collection length
- **WHEN** a Lua script accesses `#Polygons`
- **THEN** the system SHALL return the total number of polygons in the current level's `MapGeometry`

#### Scenario: Index out of bounds
- **WHEN** a Lua script accesses `Monsters[9999]` where only 5 monsters exist
- **THEN** the system SHALL return nil

#### Scenario: Player by index
- **WHEN** a Lua script accesses `Players[0]`
- **THEN** the system SHALL return the Player UserData for the local player (index 0 in single-player)

### Requirement: Unit conversion between Lua and ECS values
The system SHALL convert between Marathon's Lua API coordinate system and the internal ECS coordinate system. Marathon Lua uses world units where 1024 internal units = 1 world unit. Position values exposed to Lua SHALL be in Marathon world units (the `x`, `y`, `z` fields return `Position.0 * 1024.0`). Angle values SHALL be in Marathon's 512-unit circle (0-511) where the ECS uses radians. Height values (`floor_height`, `ceiling_height`) SHALL be in Marathon world units.

#### Scenario: Position coordinate conversion
- **WHEN** a Lua script reads `monster.x` and the monster's `Position.0.x` is `1.5` (internal f32)
- **THEN** the Lua value SHALL be `1536.0` (1.5 * 1024)

#### Scenario: Angle conversion
- **WHEN** a Lua script reads `player.yaw` and the player's `Facing.0` is `PI/2` radians
- **THEN** the Lua value SHALL be `128` (quarter turn in Marathon's 512 circle)

#### Scenario: Write position with conversion
- **WHEN** a solo script sets `monster.x = 2048`
- **THEN** the system SHALL set `Position.0.x` to `2.0` (2048 / 1024)
