## Context

The `marathon-sim` crate contains a complete platform state machine in `world_mechanics/platforms.rs` with correct Extending/AtExtended/Returning/AtRest transitions, activation trigger checks (`should_activate`), crush detection (`check_platform_crush`), and linked platform/light event emission (`check_platform_triggers`). However, none of this code is wired into the running simulation. `tick_platform()` is never called from `SimWorld::tick()`, so every platform sits permanently at rest.

Even if platforms did tick, the results would be invisible: `MapGeometry.floor_heights` and `ceiling_heights` are set once at level load from the map's static polygon heights and never updated. Player physics (`apply_player_collision`) and mesh generation both read from `MapGeometry`, so neither collision nor rendering would reflect platform movement.

There are also no activation paths. The player can stand on a platform polygon or press the action key and nothing happens because no system checks `should_activate()` or calls `activate_platform()`. The `Platform` component lacks a `platform_type` field, so all platforms are treated identically regardless of whether they are doors, elevators, or crushers. Linked platform/light indices are not stored per-platform.

The `spawn_platforms()` function in `world.rs` sets `floor_rest = minimum_height`, `floor_extended = maximum_height`, and hardcodes ceiling values to 0.0, which is incorrect for door-type platforms that move ceilings.

## Goals / Non-Goals

**Goals:**
- Platforms move: elevators rise, doors open, crushers descend, and teleporter platforms activate
- Platform positions reflected in collision and rendering via MapGeometry sync
- Player-entry, action-key, monster-entry, and projectile-impact activation triggers work
- Action key re-activation reverses a moving platform (Marathon behavior)
- Control panel activation of platforms via the existing `panels.rs` system
- Crush damage applied to entities caught between floor and ceiling
- Non-crushing platforms reverse direction when an entity is in the way
- Door-type platforms auto-return after their delay timer
- Linked platform cascading: reaching a destination activates linked platforms
- Linked light toggling: reaching a destination toggles linked lights
- Sound events emitted for platform start, stop, and looping movement
- Mesh rebuild notification so renderers know which polygons changed

**Non-Goals:**
- Light animation ticking (separate change: implement-light-state-machine)
- Media height simulation (separate change)
- Monster pathfinding through platforms (monster AI is not yet platform-aware)
- Multiplayer-specific platform behavior (e.g., per-player platform locking)

## Decisions

### 1. Six platform types as an enum on the Platform component

**Decision:** Add a `PlatformType` enum with six variants matching Marathon's platform types: `ExtendsFloorToCeiling` (0), `ExtendsCeilingToFloor` (1), `ExtendsFloorAndCeiling` (2), `FromFloor` (3), `FromCeiling` (4), `Teleporter` (5). Store this on the `Platform` component. The `spawn_platforms()` function will compute `floor_rest`, `floor_extended`, `ceiling_rest`, and `ceiling_extended` based on the type and the polygon's initial floor/ceiling from `MapGeometry`.

**Rationale:** Marathon's 6 platform types are the core differentiator for how height ranges are calculated. Without type-awareness, door platforms (types 0 and 1) cannot open because their ceiling movement is not configured.

**Height calculation per type:**
- `ExtendsFloorToCeiling` (type 0, door): Floor rises to ceiling. `floor_rest = polygon_floor`, `floor_extended = polygon_ceiling`, ceiling stays at `polygon_ceiling`.
- `ExtendsCeilingToFloor` (type 1, door): Ceiling descends to floor. `ceiling_rest = polygon_ceiling`, `ceiling_extended = polygon_floor`, floor stays at `polygon_floor`.
- `ExtendsFloorAndCeiling` (type 2): Both floor and ceiling move toward each other. `floor_extended = polygon_ceiling`, `ceiling_extended = polygon_floor` (or symmetric from center, depending on min/max).
- `FromFloor` (type 3, elevator): Floor moves between `minimum_height` and `maximum_height`. Ceiling stays at `polygon_ceiling`.
- `FromCeiling` (type 4, crusher): Ceiling moves between `minimum_height` and `maximum_height`. Floor stays at `polygon_floor`.
- `Teleporter` (type 5): No visible height movement. Activation triggers a level teleport event.

### 2. Platform links stored as fields on the Platform component

**Decision:** Add `linked_platforms: Vec<usize>` and `linked_lights: Vec<usize>` fields to the `Platform` component. These are populated during `spawn_platforms()` from the map's platform tag data and line/side trigger references.

**Rationale:** The existing `check_platform_triggers()` function already accepts these as parameters. Storing them on the component keeps the data close to the logic and avoids a separate lookup table. The vectors are small (usually 0-2 entries per platform) and allocated once at level load.

**Alternative considered:** A separate `PlatformLinks` component. Rejected because it adds ECS query complexity for minimal benefit -- platforms always need their links during the trigger check phase.

### 3. State machine in platforms.rs unchanged; orchestration in tick.rs

**Decision:** The existing `tick_platform()`, `activate_platform()`, `should_activate()`, `check_platform_crush()`, and `check_platform_triggers()` functions in `platforms.rs` remain as-is (pure functions operating on `&mut Platform`). A new `run_world_mechanics()` method on `SimWorld` in `tick.rs` orchestrates: iterate platforms, tick each, sync MapGeometry, check activations, process crush, dispatch linked events.

**Rationale:** The pure functions are already well-tested (9 unit tests). Keeping them pure makes them easy to test in isolation. The orchestration layer is the only thing that touches ECS queries and mutable world state.

### 4. Re-activation reverses moving platforms

**Decision:** Extend `activate_platform()` to handle re-activation: if a platform is `Extending`, switch to `Returning`; if `Returning`, switch to `Extending`. This matches Marathon's behavior where pressing the action key on a moving platform reverses it.

**Rationale:** This is observable Marathon behavior and important for gameplay (e.g., stopping a crusher, re-opening a closing door).

### 5. MapGeometry dirty flag for mesh rebuild notification

**Decision:** Add a `changed_polygons: Vec<bool>` field to `MapGeometry`, one entry per polygon, initially all `false`. When platform ticking updates a polygon's heights, set its entry to `true`. Renderers check and clear these flags each frame to know which mesh sections to rebuild. Also add a `has_changes: bool` convenience flag to skip the check entirely when no platforms moved.

**Rationale:** A dirty-flag approach avoids emitting per-polygon SimEvents (which would need allocation and iteration) and gives renderers fine-grained control over incremental mesh rebuilds. The `has_changes` flag makes the common case (no platforms moving) zero-cost.

**Alternative considered:** SimEvents per polygon change. Rejected because platform ticking can update many polygons per tick, and the renderer needs to batch rebuilds anyway.

### 6. Crush damage uses entity height from ECS, not a flat parameter

**Decision:** The current `check_platform_crush()` takes `entity_z` and `entity_height` as parameters. The orchestration layer will query `Position.0.z` and `EntityHeight.0` from each entity occupying the platform polygon and pass them in. This replaces the flat parameter approach.

**Rationale:** Different entity types have different heights (player = 0.8 WU, monsters vary). The existing function signature already supports this; we just need to feed it real data.

### 7. Sound events use SimEvent::SoundTrigger

**Decision:** Emit `SimEvent::SoundTrigger { sound_index, position }` when a platform starts moving, stops moving, and each tick while moving (for looping movement sound). Sound indices are derived from the platform type's Marathon sound definitions (start sound, stop sound, obstructed sound).

**Rationale:** The SimEvent system already exists and is consumed by the audio integration layer. No new event types needed.

## Risks / Trade-offs

**[Mesh rebuild performance]** Platform movement changes polygon heights every tick a platform is moving. If many platforms are active simultaneously, the renderer may need to rebuild many mesh sections each frame. Mitigation: The dirty flag system allows incremental rebuilds, and Marathon levels rarely have more than 5-10 platforms moving simultaneously.

**[Linked platform data source]** The `StaticPlatformData` struct has a `tag` field but no explicit linked platform/light index fields. In Marathon, linked triggers come from line/side data referencing platform tags. Populating `linked_platforms` and `linked_lights` during spawn requires cross-referencing lines, sides, and platform tags from the map data. This parsing may need additions to `marathon-formats` if side trigger data is not yet fully parsed. Mitigation: Start with tag-based linking (platforms sharing the same tag) and expand to line/side triggers as data becomes available.

**[Teleporter platform handling]** Teleporter platforms do not move heights but trigger level transitions. The orchestration needs to special-case them to emit `SimEvent::LevelTeleport` instead of syncing heights. Mitigation: Check `platform_type == Teleporter` early in the tick loop and handle separately.

**[Entity polygon tracking accuracy]** Activation triggers depend on knowing which entities are in which polygons. Currently only the player has `PolygonIndex` tracked via collision. Monster and projectile polygon tracking is approximate. Mitigation: For the initial implementation, focus on player-entry and action-key activation. Monster-entry and projectile activation can be refined as those systems mature.

## Open Questions

- How should the looping movement sound be handled -- should it emit a SoundTrigger every tick, or should there be a persistent spatial sound that the audio layer manages? Leaning toward a "start looping" and "stop looping" event pair to avoid per-tick allocation.
- Should `changed_polygons` be a `Vec<bool>` or a `HashSet<usize>`? Vec<bool> is O(1) per-polygon set/check but uses more memory. HashSet is compact but slower for the typical case. Leaning toward Vec<bool> since polygon counts are bounded (~500 max) and the renderer iterates all polygons anyway.
