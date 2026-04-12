---
tags: [alephone, reference, mechanics, physics, weapons, monsters]
---

# Game Mechanics Reference

This document covers the canonical Marathon game mechanics as defined by the physics model and engine code. These are the values and behaviors the Rust rebuild must replicate for authentic game feel.

## Simulation Tick Rate

Marathon runs at a fixed **30 ticks per second** (33.333 ms per tick). All physics values are expressed in per-tick units. This is hardcoded in the engine and all physics values depend on it.

## Player Movement Physics

Marathon has two physics models: **walking** and **running**. By default, the player uses the running model. The physics constants are stored as 16.16 fixed-point values in the physics data.

### Running Physics (Default, index 1)

| Parameter | Value (fixed) | Value (f32) | Description |
|-----------|--------------|-------------|-------------|
| max_forward_velocity | 0x2000 | 0.125 WU/tick | ~3.75 WU/s forward |
| max_backward_velocity | 0x1555 | 0.083 WU/tick | ~2.5 WU/s backward |
| max_perpendicular_velocity | 0x1333 | 0.077 WU/tick | ~2.3 WU/s strafe |
| acceleration | 0x00A8 | 0.01 WU/tick^2 | |
| deceleration | 0x0148 | 0.02 WU/tick^2 | |
| airborne_deceleration | 0x005B | ~0.0056 WU/tick^2 | |
| gravitational_acceleration | 0x0028 | 0.0025 WU/tick^2 | |
| terminal_velocity | 0x2480 | 0.143 WU/tick | |
| angular_acceleration | 5/4 | 1.25 angle units/tick^2 | ~0.0153 rad/tick^2 |
| angular_deceleration | 5/2 | 2.5 angle units/tick^2 | ~0.0307 rad/tick^2 |
| max_angular_velocity | 10 | 10 angle units/tick | ~0.123 rad/tick |
| maximum_elevation | 128/3 | ~42.67 angle units | ~30 degrees |
| step_delta | 0x0333 | 0.05 WU | Max step-up height |
| radius | 0x100 | 0.25 WU | Player collision radius |
| height | 0x0CCD | 0.8 WU | Player collision height |
| camera_height | 0x0CCD | 0.2 WU | Camera offset above feet |

### Walking Physics (index 0)

Similar but slower. Forward velocity is approximately 60% of running.

### Movement Model

Marathon uses an **axis-decomposed** velocity model:
- Forward/backward velocity is tracked independently from strafe velocity
- Stopping strafe does NOT affect forward speed, and vice versa
- When input opposes current velocity (direction reversal), both acceleration AND deceleration are applied simultaneously for snappier reversals
- Gravity only applies when not grounded

Angular values in the physics data are in Marathon angle units (512 = full circle). The Rust sim converts these to radians using the factor `TAU / 512 = 0.01227...`.

### Coordinate System

Marathon's 2D map plane is X/Y. Height (Z) is separate. The facing angle convention:
- 0 = east (positive X)
- Increases counterclockwise
- 512 = full circle

## Collision System

Marathon uses a polygon-based BSP approach:
- The map is divided into convex polygons
- Each polygon knows its adjacent polygons (via shared lines)
- Entity movement is tracked per-polygon
- Collision checks test line crossings against the current polygon's edges

### Wall Collision

When movement crosses a line:
1. If the line is solid: slide along the wall (project velocity onto wall parallel)
2. If passable to an adjacent polygon:
   - Check step delta: floor height difference must be <= step_delta (0.05 WU)
   - Check ceiling clearance: adjacent ceiling - floor must be >= player height (0.8 WU)
   - If both pass: allow crossing, step up Z if needed
   - If either fails: treat as solid wall, slide along it

Up to 3 slide iterations are performed per tick to handle corner cases.

### Entity Collision

Entities have circular collision radii. Entity-entity collision uses circle overlap tests in the 2D plane.

### Line of Sight

LOS uses a BFS through the polygon adjacency graph, checking if the ray from source to target crosses any solid line.

## Weapons

Marathon's weapons have complex state machines:

### Weapon States

```
Idle -> Firing -> Recovering -> Idle
         |
         v
      Reloading -> Idle
         |
      Switching -> Idle
```

### Weapon Classes

| Class | Description | Examples |
|-------|-------------|---------|
| Melee | Close combat | Fists |
| Normal | Single trigger | Magnum, Assault Rifle |
| Dual-wield | Two independent weapons | Dual Magnums, Dual Shotguns |
| Twofisted | Alternating dual | -- |
| Multipurpose | Primary + secondary fire | SPNKR (rocket + grenade) |

### Firing Mechanics

- `rounds_per_magazine`: Ammo before reload needed
- `ticks_per_round`: Minimum ticks between shots (fire rate)
- `recovery_ticks`: Ticks after firing before next shot
- Burst fire: Single ammo consumption, multiple projectiles with spread (theta_error)
- Dual-wield: Primary/secondary fire control independent left/right weapons

### Ammo Management

Weapons have primary and secondary magazines, each with current ammo and reserves. Reload transfers rounds from reserve to magazine. Auto-reload triggers when magazine empties and reserves exist.

## Projectiles

### Projectile Types

| Property | Description |
|----------|-------------|
| definition_index | Type of projectile |
| velocity | Speed and direction (per tick) |
| distance_traveled | Accumulated travel for range limit |
| gravity | Optional downward acceleration per tick |
| homing | Optional turn-toward-target per tick |
| maximum_range | Despawn distance (0 = unlimited) |

### Projectile Collision

Each tick:
1. Advance position by velocity
2. Apply gravity (if applicable)
3. Apply homing (if applicable, limited by turning_rate)
4. Check wall collision (segment intersection with polygon lines)
5. Check entity collision (circle-ray intersection with collision radii, Z overlap)
6. Check range limit

## Damage Model

### Damage Calculation

```
raw_damage = (base + random(0..=random_component)) * scale
```

Then:
- If target is **immune** to damage_type: damage = 0
- If target is **weak** to damage_type: damage = raw_damage * 2

### Damage Application

Damage is absorbed by shield first, then health:
```
1. remaining = damage
2. shield_absorbed = min(remaining, current_shield)
3. remaining -= shield_absorbed
4. health -= remaining
5. killed = (health <= 0)
```

### Damage Types

Marathon defines multiple damage types as a bitmask. Monsters have immunity and weakness bitmasks that are checked against the damage type.

### Area of Effect

AOE damage scales linearly with distance:
```
effective_damage = base_damage * (1.0 - distance / aoe_radius)
```
Zero damage at the edge of the radius.

## Monster AI

### AI State Machine

```
    +------+    can see    +---------+
    | Idle |  --------->   | Alerted |
    +------+               +---------+
       ^                    /       \
       |                   v         v
   lost target        +--------+  +--------+
       |               | Moving |  |Attacking|
       +------<--------+--------+  +--------+
                            ^          |
                            +----------+
                          out of range

    Any State ---[vitality=0]--> Dying --> Dead
```

### Vision

Monsters detect targets based on:
- **visual_range**: Maximum sight distance
- **half_visual_arc**: Horizontal FOV half-angle
- Target must be within arc AND range AND LOS (no solid walls blocking)

### Activation Cascading

When a monster becomes alerted, nearby monsters of the same class (within cascade_radius, same enemies bitmask) also become alerted. This creates realistic group alerting behavior.

### Attack Resolution

Each tick in the Attacking state:
1. Check attack_cooldown (must be 0 to attack)
2. If distance <= melee_range: execute melee attack (direct damage)
3. If distance <= ranged_range: execute ranged attack (spawn projectile)
4. Reset cooldown timer

### Flying Monsters

Flying monsters have a `preferred_hover_height` and move in full 3D toward targets. Non-flying monsters are subject to gravity.

### Target Redirection (Friendly Fire)

If a monster is hit by a friendly entity (in its friends bitmask), it may redirect its target to the attacker.

## Difficulty Scaling

Enemy parameters scale with difficulty level:

| Difficulty | Speed Scale | Attack Delay Scale |
|------------|-------------|-------------------|
| Kindergarten | 88% | 300% |
| Easy | 94% | 200% |
| Normal | 100% | 100% |
| Major Damage | 113% | 50% |
| Total Carnage | 125% | 25% |

## World Mechanics

### Platforms (Elevators/Doors)

State machine: AtRest -> Extending -> AtExtended (delay) -> Returning -> AtRest

| Trigger Type | Description |
|-------------|-------------|
| Player Entry | Activates when player walks onto platform polygon |
| Action Key | Activates when player presses action near platform |
| Monster Entry | Activates when monster walks onto platform |
| Projectile Impact | Activates when hit by projectile |

Platforms can **crush** entities (deal 10 damage per tick) or **reverse** when they would crush.

Platforms can trigger other platforms and toggle lights when reaching extended or rest positions.

### Lights

Light functions compute intensity (0.0-1.0) from time:

| Function | Behavior |
|----------|----------|
| Constant | Always at max intensity |
| Linear | Triangle wave (0->max->0 over period) |
| Smooth | Cosine wave (smooth oscillation) |
| Flicker | Random intensity each tick |

Phase offset allows synchronized or staggered light groups.

### Media (Liquids)

| Type | Drag | Deals Damage |
|------|------|-------------|
| Water | 0.5 | No |
| Lava | 0.3 | Yes |
| Goo | 0.2 | Yes |
| Sewage | 0.4 | No |
| Jjaro | 0.3 | Yes |

Media height is driven by an associated light function, interpolating between `height_low` and `height_high`.

When submerged:
- Horizontal velocity reduced by drag factor
- Oxygen depletes at 2 units per tick
- At 0 oxygen: 5 drowning damage per tick
- Above surface: oxygen recharges at 1 unit per tick

### Player Vitals

| Stat | Default | Max |
|------|---------|-----|
| Health | 150 | 150 |
| Shield | 150 | 150 |
| Oxygen | 600 | 600 |

Oxygen 600 at 30 ticks/second = 20 seconds of underwater breath.

## Polygon Types

Polygons have special types that trigger behaviors:

| Type | Effect |
|------|--------|
| Normal | Nothing |
| Platform | Polygon is a moving platform |
| Teleporter | Teleports player to permutation level/polygon |
| MinorOuch | Small periodic damage to player |
| MajorOuch | Large periodic damage to player |
| Glue/Superglue | Restricts player movement |
| Hill/Base | Multiplayer objectives |
| ItemImpassable | Items cannot be dropped here |
| MonsterImpassable | Monsters cannot enter |
| LightOnTrigger/OffTrigger | Toggles lights when player enters |
| PlatformOnTrigger/OffTrigger | Toggles platforms when player enters |
| MustBeExplored | Required for level completion |
| AutomaticExit | Triggers level exit |
