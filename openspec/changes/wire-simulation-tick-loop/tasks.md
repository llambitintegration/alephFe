## 1. Lights System

- [ ] 1.1 Add `update_lights()` private method on `SimWorld` in `tick.rs`: query all `Light` entities, get `TickCounter` and `SimRng` resources, call `compute_light_intensity()` for each light, write back `current_intensity`
- [ ] 1.2 Call `self.update_lights()` from `tick()` as the first system (before media)
- [ ] 1.3 Add integration test: construct SimWorld with a smooth-function light, tick 30 times, verify `current_intensity` reaches maximum at half-period
- [ ] 1.4 Add integration test: construct two SimWorlds with same seed and flickering light, tick both N times, verify identical intensities (determinism)

## 2. Media System

- [ ] 2.1 Add `update_media()` private method on `SimWorld` in `tick.rs`: query all `Media` entities, look up each media's associated light by `light_index` to get its `current_intensity`, call `compute_media_height()`, write back `current_height`
- [ ] 2.2 Call `self.update_media()` from `tick()` after `update_lights()`
- [ ] 2.3 Add integration test: construct SimWorld with media linked to a smooth light, tick multiple times, verify media `current_height` tracks the light intensity interpolation between `height_low` and `height_high`

## 3. Platforms System

- [ ] 3.1 Add `update_platforms()` private method on `SimWorld` in `tick.rs`: query all `Platform` entities, call `tick_platform()` for each, write `current_floor` and `current_ceiling` back to `MapGeometry::floor_heights` and `MapGeometry::ceiling_heights` for the platform's `polygon_index`
- [ ] 3.2 Add player-entry activation check: after ticking platforms, compare player's `PolygonIndex` against each platform's `polygon_index`, call `should_activate()` with `PlatformTrigger::PlayerEntry`, call `activate_platform()` if true
- [ ] 3.3 Add crush check: for entities on a platform's polygon, call `check_platform_crush()` and apply damage or reverse the platform state accordingly
- [ ] 3.4 Call `self.update_platforms()` from `tick()` after `update_media()` and before `run_player_physics()`
- [ ] 3.5 Add integration test: activate a platform, tick until extended, verify `MapGeometry::floor_heights[polygon_index]` equals `floor_extended`
- [ ] 3.6 Add integration test: place player on a player-entry-activated platform polygon, tick once, verify platform state transitions to `Extending`

## 4. Player Physics (already implemented -- extend for media interaction)

- [ ] 4.1 In `run_player_physics()`, after computing collision, check if the player's polygon has associated media with `current_height` above the player's Z position; if submerged, apply `media_drag_factor()` to velocity and decrement oxygen
- [ ] 4.2 If submerged in a damaging media type (`media_deals_damage()`), apply environmental damage to the player via `apply_damage()` each tick
- [ ] 4.3 If oxygen reaches zero while submerged, apply drowning damage each tick
- [ ] 4.4 If player is above media surface, recharge oxygen toward maximum
- [ ] 4.5 Add integration test: place player in water polygon, tick, verify oxygen decreases and velocity is scaled by drag factor

## 5. Monsters System

- [ ] 5.1 Add `update_monsters()` private method on `SimWorld` in `tick.rs`: query all `Monster` entities with `MonsterState`, `Position`, `Facing`, `Velocity`, `Health`, `AttackCooldown`, `PolygonIndex`, and optional `Flying` component
- [ ] 5.2 For each non-Dead monster: get player position, call `can_see_target()` with the monster's visual range and arc (from `PhysicsTables`), compute target distance and range flags, call `next_state()` to determine the new state, write back `MonsterState`
- [ ] 5.3 For monsters transitioning from `Idle` to `Alerted`: collect cascade data (position, class, enemies) and call `find_cascade_targets()` after the main loop; apply `Alerted` state to cascade targets
- [ ] 5.4 For monsters in `Moving` state: compute direction to target, apply movement at the monster definition's speed, update `Position` and `PolygonIndex`; for flying monsters use `compute_flying_movement()`; for ground monsters apply `apply_monster_gravity()`
- [ ] 5.5 For monsters in `Attacking` state: decrement `AttackCooldown`; when zero, call `compute_monster_attack()` and handle `AttackResult::Melee` (apply damage to target via `calculate_damage` + `apply_damage`) or `AttackResult::Ranged` (spawn projectile entity); set `AttackCooldown` to attack_frequency
- [ ] 5.6 Despawn monsters whose state is `Dead` (or keep as corpse depending on rendering needs -- initially keep for corpse rendering)
- [ ] 5.7 Call `self.update_monsters()` from `tick()` after `run_player_physics()`
- [ ] 5.8 Add integration test: construct SimWorld with idle monster facing player within visual range, tick once, verify monster state is `Alerted`
- [ ] 5.9 Add integration test: construct SimWorld with monster in Attacking state at melee range, tick once, verify player health decreased

## 6. Projectiles System

- [ ] 6.1 Add `update_projectiles()` private method on `SimWorld` in `tick.rs`: query all `Projectile` entities with `Position`, `Velocity`, and optional `ProjectileSource`
- [ ] 6.2 For each projectile: call `advance_projectile()` to update position and accumulate `distance_traveled`; check `check_range_limit()` and mark for despawn if exceeded
- [ ] 6.3 Look up projectile definition from `PhysicsTables`; if gravity-affected, call `apply_projectile_gravity()` on velocity; if homing, find nearest valid target and call `apply_homing()`
- [ ] 6.4 Call `check_projectile_wall_collision()` using the projectile's `PolygonIndex` and `MapGeometry`; on wall hit, mark for despawn and record detonation point
- [ ] 6.5 Call `check_projectile_entity_collision()` against monsters (and player for monster-fired projectiles); on entity hit, call `calculate_damage()` and `apply_damage()` on the target, mark projectile for despawn
- [ ] 6.6 For AoE projectiles on detonation: iterate nearby entities, call `calculate_aoe_damage()` for each within radius, apply damage
- [ ] 6.7 After query loop: despawn marked projectiles; for detonations with an effect definition, spawn `Effect` entities at the hit point
- [ ] 6.8 Call `self.update_projectiles()` from `tick()` after `update_monsters()`
- [ ] 6.9 Add integration test: spawn a projectile entity moving toward a wall, tick until collision, verify projectile is despawned and an effect entity exists at the hit point
- [ ] 6.10 Add integration test: spawn a projectile moving toward a monster, tick, verify monster health decreased and projectile is despawned

## 7. Effects System

- [ ] 7.1 Add `update_effects()` private method on `SimWorld` in `tick.rs`: query all `Effect` entities, decrement `ticks_remaining`, collect entities where `ticks_remaining` reaches zero
- [ ] 7.2 After query loop: despawn collected expired effect entities
- [ ] 7.3 Call `self.update_effects()` from `tick()` after `update_projectiles()`
- [ ] 7.4 Add integration test: spawn an effect entity with `ticks_remaining` = 3, tick 3 times, verify the effect entity no longer exists

## 8. Items System

- [ ] 8.1 Add `update_items()` private method on `SimWorld` in `tick.rs`: query all `Item` entities with `Position` and `CollisionRadius`, get player position and collision radius
- [ ] 8.2 For each item: check 2D distance between player and item; if overlapping, call `item_effect()` to determine pickup type; apply effect to player (health/shield/oxygen/ammo/weapon/inventory); mark item for despawn
- [ ] 8.3 Skip pickup if the effect cannot be applied (e.g., health at max for health items)
- [ ] 8.4 After query loop: despawn collected picked-up item entities
- [ ] 8.5 Tick any active `ItemRespawnState` timers (if multiplayer respawn tracking is active)
- [ ] 8.6 Call `self.update_items()` from `tick()` after `update_effects()`
- [ ] 8.7 Add integration test: place player overlapping a health item with health below max, tick once, verify health increased and item entity despawned
- [ ] 8.8 Add integration test: place player overlapping a health item with health at max, tick once, verify item still exists

## 9. Weapon Firing (Player Combat)

- [ ] 9.1 Add a `PlayerWeapons` resource (or component) to hold the player's weapon inventory (`Vec<WeaponSlot>`, equipped index); initialize from physics data in `SimWorld::new()`
- [ ] 9.2 Add `run_player_weapons()` private method (or extend `run_player_physics()`): read `FIRE_PRIMARY`/`FIRE_SECONDARY` from action flags, call `tick_weapon()` on the equipped weapon slot
- [ ] 9.3 When `tick_weapon()` returns true: look up the weapon's trigger definition from `PhysicsTables`, spawn a `Projectile` entity at the player's position + weapon offset, with velocity along the player's facing direction at the projectile's defined speed; add `PolygonIndex` matching the player's current polygon
- [ ] 9.4 Handle `CYCLE_WEAPON_FWD` and `CYCLE_WEAPON_BACK` action flags to cycle the equipped weapon index
- [ ] 9.5 Call `self.run_player_weapons()` from `tick()` immediately after `run_player_physics()`
- [ ] 9.6 Add integration test: set up player with a weapon and ammo, tick with `FIRE_PRIMARY`, verify a projectile entity was spawned and ammo decremented

## 10. Final Tick Ordering Assembly

- [ ] 10.1 Verify `tick()` calls all methods in the correct order: `update_lights()`, `update_media()`, `update_platforms()`, `run_player_physics()`, `run_player_weapons()`, `update_monsters()`, `update_projectiles()`, `update_effects()`, `update_items()`, then increment tick counter
- [ ] 10.2 Add integration test: full-loop test with a level containing lights, platforms, monsters, and items; tick 100 times with various action flags; verify no panics and world state is consistent
- [ ] 10.3 Add determinism test: two SimWorlds with same seed and inputs produce identical snapshots after N ticks
