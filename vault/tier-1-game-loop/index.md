---
tags: [tier-1, index, game-loop]
status: in-progress
---

# Tier 1: Complete Game Loop

Research notes for achieving a playable single-player Marathon experience. These topics cover the remaining gaps between the current Rust simulation and a fully functional game loop.

## Topics

| Topic | Status | Priority | Notes |
|-------|--------|----------|-------|
| [[item-pickup-system]] | Research complete | High | Item effects defined but not wired into tick loop |
| [[weapon-behaviors]] | Research complete | High | Weapon state machine exists but no class-specific behavior |
| [[projectile-physics]] | Research complete | High | Building blocks exist but no tick system |
| [[platform-mechanics]] | Research complete | Medium | State machine complete, not wired into tick loop |
| [[full-screen-effects]] | Research complete | Medium | Nothing implemented; needs post-process pass |

## Current Architecture Summary

The Rust rebuild uses a `bevy_ecs` World driven manually (no Bevy scheduler). The simulation runs at 30 ticks per second, matching Marathon's original tick rate.

### What works now
- Player movement physics (acceleration, deceleration, gravity, collision, wall sliding)
- Map geometry loading and collision
- Monster spawning with AI state machine (idle, alerted, attacking, fleeing, dying)
- Light animation (constant, linear, smooth, flicker)
- Media (liquid) height computation and drag
- Damage calculation with immunities, weaknesses, AOE
- Weapon inventory structure and generic state machine
- Platform state machine (rest/extend/delay/return)
- 3D rendering with textured walls, floors, ceilings, sprites
- Spatial audio engine
- Save/load via snapshot serialization

### Critical gap: tick loop integration

The single biggest gap is that `SimWorld::tick()` only runs player physics. The following systems exist as standalone functions but are **never called** from the tick loop:

1. **Platform ticking** -- `tick_platform()` in `world_mechanics/platforms.rs`
2. **Item pickup** -- `item_effect()` in `world_mechanics/items.rs`
3. **Weapon firing** -- `tick_weapon()` in `combat/weapons.rs`
4. **Projectile advancement** -- `advance_projectile()` in `combat/projectiles.rs`
5. **Monster AI** -- `decide_action()` in `monster/ai.rs`
6. **Light updates** -- `compute_light_intensity()` in `world_mechanics/lights.rs`
7. **Damage application** -- `apply_damage()` in `combat/damage.rs`

Wiring these into `SimWorld::tick()` in the correct order is the highest-priority task for achieving a complete game loop.

### Recommended tick order

```
SimWorld::tick(input):
  1. Process input (already done)
  2. Player physics (already done)
  3. Monster AI decisions
  4. Weapon tick (fire/reload) -> spawn projectiles
  5. Projectile tick (advance, collide, detonate)
  6. Damage resolution (apply queued damage events)
  7. Item pickup check (player proximity to items)
  8. Platform tick (advance movement, check crush)
  9. Light tick (update intensities)
  10. Media tick (update heights)
  11. Cleanup (despawn dead entities, expired effects)
  12. Advance tick counter
```

## Dependency Graph Between Topics

```
   item-pickup-system
        |
        v
   weapon-behaviors  <----  (weapons acquired from item pickups)
        |
        v
   projectile-physics  <---- (weapons spawn projectiles)
        |
        v
   platform-mechanics  <---- (projectiles can activate platforms)
        |
   full-screen-effects  <---- (damage, teleport, powerups trigger faders)
        ^
        |
   (all combat systems feed into visual feedback)
```

## Key Files in Rust Codebase

| File | Relevance |
|------|-----------|
| `/marathon-sim/src/tick.rs` | Main tick loop entry point |
| `/marathon-sim/src/world.rs` | World construction, entity spawning, snapshot |
| `/marathon-sim/src/components.rs` | All ECS components |
| `/marathon-sim/src/player/inventory.rs` | Weapon inventory system |
| `/marathon-sim/src/player/movement.rs` | Player physics |
| `/marathon-sim/src/combat/weapons.rs` | Weapon state machine |
| `/marathon-sim/src/combat/projectiles.rs` | Projectile physics functions |
| `/marathon-sim/src/combat/damage.rs` | Damage calculation |
| `/marathon-sim/src/world_mechanics/items.rs` | Item pickup effects |
| `/marathon-sim/src/world_mechanics/platforms.rs` | Platform state machine |
| `/marathon-sim/src/world_mechanics/lights.rs` | Light computation |
| `/marathon-sim/src/world_mechanics/media.rs` | Media height and drag |
| `/marathon-sim/src/collision.rs` | Spatial queries |
| `/marathon-sim/src/monster/ai.rs` | Monster AI |
| `/marathon-formats/src/physics.rs` | Physics data definitions |
| `/marathon-game/src/render.rs` | Rendering pipeline |
| `/marathon-game/src/shader.wgsl` | Fragment shader |

## Alephone Reference Files

The original C++ source lives at [github.com/Aleph-One-Marathon/alephone](https://github.com/Aleph-One-Marathon/alephone). Key files:

| File | Topic |
|------|-------|
| `Source_Files/GameWorld/items.h` / `items.cpp` | Item definitions and pickup |
| `Source_Files/GameWorld/weapons.cpp` | Weapon state machine |
| `Source_Files/GameWorld/weapon_definitions.h` | Weapon and trigger structs |
| `Source_Files/GameWorld/projectiles.cpp` | Projectile advancement |
| `Source_Files/GameWorld/platforms.cpp` | Platform movement |
| `Source_Files/RenderOther/screen_drawing.h` | Fader/screen effect types |
| `Source_Files/RenderOther/faders.cpp` | Fader implementation |
| `docs/Lua.html` | Lua API (documents all data fields) |

## Related Tiers

- [[../tier-2-visual-audio/index|Tier 2: Visual & Audio]] -- Rendering polish, VFX, liquid animation
- [[../tier-3-content-pipeline/index|Tier 3: Content Pipeline]] -- MML overrides, Lua scripting
- [[../architecture/index|Architecture]] -- Crate structure, ECS patterns
