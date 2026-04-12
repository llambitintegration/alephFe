## 1. Platform Type Enum and Component Extension

- [ ] 1.1 Add `PlatformType` enum to `marathon-sim/src/components.rs` with variants: `ExtendsFloorToCeiling` (0), `ExtendsCeilingToFloor` (1), `ExtendsFloorAndCeiling` (2), `FromFloor` (3), `FromCeiling` (4), `Teleporter` (5). Derive `Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize`.
- [ ] 1.2 Add `platform_type: PlatformType` field to the `Platform` component struct.
- [ ] 1.3 Add `linked_platforms: Vec<usize>` and `linked_lights: Vec<usize>` fields to the `Platform` component struct.
- [ ] 1.4 Update all existing `Platform` construction sites (unit tests in `platforms.rs`, `make_platform()` helper) to include the new fields with default values (`platform_type: PlatformType::FromFloor`, empty vecs).

## 2. Platform Type-Aware Height Initialization

- [ ] 2.1 Rewrite `spawn_platforms()` in `marathon-sim/src/world.rs` to read `StaticPlatformData.platform_type` and compute `floor_rest`, `floor_extended`, `ceiling_rest`, `ceiling_extended` per the six-type formula using the polygon's initial floor/ceiling heights from `MapGeometry`.
- [ ] 2.2 Populate `linked_platforms` and `linked_lights` from map data during `spawn_platforms()`. Use `StaticPlatformData.tag` to cross-reference platforms sharing the same tag.
- [ ] 2.3 Set `current_floor` and `current_ceiling` to their rest values for each platform type during spawn.
- [ ] 2.4 Add unit tests for `spawn_platforms()` covering each of the 6 platform types to verify correct height range computation.

## 3. MapGeometry Dirty Flag Extension

- [ ] 3.1 Add `changed_polygons: Vec<bool>` and `has_changes: bool` fields to `MapGeometry` in `marathon-sim/src/world.rs`.
- [ ] 3.2 Initialize `changed_polygons` to `vec![false; polygon_count]` and `has_changes` to `false` in `build_map_geometry()`.
- [ ] 3.3 Add a `clear_changes(&mut self)` method on `MapGeometry` that sets `has_changes = false` and fills `changed_polygons` with `false`.
- [ ] 3.4 Update the `MapGeometry` clone in `run_player_physics()` (tick.rs) to include the new fields.

## 4. Re-Activation Logic in platforms.rs

- [ ] 4.1 Extend `activate_platform()` in `marathon-sim/src/world_mechanics/platforms.rs` to handle re-activation: if state is `Extending`, transition to `Returning`; if state is `Returning`, transition to `Extending`; if state is `AtExtended`, transition to `Returning`.
- [ ] 4.2 Add unit tests for re-activation: activate while Extending reverses, activate while Returning reverses, activate while AtExtended starts returning.

## 5. World Mechanics Orchestration in tick.rs

- [ ] 5.1 Add a `run_world_mechanics(&mut self)` method to `SimWorld` in `marathon-sim/src/tick.rs`.
- [ ] 5.2 In `run_world_mechanics()`, query all `Platform` components and call `tick_platform()` on each. After ticking, write `current_floor` and `current_ceiling` into `MapGeometry.floor_heights[polygon_index]` and `MapGeometry.ceiling_heights[polygon_index]`. Set `changed_polygons[polygon_index] = true` and `has_changes = true` for any platform that moved.
- [ ] 5.3 Call `self.run_world_mechanics()` from `SimWorld::tick()` after `self.run_player_physics()` and before the tick counter increment.
- [ ] 5.4 At the start of `run_world_mechanics()`, clear the previous tick's dirty flags by calling `MapGeometry::clear_changes()`.

## 6. Player-Entry and Action-Key Activation

- [ ] 6.1 In `run_world_mechanics()`, after ticking platforms, query the player's `PolygonIndex`. Build a lookup from polygon index to platform entity. If the player's polygon matches a platform, check `should_activate()` with `PlatformTrigger::PlayerEntry` and activate if appropriate.
- [ ] 6.2 Read the `TickInput` action flags. If ACTION is pressed and the player's polygon matches a platform with `ACTIVATE_ON_ACTION_KEY`, call the extended `activate_platform()` (which handles re-activation).
- [ ] 6.3 Add integration tests: player on platform polygon with entry flag triggers activation; player pressing ACTION on action-key platform triggers activation; action on moving platform reverses it.

## 7. Monster and Projectile Activation

- [ ] 7.1 In `run_world_mechanics()`, query all `Monster` entities with `PolygonIndex`. For each monster, check if its polygon matches a platform with `ACTIVATE_ON_MONSTER_ENTRY` and activate if appropriate.
- [ ] 7.2 Query all `Projectile` entities with `PolygonIndex`. For each projectile, check if its polygon matches a platform with `ACTIVATE_ON_PROJECTILE` and activate if appropriate.
- [ ] 7.3 Add unit tests for monster-entry and projectile-impact activation.

## 8. Crush Damage Integration

- [ ] 8.1 In `run_world_mechanics()`, after ticking platforms, for each moving platform query all entities (Player, Monster) with `PolygonIndex` matching the platform polygon. For each such entity, call `check_platform_crush()` with the entity's `Position.0.z` and `EntityHeight.0`.
- [ ] 8.2 If `PlatformCrushResult::Crush`, emit `SimEvent::EntityDamaged` with the entity handle, damage amount, and a platform-crush damage type.
- [ ] 8.3 If `PlatformCrushResult::Reverse`, toggle the platform state: Extending becomes Returning, Returning becomes Extending.
- [ ] 8.4 Add unit tests for crush: crushing platform damages entity (event emitted), non-crushing platform reverses on obstruction.

## 9. Linked Platform and Light Event Dispatch

- [ ] 9.1 In `run_world_mechanics()`, after ticking platforms, for each platform that just reached `AtExtended` or `AtRest`, call `check_platform_triggers()` with the platform's `linked_platforms` and `linked_lights`.
- [ ] 9.2 Process the returned `PlatformTriggerEvent` list: for `ActivatePlatform` events, activate the target platform. For `ToggleLight` events, emit a light toggle event (or directly toggle the light component if the light system is wired).
- [ ] 9.3 Add integration tests for linked platform cascading and light toggling.

## 10. Teleporter Platform Handling

- [ ] 10.1 In `run_world_mechanics()`, when a Teleporter platform (type 5) is activated while the player is on it, emit `SimEvent::LevelTeleport` instead of performing height movement.
- [ ] 10.2 Skip height sync for Teleporter platforms (they do not change MapGeometry).
- [ ] 10.3 Add a unit test verifying teleporter activation emits LevelTeleport and does not modify heights.

## 11. Sound Event Emission

- [ ] 11.1 Track previous platform state before ticking. After ticking, compare to detect state transitions (AtRest->Extending, AtExtended->Returning, etc.).
- [ ] 11.2 Emit `SimEvent::SoundTrigger` for start-movement transitions (AtRest->Extending, AtExtended->Returning) with the platform's start sound index.
- [ ] 11.3 Emit `SimEvent::SoundTrigger` for stop-movement transitions (reaching AtExtended, reaching AtRest) with the platform's stop sound index.
- [ ] 11.4 Add unit test verifying correct sound events are emitted on platform state transitions.

## 12. Control Panel Wiring

- [ ] 12.1 In `run_world_mechanics()` or a dedicated panel-check phase, when ACTION is pressed, check `can_activate_panel()` for each control panel. If a panel with `PanelAction::ActivatePlatform` is activated, call `activate_platform()` on the target platform.
- [ ] 12.2 Add integration test: player facing panel, pressing ACTION, activates the linked platform.

## 13. Renderer Mesh Rebuild Integration

- [ ] 13.1 In `marathon-web/src/mesh.rs` and `marathon-game/src/mesh.rs`, after each tick, check `MapGeometry.has_changes`. If true, iterate `changed_polygons` and rebuild mesh sections for changed polygon indices.
- [ ] 13.2 After processing changes, call `MapGeometry::clear_changes()` (or have the sim clear at tick start per task 5.4).
- [ ] 13.3 Verify visually that platform movement is reflected in the rendered level (doors open, elevators rise).

## 14. Snapshot/Serialization Update

- [ ] 14.1 Update `SimSnapshot` and the `snapshot()`/`deserialize()` methods in `world.rs` to include the new `Platform` fields (`platform_type`, `linked_platforms`, `linked_lights`). Since Platform already derives Serialize/Deserialize, adding the fields should work automatically.
- [ ] 14.2 Add a round-trip serialization test with platforms that have non-default types and linked indices.
