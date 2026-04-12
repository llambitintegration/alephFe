## Why

Platforms in Marathon are the primary mechanism for elevators, doors, crushers, and puzzle triggers. The platform state machine exists in `platforms.rs` with correct Extending/AtExtended/Returning/AtRest transitions, activation trigger checks, crush detection, and linked platform/light events. However, none of this code is wired into the game. `tick_platform()` is never called from `SimWorld::tick()`, so every platform sits permanently at rest. Even if platforms did move, `MapGeometry.floor_heights` and `ceiling_heights` are never updated to reflect platform positions, so collision and rendering would still treat the polygon as static. There are also no activation paths: the player can stand on a platform polygon or press the action key and nothing happens because no system checks `should_activate()` or calls `activate_platform()`. This means elevators do not move, doors do not open, crushers do not crush, and linked platform chains do not cascade -- making many Marathon levels uncompletable.

## What Changes

- **Call `tick_platform()` from the tick loop**: Add a `run_world_mechanics()` phase to `SimWorld::tick()` (step 7 per the existing comment) that iterates all `Platform` components and advances their state machines each tick.
- **Sync MapGeometry with platform positions**: After ticking platforms, write each platform's `current_floor` and `current_ceiling` back into `MapGeometry.floor_heights[polygon_index]` and `ceiling_heights[polygon_index]`. This makes collision (player physics already reads MapGeometry) and rendering (mesh builders read floor/ceiling heights) reflect the platform's actual position.
- **Platform type-aware height calculation**: The `Platform` component currently only tracks floor movement. Extend `spawn_platforms()` and the `Platform` struct to handle all 6 Marathon platform types: extends-floor-to-ceiling, extends-ceiling-to-floor, extends-floor-and-ceiling, from-floor, from-ceiling, and teleporter. Each type determines which of `floor_rest/floor_extended/ceiling_rest/ceiling_extended` are set based on the polygon's initial heights and the platform's min/max range.
- **Player-entry activation**: Each tick, check if the player's current polygon is a platform polygon. If the platform is at rest and has the `ACTIVATE_ON_PLAYER_ENTRY` flag, call `activate_platform()`.
- **Action-key activation**: When the player presses ACTION while on a platform polygon with `ACTIVATE_ON_ACTION_KEY`, or when the player activates a control panel linked to a platform (already modeled in `panels.rs`), activate the target platform. Also handle re-activation (pressing action on an already-extending platform to reverse it, matching Marathon behavior).
- **Monster and projectile activation**: Check monster polygon positions against platform polygons each tick for `ACTIVATE_ON_MONSTER_ENTRY`. Check projectile polygon positions for `ACTIVATE_ON_PROJECTILE`.
- **Crush damage integration**: When a platform is moving and entities occupy its polygon, call `check_platform_crush()`. If crushing, emit `SimEvent::EntityDamaged`. If reversing, toggle the platform direction. Use per-entity height/z checks rather than the current flat entity_z parameter.
- **Door behavior (auto-return)**: The return_delay mechanism already exists in the state machine. Ensure door-type platforms (extends-floor-to-ceiling, extends-ceiling-to-floor) automatically start extending on entry and return after their delay, matching Marathon's door feel.
- **Linked platform cascading**: When a platform reaches AtExtended or AtRest, call `check_platform_triggers()` to fire linked platform activations and light toggles. Process the returned `PlatformTriggerEvent` list by activating the referenced platforms/lights. The linked platform/light indices need to be stored per-platform (sourced from the map's side/line trigger data or the platform tag).
- **Sound events**: Emit `SimEvent::SoundTrigger` when a platform starts moving, stops, and while in motion (looping movement sound). Sound indices come from the platform type's sound definitions.
- **Mesh rebuild notification**: Expose a way for renderers to detect that polygon heights have changed so they can rebuild affected mesh sections, either via a dirty flag on MapGeometry or via SimEvents.

## Capabilities

### New Capabilities

- `platform-activation`: Detection and dispatch of platform activation triggers from player entry, action key, monster entry, projectile impact, and linked platform cascading.
- `platform-geometry-sync`: Per-tick writeback of platform floor/ceiling positions into MapGeometry so collision and rendering reflect moving platforms.

### Modified Capabilities

- `world-mechanics`: Platform ticking integrated into the tick loop. Platform type-specific height ranges computed from map data. Crush damage and platform reversal applied to entities. Linked platform/light trigger events processed. Sound events emitted.
- `game-loop`: `SimWorld::tick()` gains a world mechanics phase that runs platform, light, and media systems after player physics and before cleanup.
- `player-physics`: Player collision already reads MapGeometry floor/ceiling heights. No code changes needed, but behavior changes because those heights now move.

## Impact

- `marathon-sim/src/tick.rs` -- Add `run_world_mechanics()` call in `tick()` after player physics. This method iterates Platform entities, calls `tick_platform()`, syncs MapGeometry, checks activation triggers, processes crush checks, and dispatches linked events.
- `marathon-sim/src/world_mechanics/platforms.rs` -- Extend `activate_platform()` to handle re-activation (reverse while extending). Add platform type enum and type-aware height initialization. Add `PlatformLinks` component or field for storing linked platform/light indices per platform. No changes to the core state machine (`tick_platform`, `move_toward`) which is already correct.
- `marathon-sim/src/components.rs` -- Add `platform_type` field to `Platform` (enum: ExtendsFloorToCeiling, ExtendsCeilingToFloor, ExtendsFloorAndCeiling, FromFloor, FromCeiling, Teleporter). Possibly add a `PlatformLinks` component with `linked_platforms: Vec<usize>` and `linked_lights: Vec<usize>`.
- `marathon-sim/src/world.rs` -- Update `spawn_platforms()` to compute rest/extended heights per platform type using the polygon's initial floor/ceiling from MapGeometry. Populate linked platform/light data from map side/line references. Update `MapGeometry` struct if a dirty flag or changed-polygons list is added.
- `marathon-sim/src/world_mechanics/panels.rs` -- Wire `PanelAction::ActivatePlatform` into the new activation system (already structurally ready, just needs to call through).
- `marathon-web/src/mesh.rs`, `marathon-game/src/mesh.rs` -- Check for changed polygon heights and rebuild affected mesh sections. Currently meshes are built once at level load; will need incremental or full rebuild when platforms move.
- All existing platform unit tests in `platforms.rs` remain valid. New tests needed for: platform type height calculation, MapGeometry sync, activation trigger dispatch, crush damage event emission, linked platform cascading, and door auto-return behavior.
