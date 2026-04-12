---
tags: [tier-1, projectiles, physics, combat, game-loop]
status: research-complete
---

# Projectile Physics

The Alephone projectile system is one of the engine's most complex subsystems, supporting homing, bouncing, gravity, splash damage, splitting, contrails, and media interaction.

## Original Alephone / Marathon Behavior

### ProjectileDefinition Structure

Each projectile type is defined in the physics data (48 bytes per record, parsed from the `PRpx` WAD tag):

```
ProjectileDefinition {
    collection: i16,                 // Shape collection (-1 = invisible)
    shape: i16,                      // Shape index within collection
    detonation_effect: i16,          // Effect to spawn on detonation (-1 = none)
    media_detonation_effect: i16,    // Effect when detonating in liquid
    contrail_effect: i16,            // Trail effect spawned periodically
    ticks_between_contrails: i16,    // Interval between contrail spawns
    maximum_contrails: i16,          // Cap on contrail count
    media_projectile_promotion: i16, // Projectile type to become when entering media
    radius: i16,                     // Collision radius (world distance units)
    area_of_effect: i16,             // Splash damage radius (0 = no splash)
    damage: DamageDefinition,        // Direct hit damage
    flags: u32,                      // Behavior flags (see below)
    speed: i16,                      // Velocity in world distance units per tick
    maximum_range: i16,              // Max travel distance (0 = unlimited)
    sound_pitch: f32,                // Pitch modifier for sounds
    flyby_sound: i16,                // Sound when projectile passes near player
    rebound_sound: i16,              // Sound on bounce
}
```

### Projectile Flags

The `flags` field controls projectile behavior. These are the known flags from `weapon_definitions.h` and community documentation:

| Bit | Flag | Description |
|-----|------|-------------|
| 0 | `_guided` | Homing projectile: tracks toward player's crosshair (rockets) or monster's target |
| 1 | `_stop_when_animation_loops` | Detonates when its animation sequence completes (flame puffs) |
| 2 | `_persistent` | Does not detonate on contact; passes through targets dealing damage (flame stream) |
| 3 | `_alien_projectile` | Affected by vacuum environments differently; visual differences |
| 4 | `_affected_by_gravity` | Subject to downward acceleration each tick (grenades, flame) |
| 5 | `_rebounds_from_floor` | Bounces off floors instead of detonating |
| 6 | `_bleeding` | Leaves a contrail/blood effect |
| 7 | `_usually_pass_transparent_side` | Can pass through transparent walls |
| 8 | `_sometimes_pass_transparent_side` | Random chance of passing through transparent walls |
| 9 | `_double_gravity` | Double gravity acceleration |
| 10 | `_rebounds_from_walls` | Bounces off walls (ricochet) |
| 11 | `_can_toggle_control_panels` | Can activate wall switches on impact |
| 12 | `_positive_vertical_error` | Spawns with upward bias |
| 13 | `_melee_projectile` | Very short range; treated as melee |
| 14 | `_persistent_and_virulent` | Persistent AND leaves lingering damage area |
| 15 | `_becomes_item_on_detonation` | On detonation, spawns an item (dropped weapon) |
| 16 | `_bleeding_projectile` | Another form of contrail emission |
| 17 | `_horizontal_wander` | Projectile randomly wanders horizontally |
| 18 | `_vertical_wander` | Projectile randomly wanders vertically |
| 19 | `_affected_by_half_gravity` | Half gravity acceleration |
| 20 | `_passes_media_boundary` | Can cross liquid surfaces |

### Per-Weapon Projectile Types (Marathon 2/Infinity)

| Weapon | Primary Projectile | Secondary Projectile | Key Flags |
|--------|-------------------|---------------------|-----------|
| Fist | (melee, no projectile) | same | `_melee_projectile` |
| Pistol | Minor bullet | same (dual) | none (instant hit-scan style, fast speed) |
| Fusion Pistol | Fusion bolt | Overcharged bolt (bigger damage) | none |
| Assault Rifle | AR bullet | Grenade | Grenade: `_affected_by_gravity`, `_rebounds_from_floor` |
| Rocket Launcher | Guided rocket | same | `_guided`, large `area_of_effect` |
| Flamethrower | Flame puff | none | `_persistent`, `_affected_by_gravity`, `_stop_when_animation_loops` |
| Alien Weapon | Alien bolt | none | `_alien_projectile` |
| Shotgun | Shotgun pellet | same (dual) | burst_count in trigger (multiple pellets) |
| SMG | Flechette | same (dual) | Can fire underwater |

### Projectile Lifecycle (per tick)

From `projectiles.cpp`, each tick:

1. **Advance position:** `new_pos = pos + velocity`
2. **Apply gravity** (if `_affected_by_gravity`): `velocity.z -= gravity_constant`
   - Double gravity if `_double_gravity`
   - Half gravity if `_affected_by_half_gravity`
3. **Apply homing** (if `_guided`): Adjust velocity direction toward target with `turning_rate`
4. **Apply wander** (if `_horizontal_wander` or `_vertical_wander`): Random perturbation
5. **Check wall collision:** Ray-cast from old to new position through polygon adjacency
   - If solid wall hit and `_rebounds_from_walls`: reflect velocity, play `rebound_sound`
   - If solid wall hit without rebounds: detonate
   - If transparent wall: check `_usually_pass_transparent_side` / `_sometimes_pass_transparent_side`
6. **Check floor/ceiling collision:**
   - If floor hit and `_rebounds_from_floor`: reflect Z velocity, play `rebound_sound`
   - Otherwise: detonate
7. **Check entity collision:** Circle-line intersection in 2D, with Z range check
   - If `_persistent`: deal damage but do NOT detonate, continue traveling
   - If not persistent: detonate on first entity hit
8. **Check media boundary:**
   - If entering liquid and `media_projectile_promotion >= 0`: replace with promoted type
   - If not `_passes_media_boundary`: detonate at surface
9. **Check animation loop** (if `_stop_when_animation_loops`): detonation when animation completes
10. **Check range limit:** If `distance_traveled >= maximum_range` and `maximum_range > 0`: detonate
11. **Spawn contrail** (if `contrail_effect >= 0`): every `ticks_between_contrails` ticks, spawn effect at current position

### Detonation

On detonation:
1. Spawn `detonation_effect` visual at impact point (or `media_detonation_effect` if in liquid)
2. If `area_of_effect > 0`: apply splash damage to all entities within radius, scaled linearly by distance (full damage at center, zero at edge)
3. If `_becomes_item_on_detonation`: spawn an item entity at detonation point
4. If `_can_toggle_control_panels` and hit a panel wall: activate the panel
5. If hit a platform activation surface: may trigger platform (if platform accepts projectile activation)
6. Remove the projectile entity

### Splash Damage Calculation

```
for each entity in radius:
    distance = |entity_pos - detonation_pos|
    if distance < area_of_effect:
        scale = 1.0 - (distance / area_of_effect)
        damage = base_damage * scale
        apply_damage(entity, damage)
```

### Guided Projectile (Rocket) Behavior

- After firing, the rocket tracks the **player's crosshair direction** (not a specific entity)
- The turning rate is limited, so the rocket curves toward the aim point
- If the player moves their crosshair, the rocket adjusts mid-flight
- This makes rocket guidance a skill-based mechanic
- Monsters with guided projectiles track their target entity instead

### Grenade Bounce

AR grenades have `_affected_by_gravity` and `_rebounds_from_floor`:
- They arc downward due to gravity
- On hitting a floor, the Z velocity is negated (with energy loss)
- They detonate after the first bounce off a wall, or after range expiry
- Each bounce plays `rebound_sound`

## Current State in Rust Rebuild

### Implemented

**Projectile advancement:** `/marathon-sim/src/combat/projectiles.rs`
- `advance_projectile()` -- moves projectile by velocity, returns distance traveled
- `apply_projectile_gravity()` -- applies downward Z acceleration
- `apply_homing()` -- turns velocity toward target with max turning rate per tick
- `check_projectile_wall_collision()` -- ray-cast against polygon edges, returns closest wall hit
- `check_projectile_entity_collision()` -- circle-line intersection with Z range check
- `check_range_limit()` -- maximum range detonation

**Damage system:** `/marathon-sim/src/combat/damage.rs`
- `calculate_damage()` -- with immunities, weaknesses, random component, scale
- `calculate_aoe_damage()` -- linear falloff splash damage
- `apply_damage()` -- shield-then-health model

**Projectile components:** `/marathon-sim/src/components.rs`
- `Projectile { definition_index, distance_traveled }` component
- `ProjectileSource(Entity)` for friendly-fire tracking

**Projectile definitions parsing:** `/marathon-formats/src/physics.rs`
- Full `ProjectileDefinition` struct parsed with all fields including flags, area_of_effect, speed, etc.

### Gaps

1. **No projectile tick system.** None of the projectile functions are called from `SimWorld::tick()`. Projectiles exist as entities but are never advanced, checked for collision, or detonated. The building blocks exist but are not wired into the game loop.

2. **No flag-based behavior dispatch.** The `flags` field is parsed from physics data but never inspected. There is no code to:
   - Check `_guided` and call `apply_homing()`
   - Check `_affected_by_gravity` and call `apply_projectile_gravity()`
   - Check `_rebounds_from_floor/walls` and implement reflection
   - Check `_persistent` to skip detonation on entity hit
   - Check `_stop_when_animation_loops` for animation-based detonation

3. **No bouncing/ricochet.** Wall and floor reflection logic does not exist. The wall collision returns a hit point but there is no code to reflect the velocity vector.

4. **No detonation effects.** When a projectile should detonate, there is no code to:
   - Spawn visual effects (`detonation_effect`)
   - Apply splash damage to nearby entities
   - Trigger panel activations or platform activations
   - Remove the projectile entity

5. **No contrail system.** Contrail spawning is not implemented.

6. **No media interaction.** Projectiles do not check for liquid surfaces, media promotion, or media detonation effects.

7. **No projectile spawning.** There is no code to create a projectile entity from a weapon fire event. The weapon system's `FireResult` is produced but never consumed.

8. **No flyby sounds.** The `flyby_sound` field is parsed but there is no system to detect projectile proximity to the player and emit sound events.

9. **Homing targets player crosshair but code uses entity position.** The `apply_homing()` function targets a `Vec3` position, but for guided rockets, the target should be the point the player is looking at (ray-cast from camera), not a specific entity position.

## Implementation Recommendations

### Priority 1: Projectile tick system

Add a `projectile_system` that runs each tick:
```
for each (entity, projectile, position, velocity) in projectile_query:
    let def = physics_tables.projectiles[projectile.definition_index]
    
    // Apply gravity
    if def.flags & GRAVITY != 0:
        velocity = apply_projectile_gravity(velocity, gravity_constant)
    
    // Apply homing
    if def.flags & GUIDED != 0:
        velocity = apply_homing(velocity, position, target, turning_rate)
    
    // Advance
    let (new_pos, dist) = advance_projectile(position, velocity)
    projectile.distance_traveled += dist
    
    // Check collisions
    // ... wall, entity, floor/ceiling, media
    
    // Check range limit
    if check_range_limit(projectile.distance_traveled, def.maximum_range):
        detonate(entity, position)
```

### Priority 2: Detonation system

- Spawn detonation `Effect` entity at impact point
- If `area_of_effect > 0`, query all entities within radius and apply scaled damage
- Emit `SimEvent::SoundTrigger` for detonation sound
- Remove projectile entity

### Priority 3: Bounce/ricochet

For walls: reflect velocity across wall normal.
For floors: negate Z velocity (with optional energy loss factor).
Play `rebound_sound`.

### Priority 4: Persistent projectiles (flame)

For `_persistent` flag: on entity collision, deal damage but do NOT despawn. Continue advancing. Despawn when `_stop_when_animation_loops` triggers or range exceeded.

## Related Notes

- [[weapon-behaviors]] -- Weapons that spawn projectiles
- [[item-pickup-system]] -- `_becomes_item_on_detonation`
- [[platform-mechanics]] -- Projectile-activated platforms
- [[full-screen-effects]] -- Explosion screen shake (not yet in codebase)

## Sources

- [Alephone weapon_definitions.h](https://github.com/Aleph-One-Marathon/alephone/blob/master/Source_Files/GameWorld/weapon_definitions.h)
- [Alephone weapons.cpp](https://github.com/Aleph-One-Marathon/alephone/blob/master/Source_Files/GameWorld/weapons.cpp)
- [Alephone Lua Documentation](https://github.com/Aleph-One-Marathon/alephone/blob/master/docs/Lua.html)
- [Marrub's Marathon Format Documentation](https://gist.github.com/marrub--/98af41f36e15a277088b220a6a9f4244)
- [Alephone Difficulty Changes Wiki](https://github.com/Aleph-One-Marathon/alephone/wiki/Changes-Based-on-Difficulty)
