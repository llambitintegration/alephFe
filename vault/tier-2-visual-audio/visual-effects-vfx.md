---
tags: [tier-2, rendering, effects, sprites, vfx, explosions]
status: research-complete
---

# Visual Effects (VFX)

How Marathon/Alephone handles explosion sprites, projectile trails, muzzle flashes, teleport effects, blood splatter, and other visual effects.

## Original Alephone Implementation

### Effect System Architecture

The effect system in Alephone (`effects.h`, `effects.cpp`) manages short-lived visual effects as map objects. Each effect has:

- A **type** (from ~79 defined effect types)
- A **shape descriptor** (collection + shape index for the sprite)
- **Flags** controlling lifecycle behavior
- An optional **delay** before becoming visible
- A link to the **map object** it controls

### Effect Types (79 total)

The full enum from `effects.h` includes:

**Weapon Detonations:**
- `_effect_rocket_explosion`, `_effect_rocket_contrail`
- `_effect_grenade_explosion`, `_effect_grenade_contrail`
- `_effect_bullet_ricochet`, `_effect_alien_weapon_ricochet`
- `_effect_flamethrower_burst`
- `_effect_compiler_bolt_minor_detonation`, `_effect_compiler_bolt_major_detonation`
- `_effect_hunter_projectile_detonation`
- `_effect_fist_detonation`
- Various fusion bolt, SPNKR, SMG detonations

**Blood/Impact Splashes (per creature type):**
- `_effect_fighter_blood_splash`
- `_effect_player_blood_splash`
- `_effect_civilian_blood_splash`, `_effect_assimilated_civilian_blood_splash`
- `_effect_enforcer_blood_splash`
- `_effect_trooper_blood_splash`
- `_effect_cyborg_blood_splash`
- `_effect_sewage_yeti_blood_splash`

**Media Splashes (per media type, 4 sizes each):**
- `_effect_small_water_splash`, `_effect_medium_water_splash`, `_effect_large_water_splash`, `_effect_large_water_emergence`
- Same pattern for lava, sewage, goo, and Jjaro

**Teleportation:**
- `_effect_teleport_object_in`
- `_effect_teleport_object_out`

**Creature-Specific:**
- `_effect_defender_spark`, `_effect_hummer_spark`, `_effect_juggernaut_spark`
- Various creature-specific weapon effects

**Environmental:**
- `_effect_water_lamp_breaking`, `_effect_lava_lamp_breaking`, `_effect_sewage_lamp_breaking`, `_effect_alien_lamp_breaking`
- `_effect_metallic_clang`

### Effect Definition Structure (14 bytes)

Each effect type has a static definition (`effect_definitions[]` array in `effect_definitions.h`):

```c
struct effect_definition {
    int16 collection;        // Shapes collection for the sprite
    int16 shape;             // Shape index within collection
    fixed sound_pitch;       // _normal_frequency, _higher_frequency, _lower_frequency
    uint16 flags;            // Behavior flags (see below)
    int16 delay;             // Ticks before becoming visible
    int16 delay_sound;       // Sound to play during delay (NONE = -1)
};
```

### Effect Flags

- `_end_when_animation_loops` (0x0001) - Remove effect when its sprite animation completes one cycle
- `_end_when_transfer_animation_loops` (0x0002) - Remove when transfer mode animation completes
- `_sound_only` (0x0004) - No visual; only plays a sound at the position
- `_make_twin_visible` (0x0008) - Make an associated object visible (used for teleport)
- `_media_effect` (0x0010) - Marks as a media detonation effect

Most effects use `_end_when_animation_loops` as their primary flag. Teleport effects use `_make_twin_visible`.

### Effect Lifecycle

**Creation** (`new_effect()`):
1. Find an available effect slot
2. Create a map object at the specified world position using the definition's collection/shape
3. Set the object ownership to `_object_is_effect`
4. If `delay > 0`, make the object invisible and start the delay countdown
5. If `_sound_only` flag, play sound without creating a visible object

**Update** (`update_effects()` - called each tick):
1. For delayed effects: decrement delay counter, make visible when delay reaches 0
2. For visible effects: call `animate_object()` to advance the sprite animation
3. Check termination: if `_end_when_animation_loops` and animation has looped, remove the effect
4. Handle `_make_twin_visible` to reveal paired objects

**Removal** (`remove_effect()`):
1. Remove the associated map object
2. Free the effect slot

### Teleport Effects

Two specialized functions handle teleportation with visual flair:

**`teleport_object_out()`:**
1. Creates a `_effect_teleport_object_out` effect at the object's position
2. Mirrors the object's appearance onto the effect (same collection/shape)
3. Applies `_xfer_fold_out` transfer mode (horizontal stretch + vertical squeeze)
4. Makes the original object invisible
5. Plays teleport sound
6. Stores original object index in effect data for cleanup

**`teleport_object_in()`:**
1. Creates a `_effect_teleport_object_in` effect at the destination
2. Applies `_xfer_fold_in` transfer mode (reverse of fold_out)
3. Checks for duplicate teleport-in effects on the same object
4. Restores object visibility when the effect completes

### Projectile Contrails

Projectiles can spawn contrail effects based on their definition:
- `contrail_effect` - which effect type to spawn
- `ticks_between_contrails` - spawn frequency
- `maximum_contrails` - cap on active contrails

The projectile system spawns a `new_effect()` at the projectile's position every N ticks.

### Muzzle Flash / Firing Light

Weapon firing produces:
1. A **firing light intensity** (`firing_light_intensity` in `WeaponDefinition`) that temporarily increases the polygon's light level
2. The **firing shape** from the weapon's collection (rendered as HUD overlay, not a world effect)
3. A **shell casing** effect (if defined)

Source: `Source_Files/GameWorld/weapons.cpp`, `weapon_definitions.h`

### Rendering Effects on View

Three screen-level render effects exist (from `render.h`):
- `_render_effect_fold_in` - horizontal stretch for teleport arrival
- `_render_effect_fold_out` - vertical squeeze for teleport departure
- `_render_effect_explosion` - screen shake/flash for nearby explosions

## Current State in Rust Rebuild

### What Exists

**marathon-formats** (`/home/llambit/0_repos/alephone-rust/marathon-formats/src/physics.rs`):
- `EffectDefinition` struct fully parsed (14 bytes): `collection`, `shape`, `sound_pitch`, `flags`, `delay`, `delay_sound`
- `ProjectileDefinition` includes `contrail_effect`, `ticks_between_contrails`, `maximum_contrails`, `detonation_effect`, `media_detonation_effect`
- `MonsterDefinition` includes `impact_effect`, `melee_impact_effect`, `contrail_effect`
- `WeaponDefinition` includes `firing_light_intensity`, `firing_intensity_decay_ticks`
- `PhysicsData` aggregates all definitions from WAD

**marathon-integration** (`/home/llambit/0_repos/alephone-rust/marathon-integration/src/sprites/mod.rs`):
- `SpriteEntityType::Effect` variant exists
- `EntitySpriteState` / `SpriteRenderCommand` for all entity types including effects
- `SpriteBridge` tracks entity lifecycle (added/removed between frames)

**marathon-game/sprite_shader.wgsl** (`/home/llambit/0_repos/alephone-rust/marathon-game/src/sprite_shader.wgsl`):
- Billboarded sprite rendering with alpha test and tint multiplier
- Used for all entity sprites including effects

**marathon-sim/combat/projectiles.rs** (`/home/llambit/0_repos/alephone-rust/marathon-sim/src/combat/projectiles.rs`):
- `advance_projectile()`, `apply_projectile_gravity()`, `apply_homing()`
- `WallHitResult` for collision detection
- No contrail spawning yet

### Gaps

1. **No effect entity system** - no `Effect` component or effect lifecycle manager in marathon-sim
2. **No `new_effect()` equivalent** - no function to spawn effects at world positions
3. **No effect animation** - effects need per-tick animation advancement with loop detection
4. **No teleport visual effects** - fold-in/fold-out transfer modes not implemented
5. **No contrail spawning** - projectile system does not emit trail effects
6. **No blood/impact variety** - no per-creature blood splash type selection
7. **No media splash effects** - impacts on liquid surfaces don't spawn splash sprites
8. **No screen effects** - no explosion shake, teleport warp, or damage flash
9. **No firing light** - weapon discharge doesn't temporarily brighten the polygon
10. **No effect flags implementation** - `_end_when_animation_loops`, `_make_twin_visible` etc.

## Implementation Recommendations

### Phase 1: Core Effect System

1. **Effect component** in marathon-sim:
   ```rust
   pub struct Effect {
       pub effect_type: i16,
       pub position: Vec3,
       pub polygon_index: i16,
       pub facing: u16,
       pub shape: ShapeDescriptor,
       pub frame: u16,
       pub delay_remaining: i16,
       pub flags: u16,
       pub linked_object: Option<u32>,
       pub animation_complete: bool,
   }
   ```

2. **Effect spawner**: `spawn_effect(world, position, polygon_index, effect_type, facing)` that:
   - Looks up the `EffectDefinition` from physics data
   - Creates an `Effect` entity with the definition's collection/shape
   - Handles the delay timer
   - Returns an effect handle

3. **Effect update system**: Each tick, iterate effects:
   - Decrement delay, make visible when delay hits 0
   - Advance animation frame
   - Remove when animation loops (if `_end_when_animation_loops`)

### Phase 2: Integration Points

4. **Projectile detonation**: When a projectile hits a wall/entity, spawn `detonation_effect`. When hitting media, spawn `media_detonation_effect`.

5. **Projectile contrails**: Every `ticks_between_contrails` ticks, spawn `contrail_effect` at the projectile's position, up to `maximum_contrails`.

6. **Monster blood**: On damage, spawn the blood splash effect type appropriate for the monster's collection.

7. **Firing light**: On weapon fire, temporarily increase `floor_light` for the player's polygon by `firing_light_intensity`, decaying over `firing_intensity_decay_ticks`.

### Phase 3: Screen Effects

8. **Teleport fold effect**: Implement as a post-process shader pass that stretches/squeezes the framebuffer horizontally/vertically over several frames.

9. **Explosion shake**: Offset the camera view matrix by a decaying random displacement.

10. **Damage flash**: Overlay a colored quad (red for damage, white for shield hit) with decaying alpha.

### Shader Considerations

Effects are rendered as billboarded sprites using the existing `sprite_shader.wgsl`. The key additions:

- **Transfer mode support for sprites**: Fold-in/fold-out need UV distortion in the sprite shader
- **Additive blending**: Some effects (sparks, energy bolts) should use additive blend mode rather than alpha blending
- **Animation frame selection**: The sprite system needs to advance through animation sequences based on the shapes file's `high_level_shape` -> `low_level_shape` -> bitmap chain

## Related Notes

- [[liquid-surface-rendering]] - Media detonation splash effects
- [[glow-transfer-modes]] - Transfer modes including fold-in/fold-out
- [[dynamic-lighting]] - Firing light temporarily modifies polygon lighting
