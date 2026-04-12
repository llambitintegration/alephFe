## Why

`SimWorld::tick()` only runs player physics. The seven other simulation systems -- lights, media, platforms, monsters, projectiles, effects, and items -- all have their logic implemented as standalone functions in `world_mechanics/` and `combat/` but are never called from the tick loop. Nothing in the game world moves, animates, or reacts. This is the single highest-impact blocker for a playable single-player experience: without wiring these calls, platforms are inert, lights are frozen, monsters are statues, weapons cannot fire, and projectiles do not exist.

## What Changes

Wire the existing simulation subsystem functions into `SimWorld::tick()` in the correct order matching alephone's `update_world()`:

1. **Update lights** -- call `compute_light_intensity()` for each `Light` entity, write back `current_intensity`
2. **Update media** -- call `compute_media_height()` for each `Media` entity using its associated light's intensity, write back `current_height`; apply media damage and current forces to submerged entities
3. **Update platforms** -- call `tick_platform()` for each `Platform` entity, update `MapGeometry` floor/ceiling heights for the controlled polygon; check `should_activate()` for player entry triggers; process crush checks and trigger events
4. **Update player physics** -- already implemented (`run_player_physics`)
5. **Update monsters** -- for each monster entity: run `can_see_target()`, `next_state()` to advance AI state machine; apply movement via `compute_flying_movement()` or ground pathfinding; run `compute_monster_attack()` and spawn projectiles or apply melee damage; cascade alerts via `find_cascade_targets()`
6. **Update projectiles** -- for each projectile entity: call `advance_projectile()`, `apply_projectile_gravity()`, `apply_homing()`; run `check_projectile_wall_collision()` and `check_projectile_entity_collision()`; on hit, call `calculate_damage()` / `calculate_aoe_damage()` and `apply_damage()`; despawn on hit or range exceeded
7. **Update effects** -- decrement `Effect::ticks_remaining`, despawn when zero
8. **Update items** -- check player-item overlap for pickups via `item_effect()`; tick `ItemRespawnState` timers and respawn items

Each step becomes a private method on `SimWorld` (like the existing `run_player_physics`), called in sequence from `tick()`. No new algorithms or data structures -- this change is purely integration wiring.

## Capabilities

### New Capabilities

None. All subsystem logic already exists.

### Modified Capabilities

- `game-loop` -- `SimWorld::tick()` grows from 1 system call to 8 ordered system calls, fulfilling the "advance simulation by one tick" requirement that all systems execute in the defined order
- `player-physics` -- platform height changes feed into floor/ceiling used by collision; media current forces apply as external velocity; media damage applies to submerged player
- `combat-system` -- weapon `tick_weapon()` called from the tick loop with player action flags; projectile lifecycle (spawn, move, collide, detonate, despawn) driven each tick
- `monster-ai` -- AI state machine advanced each tick; monster movement, attack decisions, and alert cascading all execute
- `world-mechanics` -- lights, media, platforms, and items updated each tick; platform polygon heights written back to `MapGeometry`; item pickup and respawn logic runs

## Impact

- **marathon-sim/src/tick.rs** -- Primary change site. `SimWorld::tick()` expanded; 7 new private methods added alongside `run_player_physics`
- **marathon-sim/src/world.rs** -- May need a deterministic RNG resource (for light flicker, damage rolls) accessible to tick methods
- **MapGeometry** -- Floor/ceiling height arrays become mutable during platform updates (already `pub` fields)
- **No new crates or dependencies** -- all called functions already exist in marathon-sim
- **No API changes** -- `SimWorld::tick(TickInput)` signature unchanged; `entities()` and player query methods unchanged
- **Existing tests unaffected** -- unit tests for each subsystem function remain valid; new integration tests verify the wired-up ordering
