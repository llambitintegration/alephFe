## 1. Projectile Flags and Component Extensions

- [ ] 1.1 Define `ProjectileFlags` module in `marathon-sim/src/combat/projectiles.rs` with named `pub const u32` values for all 21 flag bits (`GUIDED`, `STOP_WHEN_ANIMATION_LOOPS`, `PERSISTENT`, `ALIEN_PROJECTILE`, `AFFECTED_BY_GRAVITY`, `REBOUNDS_FROM_FLOOR`, `BLEEDING`, `USUALLY_PASS_TRANSPARENT_SIDE`, `SOMETIMES_PASS_TRANSPARENT_SIDE`, `DOUBLE_GRAVITY`, `REBOUNDS_FROM_WALLS`, `CAN_TOGGLE_CONTROL_PANELS`, `POSITIVE_VERTICAL_ERROR`, `MELEE_PROJECTILE`, `PERSISTENT_AND_VIRULENT`, `BECOMES_ITEM_ON_DETONATION`, `BLEEDING_PROJECTILE`, `HORIZONTAL_WANDER`, `VERTICAL_WANDER`, `HALF_GRAVITY`, `PASSES_MEDIA_BOUNDARY`)
- [ ] 1.2 Extend `Projectile` component in `marathon-sim/src/components.rs` with `contrails_spawned: u16`, `ticks_alive: u16`, `current_polygon: usize`
- [ ] 1.3 Add `HomingTarget` component (`pub struct HomingTarget(pub Vec3)`) to `marathon-sim/src/components.rs`
- [ ] 1.4 Update `ProjectileSnapshot` in `marathon-sim/src/world.rs` to include `ticks_alive`, `contrails_spawned`, `current_polygon` fields
- [ ] 1.5 Update snapshot serialization/deserialization in `world.rs` to read/write the new `Projectile` and `ProjectileSnapshot` fields

## 2. Velocity Reflection Helpers

- [ ] 2.1 Implement `reflect_velocity_wall(velocity: Vec3, wall_a: Vec2, wall_b: Vec2) -> Vec3` in `combat/projectiles.rs` that reflects the XY velocity across the wall normal, preserving Z
- [ ] 2.2 Implement `reflect_velocity_floor(velocity: Vec3, energy_loss: f32) -> Vec3` in `combat/projectiles.rs` that negates Z velocity with energy loss factor
- [ ] 2.3 Add unit tests: wall reflection reverses approach direction, floor reflection preserves XY and reverses Z, energy loss reduces bounce height

## 3. Detonation Logic

- [ ] 3.1 Define `DetonationResult` struct in `combat/projectiles.rs`: `effect_to_spawn: Option<(Vec3, usize)>` (position, effect definition index), `aoe_damages: Vec<(Entity, i16)>`, `sound_event: Option<(usize, Vec3)>`, `despawn_projectile: Entity`
- [ ] 3.2 Implement `compute_detonation(projectile_entity, hit_point, def, entities_in_radius, rng) -> DetonationResult` function that computes direct hit damage (if entity hit), AoE damage for all entities in `area_of_effect` radius, selects `detonation_effect` vs `media_detonation_effect` based on submersion state, and marks the projectile for despawn
- [ ] 3.3 Implement `apply_detonation_result(world, result)` helper that spawns `Effect` entities, applies damage to `Health`/`Shield` components, emits `SimEvent::EntityDamaged` and `SimEvent::EntityKilled`, and despawns the projectile
- [ ] 3.4 Add unit tests: detonation with no AoE just despawns and spawns effect; detonation with AoE applies scaled damage; media detonation uses media effect; no effect spawned when `detonation_effect == -1`

## 4. Projectile Spawning

- [ ] 4.1 Implement `spawn_projectile(world, def_index, position, velocity, source_entity, polygon_index)` function in `combat/projectiles.rs` (or a new `combat/spawn.rs`) that spawns a projectile ECS entity with all required components (`Projectile`, `Position`, `Velocity`, `PolygonIndex`, `ProjectileSource`, and optionally `HomingTarget` if `GUIDED` flag is set)
- [ ] 4.2 Implement `spawn_projectiles_from_fire_result(world, fire_result, trigger_def, player_pos, player_facing, player_polygon)` that interprets a `FireResult` and calls `spawn_projectile` the correct number of times with theta_error spread
- [ ] 4.3 Wire weapon `FireResult` consumption into `tick()`: after the weapon/combat step, read any pending `FireResult` and call `spawn_projectiles_from_fire_result`
- [ ] 4.4 Wire monster ranged attack: when monster AI produces a ranged attack action, call `spawn_projectile` with the monster's projectile type, position, and direction toward target
- [ ] 4.5 Add unit tests: spawned projectile has correct components, burst fire spawns correct count with spread, monster projectile has correct source

## 5. Projectile Physics Orchestrator

- [ ] 5.1 Implement `run_projectile_physics(&mut self)` on `SimWorld` using collect-then-process pattern: collect all `(Entity, Projectile, Position, Velocity, PolygonIndex)` into a `Vec`, then process each
- [ ] 5.2 For each projectile, read `ProjectileDefinition` from `PhysicsTables.data.projectiles[def_index]`
- [ ] 5.3 Apply gravity: if `AFFECTED_BY_GRAVITY`, call `apply_projectile_gravity()` with gravity constant `1.0/120.0`, multiplied by 2.0 for `DOUBLE_GRAVITY` or 0.5 for `HALF_GRAVITY`
- [ ] 5.4 Apply homing: if `GUIDED`, read `HomingTarget` component (or compute from player aim ray / monster target), call `apply_homing()`
- [ ] 5.5 Apply wander: if `HORIZONTAL_WANDER` or `VERTICAL_WANDER`, generate random perturbation from `SimRng` and add to velocity
- [ ] 5.6 Advance position: call `advance_projectile()`, update `distance_traveled` and `ticks_alive`
- [ ] 5.7 Check wall collision: call `check_projectile_wall_collision()` using polygon adjacency data; handle rebounds (reflect velocity, emit sound) vs detonation; handle transparent wall pass-through based on flags and `SimRng`
- [ ] 5.8 Check floor/ceiling collision: compare new Z against `floor_heights[current_polygon]` and `ceiling_heights[current_polygon]`; handle floor rebound vs detonation; ceiling always detonates
- [ ] 5.9 Check entity collision: call `check_projectile_entity_collision()` against monsters and player (excluding source entity); for persistent projectiles apply damage without detonation; for non-persistent, apply damage and detonate
- [ ] 5.10 Check media interaction: compare position Z against media surface heights in the current polygon; handle promotion (replace projectile), media boundary detonation, or pass-through based on flags
- [ ] 5.11 Check animation-based detonation: if `STOP_WHEN_ANIMATION_LOOPS` and `ticks_alive >= 15`, detonate (tick-based approximation)
- [ ] 5.12 Check range limit: call `check_range_limit()`; if exceeded, despawn without detonation effect
- [ ] 5.13 Spawn contrails: if `contrail_effect >= 0` and `ticks_alive % ticks_between_contrails == 0` and `contrails_spawned < maximum_contrails`, spawn `Effect` entity and increment counter
- [ ] 5.14 Update components: write back modified `Position`, `Velocity`, `Projectile` fields, and `PolygonIndex` to the world

## 6. Integration into tick()

- [ ] 6.1 Add `self.run_projectile_physics()` call in `SimWorld::tick()` after player physics (step 5 position), before tick counter increment
- [ ] 6.2 Ensure `PhysicsTables` resource is accessible within `run_projectile_physics()` (already inserted during `SimWorld::new()`)
- [ ] 6.3 Ensure `MapGeometry` resource (floor heights, ceiling heights, polygon adjacency) is accessible for collision checks
- [ ] 6.4 Ensure `SimRng` resource is accessible for damage randomness, wander, and transparent wall pass-through

## 7. Testing

- [ ] 7.1 Unit test: projectile advances position each tick and accumulates distance
- [ ] 7.2 Unit test: gravity-affected projectile arcs downward over multiple ticks
- [ ] 7.3 Unit test: homing projectile turns toward target over multiple ticks
- [ ] 7.4 Unit test: projectile detonates on solid wall hit, spawning detonation effect
- [ ] 7.5 Unit test: projectile with `REBOUNDS_FROM_WALLS` reflects velocity on wall hit
- [ ] 7.6 Unit test: projectile with `REBOUNDS_FROM_FLOOR` bounces on floor contact
- [ ] 7.7 Unit test: non-persistent projectile detonates on entity hit, dealing damage
- [ ] 7.8 Unit test: persistent projectile passes through entity, dealing damage without detonation
- [ ] 7.9 Unit test: AoE detonation applies distance-scaled damage to entities within radius
- [ ] 7.10 Unit test: projectile despawned (no effect) when exceeding maximum range
- [ ] 7.11 Unit test: contrails spawn at correct intervals up to maximum count
- [ ] 7.12 Unit test: projectile promoted when entering media with `media_projectile_promotion >= 0`
- [ ] 7.13 Integration test: full tick with weapon fire produces projectile that advances, collides, and detonates across multiple ticks
- [ ] 7.14 Integration test: snapshot round-trip preserves in-flight projectile state
