---
tags: [tier-1, platforms, world-mechanics, game-loop]
status: research-complete
---

# Platform Mechanics

Platforms (elevators, doors, crushers) are a core Marathon mechanic. They move polygon floor/ceiling heights and interact with entities, triggers, and sounds.

## Original Alephone / Marathon Behavior

### Platform Types

In `platforms.h` / `map_constructors.cpp`, platforms have a `type` field that determines their movement behavior:

| Type | Name | Description |
|------|------|-------------|
| 0 | `_platform_is_from_floor` | Floor rises from rest position upward |
| 1 | `_platform_is_from_ceiling` | Ceiling drops from rest position downward |
| 2 | `_platform_is_from_floor_and_ceiling` | Both floor rises and ceiling drops (closing door) |
| 3 | `_platform_is_door` | Special door behavior: ceiling drops down, then returns |
| 4 | `_platform_is_from_floor_to_ceiling` | Floor rises to ceiling height (elevator to upper level) |
| 5 | `_platform_is_from_ceiling_to_floor` | Ceiling drops to floor height |

### Platform Flags (static_flags)

The `static_flags` field is a bitmask controlling behavior:

| Bit | Flag | Description |
|-----|------|-------------|
| 0 | `_platform_is_initially_active` | Starts in motion at level load |
| 1 | `_platform_is_initially_extended` | Starts in extended position |
| 2 | `_platform_deactivates_at_each_level` | Stops after one cycle |
| 3 | `_platform_deactivates_at_initial_level` | Stops after returning to rest |
| 4 | `_platform_activates_adjacent_platforms_when_activating` | Triggers linked platforms |
| 5 | `_platform_extends_floor_to_ceiling` | Floor rises all the way to ceiling |
| 6 | `_platform_comes_from_floor` | Floor moves (instead of or in addition to ceiling) |
| 7 | `_platform_comes_from_ceiling` | Ceiling moves |
| 8 | `_platform_causes_damage` | Entity crushing deals damage |
| 9 | `_platform_does_not_activate_parent` | Does not re-activate parent platform |
| 10 | `_platform_activates_only_once` | Single use: does not re-activate |
| 11 | `_platform_activates_light` | Toggles associated light on activation |
| 12 | `_platform_deactivates_light` | Toggles light off on deactivation |
| 13 | `_platform_is_player_controllable` | Player can activate via action key |
| 14 | `_platform_is_monster_controllable` | Monsters can trigger |
| 15 | `_platform_reverses_direction_when_obstructed` | Reverses instead of crushing |
| 16 | `_platform_cannot_be_externally_deactivated` | Cannot be stopped by other triggers |
| 17 | `_platform_uses_native_polygon_heights` | Uses polygon's floor/ceiling as limits |
| 18 | `_platform_delays_before_activation` | Waits before starting to move |
| 19 | `_platform_activates_adjacent_platforms_when_deactivating` | Triggers linked on return |
| 20 | `_platform_contracts_slower` | Slower return speed |
| 21 | `_platform_activates_adjacent_platforms_at_each_level` | Triggers linked at each stop |
| 22 | `_platform_is_locked` | Requires key/chip to activate |
| 23 | `_platform_is_secret` | Not shown on automap |
| 24 | `_platform_is_door` | Behaves as a door (quick open/close cycle) |

### Activation Triggers

Platforms can be activated by multiple trigger types:

1. **Player entry** (`_platform_is_player_controllable` + `_platform_activates_on_entry`): When a player enters the platform's polygon, the platform activates. This is the most common trigger for elevators.

2. **Action key** (`_platform_is_player_controllable`): Player presses the action key while on or near the platform polygon.

3. **Monster entry** (`_platform_is_monster_controllable`): Monsters stepping on the polygon trigger it.

4. **Projectile impact** (`_platform_activates_on_projectile`): Certain projectiles (with `_can_toggle_control_panels`) hitting the platform's polygon or adjacent walls activate it.

5. **Adjacent platform trigger** (`_platform_activates_adjacent_platforms`): When one platform reaches its destination, it triggers linked platforms.

6. **Control panel** (switch on a wall): A side texture marked as a platform switch activates the linked platform.

### Platform State Machine (Original)

```
         activate()
AtRest ──────────► Extending
  ▲                    │
  │                    │ (reaches extended position)
  │                    ▼
  │              AtExtended
  │                    │
  │                    │ (delay_remaining counts down)
  │                    ▼
  └──── Returning ◄────┘
         (reaches rest)
```

Key timing:
- **Speed:** Configured per-platform in world distance units per tick
- **Delay:** Ticks to wait at extended position before returning (0 = instant return, i.e., a door that opens and immediately closes)
- **Contract speed:** Can be slower than extend speed with `_platform_contracts_slower`

### Crushing

When a platform's floor and ceiling close to less than an entity's height:

1. If `_platform_causes_damage` (bit 8 / flag `0x2000` in the Rust code's interpretation): apply crush damage (10 HP per tick) to the entity
2. If `_platform_reverses_direction_when_obstructed` (bit 15): the platform reverses direction
3. If neither: the entity is simply blocked/stuck

### Sound Effects

Platforms have associated sounds:
- **Start sound:** When platform begins moving
- **Moving sound:** Looping sound while in motion
- **Stop sound:** When platform reaches destination
- **Obstructed sound:** When reversing due to obstruction

### Door Behavior

Doors (`_platform_is_door`) have special behavior:
- Typically ceiling-only movement (drops ceiling to floor level, then raises back)
- Short delay at extended position
- Often activated by player proximity (monster-controllable too for AI pathing)
- Adjacent platforms can be linked so opening one door opens another

### Height Limits

The platform's minimum and maximum heights define the range:
- `minimum_height`: The "rest" position (usually the polygon's floor height)
- `maximum_height`: The "extended" position
- For ceiling platforms: rest = ceiling height, extended = lower position
- For floor-and-ceiling: both move toward each other

## Current State in Rust Rebuild

### Implemented

**Platform component and state machine:** `/marathon-sim/src/components.rs`
- `Platform` component with all fields: `polygon_index`, `floor_rest`, `floor_extended`, `ceiling_rest`, `ceiling_extended`, `current_floor`, `current_ceiling`, `speed`, `state`, `return_delay`, `delay_remaining`, `activation_flags`, `crushes`
- `PlatformState` enum: `AtRest`, `Extending`, `AtExtended`, `Returning`

**Platform tick system:** `/marathon-sim/src/world_mechanics/platforms.rs`
- `tick_platform()` -- advances platform state machine each tick:
  - `move_toward()` helper for smooth floor/ceiling interpolation
  - Extends until reaching target, then transitions to `AtExtended` with delay
  - Counts down delay, then transitions to `Returning`
  - Returns until reaching rest, then transitions to `AtRest`
- `activate_platform()` -- triggers a platform from `AtRest` to `Extending`
- Activation trigger constants: `PLATFORM_ACTIVATE_ON_PLAYER_ENTRY`, `PLATFORM_ACTIVATE_ON_ACTION_KEY`, `PLATFORM_ACTIVATE_ON_MONSTER_ENTRY`, `PLATFORM_ACTIVATE_ON_PROJECTILE`
- `should_activate()` -- checks if a trigger type matches the platform's activation flags
- `PlatformTrigger` enum: `PlayerEntry`, `ActionKey`, `MonsterEntry`, `ProjectileImpact`

**Crush detection:** `/marathon-sim/src/world_mechanics/platforms.rs`
- `check_platform_crush()` -- returns `PlatformCrushResult`: `None`, `Crush { damage: 10 }`, or `Reverse`
- Checks if clearance < entity height

**Platform trigger events:** 
- `check_platform_triggers()` -- returns events when platform reaches extended or rest position
- `PlatformTriggerEvent` with `ActivatePlatform` and `ToggleLight` types

**Platform spawning:** `/marathon-sim/src/world.rs`
- Platforms parsed from map data and spawned as ECS entities
- `speed`, `minimum_height`/`maximum_height` converted to f32
- Crush flag derived from `static_flags & 0x2000`

### Gaps

1. **Not wired into tick loop.** `tick_platform()` exists but is never called from `SimWorld::tick()`. Platforms are static after spawning.

2. **No player entry detection.** There is no system to check if a player (or monster) has entered a platform's polygon and trigger activation. The `should_activate()` function exists but nothing calls it.

3. **No MapGeometry update on platform movement.** When a platform moves, the floor/ceiling heights in `MapGeometry` must be updated so collision and rendering reflect the new heights. This is not implemented.

4. **No platform type handling.** The `Platform` component stores floor and ceiling rest/extended heights, but there is no logic differentiating between floor-only, ceiling-only, and combined platforms. The `spawn_platforms()` function defaults ceiling to 0.0 for all platforms.

5. **No linked platform chain.** `check_platform_triggers()` returns events for linked platforms but no code processes these events to actually activate the linked targets.

6. **No sound events.** Platform movement, arrival, and crush events do not emit `SimEvent::SoundTrigger`.

7. **No door-specific behavior.** Doors are not specially handled. The state machine is generic, which mostly works, but door-specific features (monster pathing interaction, automap visibility) are missing.

8. **No platform lock/key system.** `_platform_is_locked` is not checked; there is no inventory check for keys/uplink chips.

9. **No polygon height writeback.** The platform moves `current_floor` and `current_ceiling` but these values are not propagated to the `MapGeometry` resource, so other systems (collision, rendering) do not see the change.

10. **No contract speed modifier.** `_platform_contracts_slower` flag handling is missing.

11. **Activation flags mapping is incomplete.** The Rust code maps `static_flags` directly as activation flags, but the original engine uses different flag bits for activation vs. behavior. The bit values in the Rust constants (`0x0001`, `0x0004`, `0x0010`, `0x0040`) may not match the original engine's actual activation flag bits.

## Implementation Recommendations

### Priority 1: Wire platforms into tick loop

Add a `platform_system` that runs each tick:
1. Tick all platform entities with `tick_platform()`
2. Write back `current_floor` / `current_ceiling` to `MapGeometry.floor_heights[polygon_index]` / `ceiling_heights[polygon_index]`
3. Check for entity crushing

### Priority 2: Player activation

Each tick, for each platform:
- If player's `PolygonIndex` matches `platform.polygon_index` and `should_activate(platform, PlayerEntry)`: activate
- If `ActionFlags::ACTION` pressed and player is on/near the polygon: activate

### Priority 3: Platform type differentiation

Implement proper floor/ceiling target computation based on platform type:
- Floor-only: only `current_floor` moves
- Ceiling-only: only `current_ceiling` moves
- Both: both move
- Door: ceiling drops, then returns

### Priority 4: Linked platform chains

Process `PlatformTriggerEvent` outputs:
- For `ActivatePlatform`: find the platform entity with matching target index and call `activate_platform()`
- For `ToggleLight`: toggle the corresponding light entity

## Related Notes

- [[item-pickup-system]] -- Items on platforms move with them
- [[projectile-physics]] -- Projectiles can activate platforms
- [[weapon-behaviors]] -- Action key activation context
- [[full-screen-effects]] -- Crush damage flash

## Sources

- [Alephone GitHub - Source_Files/GameWorld/](https://github.com/Aleph-One-Marathon/alephone/tree/master/Source_Files/GameWorld)
- [Alephone Lua API - Platform properties](https://github.com/Aleph-One-Marathon/alephone/blob/master/docs/Lua.html)
- [Alephone GitHub Issue #220 - M1 activation differences](https://github.com/Aleph-One-Marathon/alephone/issues/220)
- [Marathon SDA Knowledge Base](https://kb.speeddemosarchive.com/Marathon)
- [Marrub's Marathon Format Documentation](https://gist.github.com/marrub--/98af41f36e15a277088b220a6a9f4244)
