---
tags: [tier-1, items, pickup, inventory, game-loop]
status: research-complete
---

# Item Pickup System

How Alephone handles item pickups and the current state in the Rust rebuild.

## Original Alephone / Marathon Behavior

### Item Types

Marathon defines items via an enum in `items.h` (Source_Files/GameWorld/items.h). There are approximately 39 item types organized into categories:

| Index | Constant | Category |
|-------|----------|----------|
| 0 | `_i_knife` (Fists) | Weapon |
| 1 | `_i_magnum` (Pistol) | Weapon |
| 2 | `_i_magnum_magazine` | Ammo |
| 3 | `_i_plasma_pistol` (Fusion) | Weapon |
| 4 | `_i_plasma_magazine` | Ammo |
| 5 | `_i_assault_rifle` | Weapon |
| 6 | `_i_assault_rifle_magazine` | Ammo |
| 7 | `_i_assault_grenade_magazine` | Ammo |
| 8 | `_i_missile_launcher` | Weapon |
| 9 | `_i_missile_launcher_magazine` | Ammo |
| 10 | `_i_invisibility_powerup` | Powerup |
| 11 | `_i_invincibility_powerup` | Powerup |
| 12 | `_i_infravision_powerup` | Powerup |
| 13 | `_i_alien_shotgun` (Alien Weapon) | Weapon |
| 14 | `_i_alien_shotgun_magazine` | Ammo |
| 15 | `_i_flamethrower` | Weapon |
| 16 | `_i_flamethrower_canister` | Ammo |
| 17 | `_i_extravision_powerup` | Powerup |
| 18 | `_i_oxygen_powerup` | Consumable |
| 19 | `_i_energy_powerup` (1x Shield, red) | Consumable |
| 20 | `_i_double_energy_powerup` (2x Shield, yellow) | Consumable |
| 21 | `_i_triple_energy_powerup` (3x Shield, purple) | Consumable |
| 22 | `_i_shotgun` | Weapon |
| 23 | `_i_shotgun_magazine` | Ammo |
| 24 | `_i_spht_door_key` (Uplink Chip) | Key/Inventory |
| 25 | `_i_smg` | Weapon |
| 26 | `_i_smg_ammo` | Ammo |
| 27-38 | Various balls | Inventory (MP) |

**Note:** The Rust codebase uses a slightly different numbering convention (see items.rs), with weapons at 0-8, ammo at 10-18, health at 20-21, etc.

### Pickup Mechanics

In `items.cpp`, the item pickup system works as follows:

1. **Proximity Check:** Each tick, the engine checks if the player (or monster, in some cases) overlaps an item entity. The check uses a 2D distance test between the player center and the item position, compared against the player's collision radius (typically ~0.25 WU or 256 internal units).

2. **Eligibility Check:** Before awarding the item, the engine verifies:
   - The player can hold the item (not at max capacity for ammo/shields)
   - The item is on the same polygon or an adjacent passable polygon
   - Height check: the player's Z position overlaps the item's Z range

3. **Item Consumption:** On successful pickup:
   - The item entity is removed from the world (or flagged for removal)
   - The appropriate effect is applied (ammo added, health restored, weapon granted)
   - A pickup sound is played
   - In multiplayer, a respawn timer starts (typically 300 ticks = 10 seconds)

4. **Pickup Radius:** The original engine uses the player's collision radius (~256 world distance units = 0.25 WU) for proximity checks. Items themselves do not have their own radius -- the check is purely `distance(player, item) < player_radius`.

### Shield Restoration Amounts

| Item | Shield Restored | Maximum Cap |
|------|----------------|-------------|
| 1x Shield (red) | 150 | 150 |
| 2x Shield (yellow) | 300 | 300 (or 2x max) |
| 3x Shield (purple) | 450 | 450 (or 3x max) |

The original engine allows overshielding: a 2x canister can push shields above the normal 150 cap to 300, and 3x to 450.

### Health Restoration

| Item | Health Restored |
|------|----------------|
| Minor Health (canister) | ~20 HP |
| Major Health (canister) | ~40 HP |

Health is capped at 150 in Marathon 2/Infinity.

### Powerup Durations

| Powerup | Duration (ticks) | Duration (seconds) |
|---------|------------------|--------------------|
| Invincibility | 1500 | 50 |
| Invisibility | 2100 | 70 |
| Infravision | 1800 | 60 |
| Extravision | 1800 | 60 |

### Oxygen

Oxygen canister restores 600 units. Max oxygen is typically 600 (maps may vary). The player loses 2 oxygen per tick when submerged.

### Inventory Items

Inventory items like the Uplink Chip and "balls" (colored balls used in multiplayer net games like Kill the Man With the Ball) are tracked in a separate inventory counter array. They act as keys or objectives rather than consumables.

### Monster Item Drops

When a monster dies, if its `MonsterDefinition.carrying_item_type >= 0`, it spawns that item at its death location. This is how players acquire the alien weapon (dropped by Enforcers) and certain ammo.

## Current State in Rust Rebuild

### Implemented (marathon-sim)

**Item type constants and effects:** `/marathon-sim/src/world_mechanics/items.rs`

- 39 item type constants defined (`ITEM_FISTS` through `ITEM_GREEN_BALL`)
- `item_effect()` function maps item types to `ItemEffect` enum variants:
  - `AddWeapon` -- grants weapon by definition index
  - `AddAmmo` -- adds ammo to weapon reserves (primary or secondary)
  - `RestoreHealth` -- restores HP with correct amounts (20/40)
  - `RestoreShield` -- restores shield with correct amounts (150/300/450)
  - `RestoreOxygen` -- restores 600 oxygen units
  - `AddInventoryItem` -- adds keys/balls to inventory counter
- `ItemRespawnState` struct with tick-based countdown for multiplayer respawn

**Item entity spawning:** `/marathon-sim/src/world.rs`

- Items from map data are spawned as ECS entities with `Item { item_type }`, `Position`, `CollisionRadius(0.25)`, `PolygonIndex`, `SpriteShape`, `AnimationFrame` components
- Items are correctly enumerated in `SimWorld::entities()` for rendering

**Item component:** `/marathon-sim/src/components.rs`

- `Item { item_type: i16 }` component defined

### Gaps

1. **No actual pickup logic in the tick loop.** The `SimWorld::tick()` method only runs player physics. There is no system that checks player proximity to items and triggers pickup. The `item_effect()` function exists but is never called from the game loop.

2. **No shield cap handling.** The `RestoreShield` effect knows the amount but there is no logic to enforce or allow overshielding (2x/3x cap behavior).

3. **No powerup duration tracking.** Invincibility, invisibility, infravision, and extravision have no timer components or tick systems. There are no ECS components for active powerup state.

4. **No monster item drops.** The monster AI system does not spawn items when monsters die, even though `MonsterDefinition.carrying_item_type` is parsed.

5. **No pickup sound events.** There is no `SimEvent` variant for item pickup sounds.

6. **No item-to-weapon linkage.** The `AddWeapon` effect specifies a `weapon_definition_index`, but there is no code to actually insert a weapon into the player's `WeaponInventory`. The player entity does not even have a `WeaponInventory` component attached.

7. **No height/polygon validation** for pickups -- the proximity check only exists conceptually.

8. **Ammo pickup values differ from original.** The `ITEM_FLAMETHROWER_AMMO`, `ITEM_ALIEN_AMMO`, `ITEM_SMG_AMMO` are defined but have no `item_effect()` match arms, so they return `None`.

## Implementation Recommendations

### Priority 1: Wire up pickup in tick loop

Add an `item_pickup_system` that runs each tick:
1. Query player position, polygon, collision radius
2. For each Item entity in the same or adjacent polygon, check 2D distance
3. If within pickup radius, call `item_effect()` and apply the result
4. Despawn the item entity
5. Emit a `SimEvent::ItemPickedUp` event for sound/HUD feedback

### Priority 2: Player inventory integration

- Attach a `WeaponInventory` component to the player entity at spawn time
- Implement `AddWeapon` by inserting a new `WeaponSlot` into the inventory
- Implement `AddAmmo` by finding the matching weapon and incrementing reserves
- Implement shield overshielding caps per canister tier

### Priority 3: Powerup timers

Add components:
- `Invincibility(u16)` -- ticks remaining
- `Invisibility(u16)` -- ticks remaining  
- `Infravision(u16)` -- ticks remaining
- `Extravision(u16)` -- ticks remaining

Tick each down; remove on expiry. These affect rendering (see [[full-screen-effects]]) and combat (damage immunity).

### Priority 4: Monster drops

In the monster death handling code, check `carrying_item_type` and spawn an item entity at the monster's position.

## Related Notes

- [[weapon-behaviors]] -- Weapons granted by item pickup
- [[full-screen-effects]] -- Visual effects triggered by powerups
- [[platform-mechanics]] -- Platforms can block item access
- [[projectile-physics]] -- Projectile detonation can spawn items (via `becomes_item_on_detonation` flag)

## Sources

- [Alephone GitHub - Source_Files/GameWorld/](https://github.com/Aleph-One-Marathon/alephone/tree/master/Source_Files/GameWorld)
- [Marathon Wiki - Weapons](https://marathongame.fandom.com/wiki/Weapons)
- [Marathon Wiki - Shields](https://marathongame.fandom.com/wiki/Shields)
- [Marathon Wiki - Biobus Chip Enhancements](https://marathongame.fandom.com/wiki/Biobus_Chip_Enhancements)
- [Marrub's Marathon Format Documentation](https://gist.github.com/marrub--/98af41f36e15a277088b220a6a9f4244)
