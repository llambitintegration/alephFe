## Context

`SimWorld::tick()` in `marathon-sim/src/tick.rs` currently executes only one system: `run_player_physics()`. The player can move, collide with walls, and respond to input, but everything else in the world is frozen. Seven other simulation subsystems -- lights, media, platforms, monsters, projectiles, effects, and items -- have their core logic fully implemented as standalone pure functions in `world_mechanics/` and `combat/`, each with comprehensive unit tests. None of these functions are called from the tick loop.

This is the single highest-impact blocker for a playable game. Without wiring these systems into `tick()`, platforms are inert, lights never animate, monsters stand as statues, weapons cannot fire, and projectiles do not exist.

The original alephone C codebase runs all systems in a fixed order inside `update_world()`. This change replicates that ordering.

## Goals / Non-Goals

**Goals:**
- Wire all 8 simulation systems into `SimWorld::tick()` in the canonical alephone `update_world()` order
- Each system becomes a private method on `SimWorld`, called sequentially from `tick()`
- Platform floor/ceiling height changes are written back to `MapGeometry` so collision and rendering see them
- Media height tracks its associated light's intensity each tick
- Monster AI runs: vision checks, state transitions, movement, attacks, alert cascading
- Weapons fire from player action flags, spawning projectiles as ECS entities
- Projectiles advance, apply gravity/homing, check collisions, deal damage, despawn
- Effects count down and despawn
- Items detect player overlap for pickup; respawn timers tick in multiplayer
- All tick methods use `SimRng` for determinism (light flicker, damage rolls)

**Non-Goals:**
- New algorithms or data structures (all subsystem logic already exists)
- New public API surface (`SimWorld::tick(TickInput)` signature unchanged)
- Multiplayer-specific item respawn spawning (timer logic ticks, but spawning items back is deferred)
- Control panel activation wiring (requires raycast against side textures; separate change)
- Terminal interaction (separate UI concern)
- Sound event emission (the `SimEvents` resource exists; wiring sound triggers is a follow-up)

## Decisions

### 1. Tick ordering matches alephone's update_world()

**Decision:** Systems execute in this fixed order every tick:
1. Update lights
2. Update media
3. Update platforms
4. Update player physics (already implemented)
5. Update monsters
6. Update projectiles
7. Update effects
8. Update items
9. Advance tick counter

**Rationale:** This matches the original C engine's `update_world()` ordering. Lights must run before media (media height depends on light intensity). Platforms must run before player physics (floor/ceiling heights affect collision). Player physics must run before monsters (monsters need the player's updated position for vision/targeting). Monsters must run before projectiles (monster attacks can spawn projectiles that are processed in the same tick). Effects and items are independent cleanup and can run last.

**Alternative considered:** Running systems in parallel or grouping by ECS schedule. Rejected because determinism and order-dependent interactions (light -> media, platform -> collision, monster -> projectile spawn) require strict sequential execution.

### 2. Each system is a private method on SimWorld

**Decision:** Add private methods `update_lights()`, `update_media()`, `update_platforms()`, `update_monsters()`, `update_projectiles()`, `update_effects()`, `update_items()` alongside the existing `run_player_physics()`. Each method borrows `&mut self` and operates on the bevy_ecs World directly.

**Rationale:** This follows the pattern established by `run_player_physics()`. Private methods keep the integration wiring in one file (`tick.rs`) while the actual algorithms remain as standalone testable functions in their respective modules. The methods handle ECS queries, resource access, and component mutation; the standalone functions handle the math.

**Alternative considered:** Bevy-style system functions registered in a schedule. Rejected because the current architecture uses manual `World` access (not App/Schedule), and the sequential ordering requirement means a schedule adds complexity without benefit.

### 3. Platform heights written back to MapGeometry

**Decision:** After ticking each platform, write its `current_floor` and `current_ceiling` back to `MapGeometry::floor_heights` and `MapGeometry::ceiling_heights` for the platform's controlled polygon.

**Rationale:** `MapGeometry` is the single source of truth for collision (used by player physics and projectile collision). If platform heights only live on the `Platform` component without being reflected in `MapGeometry`, the player will walk through moving platforms and projectiles will ignore them. The existing `MapGeometry` fields are `pub` and `Vec<f32>`, so writing them is straightforward.

### 4. Monster update clones geometry to avoid borrow conflicts

**Decision:** Clone `MapGeometry` (or the specific floor_heights slice) before iterating monsters, same as `run_player_physics()` does.

**Rationale:** The ECS World holds both MapGeometry (as a resource) and monster entities (as components). Iterating monster entities with `&mut` borrows the World mutably, conflicting with an immutable borrow of MapGeometry. Cloning the geometry before the loop is the established pattern in this codebase. The geometry data is small (a few KB for typical levels) so the clone cost is negligible.

**Alternative considered:** Using `world.resource_scope()` or splitting into separate `World` queries. The clone approach is simpler and already proven in the player physics method.

### 5. Weapon firing wired through player physics pass

**Decision:** Add weapon tick logic to `run_player_physics()` (or a separate `run_player_weapons()` called immediately after it). Read `FIRE_PRIMARY`/`FIRE_SECONDARY` from action flags. Call `tick_weapon()` on each weapon slot. If a weapon fires, spawn a projectile entity with the appropriate definition, position, and velocity.

**Rationale:** Weapon state is per-player and depends on player position/facing (for projectile spawn offset and direction). Running it adjacent to player physics keeps the player-focused systems together. Spawned projectiles are then processed in the subsequent `update_projectiles()` pass.

### 6. Entity despawn via deferred command pattern

**Decision:** Collect entity IDs to despawn during each update method, then despawn them after the query loop completes.

**Rationale:** bevy_ecs does not allow despawning entities while iterating them in a query. Collecting IDs into a Vec and despawning afterward is the standard pattern. This applies to: projectiles that hit something or exceed range, effects whose timer reaches zero, items that are picked up, and monsters whose death animation completes.

## Risks / Trade-offs

**[Borrow conflict complexity]** The bevy_ecs manual World API makes it cumbersome to access multiple resources and components simultaneously. Each update method will need to carefully extract data before entering query loops. Mitigation: Follow the clone-before-loop pattern from `run_player_physics()`. Keep each method focused on one system to limit the number of concurrent borrows.

**[Missing weapon/inventory ECS components]** The player entity currently has no weapon inventory components. The `WeaponSlot` and `DualWieldState` types exist but are not spawned as ECS components or resources. Mitigation: Add a `PlayerWeapons` resource (or component) during this change, initialized from physics data. The weapon system wiring can start simple (fists only) and be extended.

**[Projectile polygon tracking]** Projectiles need to know which polygon they are in for wall collision checks. The current `check_projectile_wall_collision()` takes a `current_polygon` parameter. Mitigation: Add a `PolygonIndex` component to projectile entities (already exists as a component type). Update it when projectiles cross polygon boundaries.

**[Monster pathfinding not yet integrated]** `monster/pathfinding.rs` exists but ground-monster movement toward the player requires BFS/shortest-path on the polygon adjacency graph. Mitigation: For this change, use a simple direct-movement approach (move toward player in world space, stopping at walls). Proper pathfinding integration can follow.

**[Performance with many entities]** Each update method does a full query over its entity type. Marathon levels typically have <50 monsters and <20 projectiles, so this is not a concern. If profiling reveals issues, query caching or parallel iteration can be added later.
