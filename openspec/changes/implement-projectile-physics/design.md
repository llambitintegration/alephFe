## Context

The marathon-sim crate has all the building blocks for projectile physics -- `advance_projectile()`, `apply_projectile_gravity()`, `apply_homing()`, `check_projectile_wall_collision()`, `check_projectile_entity_collision()`, `check_range_limit()`, `calculate_damage()`, `calculate_aoe_damage()`, and `apply_damage()` -- but none of them are called from `SimWorld::tick()`. Projectile entities are spawned with `Projectile { definition_index, distance_traveled }`, `Position`, and `Velocity` components, but they sit inert because step 5 ("Projectile physics") in the tick ordering is not implemented. The weapon system produces `FireResult` values that are never consumed to spawn projectile entities. `ProjectileDefinition` records with 21 behavioral flags are fully parsed from physics data but never inspected at runtime.

The result: weapons fire but nothing happens. No damage, no arcing grenades, no homing rockets, no splash damage, no bouncing, no contrails, no media interaction. Combat is non-functional.

## Goals / Non-Goals

**Goals:**
- Wire a projectile physics system into `tick()` at step 5 that runs the full per-tick lifecycle for every active `Projectile` entity
- Implement flag-driven behavioral dispatch for all 21 `ProjectileDefinition` flags
- Implement wall and entity collision with bounce/ricochet support
- Implement detonation with area-of-effect damage, effect spawning, and entity cleanup
- Implement media surface interaction (promotion, media detonation effects)
- Implement contrail spawning during flight
- Connect `FireResult` from the weapon system to projectile entity spawning
- Connect `MonsterAction::RangedAttack` to monster projectile spawning

**Non-Goals:**
- Flyby sound proximity detection (parsed but deferred to audio integration pass)
- Full animation-driven detonation for `_stop_when_animation_loops` (requires animation system; use a tick-based approximation)
- `_becomes_item_on_detonation` item spawning (requires item placement system not yet built)
- `_can_toggle_control_panels` panel activation (requires panel interaction system not yet built)
- Projectile-activated platforms (requires platform activation system integration)

## Decisions

### 1. Single orchestrator function, not an ECS schedule

**Decision:** Implement `run_projectile_physics(&mut self)` as a method on `SimWorld` (matching the pattern of `run_player_physics()`), called from `tick()` after weapon/combat and before damage resolution. This function queries all `Projectile` entities, reads their `ProjectileDefinition` from `PhysicsTables`, and runs the lifecycle in a collect-then-process pattern.

**Rationale:** The existing codebase uses direct `World` queries in methods on `SimWorld`, not bevy_ecs schedules or systems. Following the established pattern avoids introducing a second architectural style. The collect-then-process pattern (collect entity IDs first, then process each one with mutable world access) is necessary because processing a projectile may spawn or despawn entities, which invalidates query iterators.

**Alternative considered:** Adding a bevy_ecs `Schedule` with systems. Rejected because no other part of the codebase uses schedules, and introducing them for one subsystem would create inconsistency.

### 2. Flag constants as a bitflags module

**Decision:** Define projectile flag constants as `pub const` values in a `ProjectileFlags` module within `combat/projectiles.rs`, matching the 21 flags from the original engine's `weapon_definitions.h`.

**Rationale:** The flags are already parsed as a `u32` in `ProjectileDefinition.flags`. Defining named constants keeps the dispatch code readable (`if def.flags & ProjectileFlags::GUIDED != 0`) without adding a dependency on the `bitflags` crate. This matches how other flag checks in the codebase work (e.g., line flags in `world.rs`).

**Alternative considered:** Using the `bitflags` crate. Acceptable but adds a dependency for what amounts to 21 named constants and bitwise AND checks.

### 3. Velocity reflection for bouncing, not position correction

**Decision:** When a projectile hits a wall or floor with a rebound flag, reflect the velocity vector across the surface normal and place the projectile at the hit point. Do not attempt to simulate the remaining travel distance after the bounce within the same tick.

**Rationale:** Marathon's original engine processes bounces this way -- the projectile stops at the hit point with reflected velocity, and continues from there on the next tick. Simulating sub-tick post-bounce travel adds complexity with negligible visual difference at 30Hz.

### 4. Guided projectiles track a world-space aim point

**Decision:** For player-fired guided projectiles, the homing target is a point computed by ray-casting from the player's camera position along their look direction. For monster-fired guided projectiles, the target is the monster's target entity position. The existing `apply_homing()` function already accepts a `Vec3` target, so both cases are handled uniformly.

**Rationale:** Marathon's guided rockets track the player's crosshair direction, not a locked-on entity. This makes rocket guidance a skill-based mechanic. The player aim point is computed as `player_pos + look_direction * range`, where range is large enough to represent "far away."

### 5. Additional component fields on Projectile

**Decision:** Extend the `Projectile` component with `contrails_spawned: u16`, `ticks_alive: u16`, and `current_polygon: usize`. Add a `HomingTarget` component (optional, only on guided projectiles) that stores the target `Vec3` updated each tick.

**Rationale:** `contrails_spawned` is needed to enforce `maximum_contrails`. `ticks_alive` is needed for `ticks_between_contrails` interval and the `_stop_when_animation_loops` approximation. `current_polygon` is needed for spatial queries (wall collision uses polygon adjacency). `HomingTarget` as a separate component keeps the common `Projectile` struct small and avoids an `Option<Vec3>` on every projectile.

### 6. Gravity constant from Marathon's original value

**Decision:** Use Marathon's gravity constant of `1/120 WU/tick^2` (approximately 0.00833 world units per tick squared). Double-gravity and half-gravity flags multiply this by 2.0 and 0.5 respectively.

**Rationale:** This value comes directly from the Alephone source (`GRAVITATIONAL_ACCELERATION = WORLD_ONE/120`). Using the original constant ensures grenades, flames, and other gravity-affected projectiles arc at the correct rate.

### 7. Detonation as a helper function producing a list of effects

**Decision:** Implement `detonate_projectile()` as a helper that returns a `DetonationResult` containing: effect entities to spawn, damage events to emit, and the projectile entity to despawn. The caller applies these changes to the world after the helper returns.

**Rationale:** Separating the "compute what happens" from "mutate the world" makes the detonation logic testable in isolation and avoids re-entrancy issues with world mutation during iteration.

## Per-Tick Projectile Lifecycle

Each tick, for every active `Projectile` entity:

```
1. Read ProjectileDefinition from PhysicsTables[projectile.definition_index]
2. Apply gravity modifier (if AFFECTED_BY_GRAVITY flag):
   - velocity.z -= gravity_constant * gravity_multiplier
   - gravity_multiplier: 2.0 if DOUBLE_GRAVITY, 0.5 if HALF_GRAVITY, else 1.0
3. Apply homing (if GUIDED flag):
   - Compute target position (player aim ray or monster target entity)
   - velocity = apply_homing(velocity, position, target, turning_rate)
4. Apply wander (if HORIZONTAL_WANDER or VERTICAL_WANDER):
   - Add small random perturbation to velocity direction using SimRng
5. Advance position:
   - (new_pos, dist) = advance_projectile(position, velocity)
   - projectile.distance_traveled += dist
   - projectile.ticks_alive += 1
6. Check wall collision:
   - result = check_projectile_wall_collision(old_pos, new_pos, ...)
   - If Hit and REBOUNDS_FROM_WALLS: reflect velocity, play rebound_sound, set pos to hit_point
   - If Hit and not rebounds: detonate at hit_point
   - If passable wall and USUALLY_PASS_TRANSPARENT_SIDE: pass through
   - If passable wall and SOMETIMES_PASS_TRANSPARENT_SIDE: 50% chance pass
7. Check floor/ceiling collision:
   - If new_pos.z <= floor_height and REBOUNDS_FROM_FLOOR: reflect Z velocity, play rebound_sound
   - If new_pos.z <= floor_height and not rebounds: detonate
   - If new_pos.z >= ceiling_height: detonate
8. Check entity collision:
   - result = check_projectile_entity_collision(old_pos, new_pos, entities)
   - If hit and PERSISTENT: apply damage, continue (do not detonate)
   - If hit and not persistent: apply damage, detonate
9. Check media boundary:
   - If entering liquid and media_projectile_promotion >= 0: replace with promoted type
   - If entering liquid and not PASSES_MEDIA_BOUNDARY: detonate, spawn media_detonation_effect
10. Check animation-based detonation (STOP_WHEN_ANIMATION_LOOPS):
    - Approximate: detonate after a fixed tick count (e.g., 15 ticks)
11. Check range limit:
    - if check_range_limit(distance_traveled, maximum_range): despawn (no detonation effect)
12. Spawn contrail (if contrail_effect >= 0):
    - if ticks_alive % ticks_between_contrails == 0 and contrails_spawned < maximum_contrails:
      spawn Effect entity at current position, increment contrails_spawned
```

## AoE Damage Calculation

When a projectile with `area_of_effect > 0` detonates:

```
detonation_pos = hit_point or current position
aoe_radius = world_coord(def.area_of_effect)

for each entity with (Position, Health) within aoe_radius of detonation_pos:
    distance = |entity.position - detonation_pos|
    if distance < aoe_radius:
        base_dmg = calculate_damage(def.damage, entity.immunities, entity.weaknesses, rng)
        scaled_dmg = calculate_aoe_damage(base_dmg, distance, aoe_radius)
        apply_damage(scaled_dmg, entity.health, entity.shield)
        emit SimEvent::EntityDamaged
```

Direct-hit damage (the entity that was actually hit by the projectile) uses `calculate_damage()` at full scale, independent of the AoE pass. The AoE pass may also hit the same entity, which is correct behavior (direct hit + splash).

## Effect Spawning

Detonation effects are spawned as `Effect` entities with a `ticks_remaining` countdown. The `definition_index` maps to `EffectDefinition` in the physics data. The integration layer uses this to look up the correct collection/shape/sound for rendering.

Contrail effects are spawned at the projectile's current position during flight, using `contrail_effect` as the definition index. They are independent entities that expire on their own timer.

## Risks / Trade-offs

**[Collect-then-process overhead]** Collecting all projectile entity IDs into a `Vec` before processing adds a small allocation per tick. With Marathon's typical projectile counts (< 50 simultaneous), this is negligible.

**[Sub-tick bounce accuracy]** Not simulating post-bounce travel within the same tick means a bouncing grenade "loses" up to one tick of travel distance at each bounce. At 30Hz with Marathon's projectile speeds, this is imperceptible.

**[Animation loop approximation]** Using a fixed tick count for `_stop_when_animation_loops` instead of actual animation frame counting is imprecise. This affects flamethrower puff lifetime. Can be refined when the animation system is built.

**[Polygon tracking]** Projectiles need to know their current polygon for wall collision checks. Tracking polygon transitions as projectiles cross polygon boundaries adds complexity. Initial implementation: start with the source entity's polygon and update via adjacency as the projectile crosses lines.

## Open Questions

- Should the projectile system emit `SimEvent::SoundTrigger` for detonation/rebound sounds, or should the integration layer infer sounds from effect entities? Leaning toward explicit sound events for precision.
- What tick count should approximate `_stop_when_animation_loops` for flame puffs? The original uses shape animation frame count; 15 ticks (~0.5s) is a reasonable starting point.
- Should `ProjectileSource` be extended to track whether the source is a player or monster, for friendly-fire rules? The component currently stores a bare `Entity`.
