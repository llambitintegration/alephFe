## Why

Weapons fire but nothing happens. The combat system has all the groundwork -- `ProjectileDefinition` records are parsed from physics data (48-byte records with speed, range, damage, area-of-effect, 21 flag bits), projectile entities exist as ECS components with `definition_index` and `distance_traveled`, utility functions for movement/gravity/homing/collision are implemented and tested, and the weapon state machine returns `FireResult` with projectile counts. But the `tick()` method only runs player physics; step 5 ("Projectile physics") in the tick ordering is not wired in. Projectile entities are spawned but never advanced, never collide, and never detonate. This means no weapon in the game deals damage, grenades don't arc and bounce, guided rockets don't track, fusion bolts don't pass through enemies, and explosions don't splash. Combat is non-functional.

Marathon's projectile behaviors are the core of what makes each weapon feel distinct -- the SPNKR's splash radius, the grenade's bounce-and-detonate arc, the fusion pistol's penetrating bolt, the flamethrower's short-range area burn, the alien weapon's seeking behavior. Without the per-tick lifecycle that reads `ProjectileDefinition` flags and drives movement, collision, detonation, and effect spawning, the game cannot progress past "walking around an empty-feeling level."

## What Changes

- **Wire projectile tick into the simulation loop**: Add a projectile physics system that runs at step 5 of `tick()`, iterating all `Projectile` entities each tick to advance position, apply behavioral modifiers, resolve collisions, and handle detonation/cleanup.
- **Flag-driven behavioral modifiers**: Each tick, apply the subset of `ProjectileDefinition` flags that affect movement -- gravity (arc trajectory), guided (homing toward nearest target), wandering (random drift), and persistent (pass through entities instead of detonating on first hit).
- **Wall collision with bounce/ricochet**: When a projectile hits a solid wall, check the `rebounds` flag. Rebounding projectiles reflect their velocity off the wall normal; non-rebounding projectiles detonate at the impact point.
- **Entity collision with damage delivery**: When a projectile intersects an entity's collision volume, calculate and apply damage using the existing `DamageDefinition` and `calculate_damage` pipeline. Persistent projectiles continue through; others detonate.
- **Area-of-effect detonation**: When a projectile with `area_of_effect > 0` detonates (wall hit, entity hit, or range expiry), apply distance-scaled damage to all entities within the blast radius using the existing `calculate_aoe_damage` function.
- **Media interaction**: Detect when a projectile enters a liquid media surface (water, lava, sewage). Promote the projectile if `media_projectile_promotion` is set (e.g., a projectile that changes type on water contact), and spawn the `media_detonation_effect`.
- **Effect spawning**: Spawn `detonation_effect` entities on detonation, `contrail_effect` entities at intervals during flight (respecting `ticks_between_contrails` and `maximum_contrails`), and rebound sound on bounce.
- **Projectile spawning from weapons and monsters**: Connect `FireResult` from the weapon system and `MonsterAction::RangedAttack` from monster AI to actually spawn `Projectile` entities with correct initial position, velocity, and definition index.

## Capabilities

### New Capabilities

_(none -- the combat-system spec already defines projectile creation, movement, collision, homing, area-of-effect, and damage application as requirements; this change implements them)_

### Modified Capabilities

- `combat-system`: Projectile lifecycle requirements move from specified-but-unimplemented to functional. All six projectile-related requirements in the spec gain working implementations: projectile creation and movement, homing tracking, collision detection (wall and entity), damage calculation/application, and area-of-effect detonation.
- `game-loop`: The `tick()` method gains a projectile physics step that runs after weapon/combat and before damage resolution, completing the system execution order defined in the spec.

## Impact

- `marathon-sim/src/tick.rs` -- Add `run_projectile_physics()` call at step 5 in `tick()`. This is the primary integration point.
- `marathon-sim/src/combat/projectiles.rs` -- Currently contains stateless utility functions. Needs a system-level function that queries all `Projectile` entities, reads their `ProjectileDefinition` from physics data, and orchestrates the per-tick lifecycle (advance, modify, collide, detonate, cleanup). The existing utility functions (`advance_projectile`, `apply_projectile_gravity`, `apply_homing`, `check_projectile_wall_collision`, `check_projectile_entity_collision`, `check_range_limit`) are called from within this orchestrator.
- `marathon-sim/src/combat/damage.rs` -- No structural changes; `calculate_damage` and `calculate_aoe_damage` are already correct. They just need to be called from the projectile detonation path.
- `marathon-sim/src/combat/weapons.rs` -- No changes to weapon state machine. The integration layer that reads `FireResult` and spawns projectile entities may live in `tick.rs` or a new `combat/spawn.rs`.
- `marathon-sim/src/components.rs` -- `Projectile` component may need additional fields: `contrails_spawned` counter, `current_polygon` for spatial queries, and possibly `target_entity` for homing tracking. `ProjectileSource` is already defined.
- `marathon-sim/src/world.rs` -- `ProjectileSnapshot` may need additional fields to match new component state. `SimWorld` needs access to `PhysicsData.projectiles` definitions (may already be stored or may need a new resource).
- `marathon-sim/src/monster/ai.rs` -- `MonsterAction::RangedAttack` is defined but the action handler that spawns a projectile entity needs to be connected.
- `marathon-formats/src/physics.rs` -- No changes needed; `ProjectileDefinition` with all fields and flags is already parsed correctly.
