---
tags: [tier-1, weapons, combat, game-loop]
status: research-complete
---

# Weapon Behaviors

All weapon types in Marathon, their fire modes, and comparison with the Rust implementation.

## Original Alephone / Marathon Behavior

### Weapon Class System

Alephone defines weapons in `weapon_definitions.h` (Source_Files/GameWorld/weapon_definitions.h). Each weapon has a `weapon_class` that determines its fundamental behavior:

| Class | Name | Behavior |
|-------|------|----------|
| 0 | `_melee` | Close-range attack, no projectile (fist) |
| 1 | `_normal` | Standard single weapon (fusion, AR, rocket, flamethrower) |
| 2 | `_dual_function` | One weapon, two distinct triggers (AR primary=bullets, secondary=grenades) |
| 3 | `_twofisted_pistol` | Can be dual-wielded; secondary trigger fires left copy (pistol, shotgun, SMG) |
| 4 | `_multipurpose` | Shared ammo, secondary has special behavior (fusion pistol overcharge) |

### Weapon Flags

Each `WeaponDefinition` has a `flags` field:

| Bit | Flag | Description |
|-----|------|-------------|
| 0 | `_weapon_is_automatic` | Fires continuously while trigger held |
| 1 | `_weapon_disappears_after_use` | Alien weapon: removed when ammo depleted |
| 2 | `_weapon_has_random_ammo_on_pickup` | Random amount of ammo on first pickup |
| 3 | `_weapon_fires_out_of_phase` | Dual-wield: alternates left/right automatically |
| 4 | `_weapon_fires_under_media` | Can fire underwater |
| 5 | `_weapon_triggers_share_ammo` | Primary and secondary use same ammo pool |
| 6 | `_weapon_secondary_has_angular_flipping` | Dual-wield: second weapon flips firing angle |
| 7 | `_weapon_can_be_overloaded` | Fusion pistol overcharge mechanic |

### All 9 Weapons

#### 0: Fist (class: melee)
- **Primary:** Punch. Melee range (~0.5 WU). Rapid-fire capable.
- **Secondary:** Brings up second fist for alternating punches.
- **Ammo:** Unlimited.
- **Notes:** No projectile spawned; damage applied directly via melee range check.

#### 1: .44 Magnum Mega Class Pistol (class: twofisted_pistol)
- **Primary:** Single shot, semi-automatic (one shot per trigger press).
- **Secondary:** When dual-wielding, fires left pistol.
- **Ammo type:** Pistol magazines (8 rounds per magazine).
- **Dual wield:** Player can pick up a second pistol and fire both independently. Left = secondary trigger, right = primary.
- **Notes:** Most common early-game weapon. Accurate but low damage.

#### 2: Zeus Class Fusion Pistol (class: multipurpose)
- **Primary:** Standard fusion bolt. Semi-automatic.
- **Secondary:** Overcharge -- hold secondary trigger to charge, release to fire a powerful bolt. Holding too long causes self-damage explosion.
- **Ammo type:** Fusion battery (shared between primary and secondary). 20 units per battery.
- **Overcharge cost:** Consumes multiple ammo units. Charging has a timer; if `charging_ticks` exceeded, weapon overloads.
- **Notes:** Cannot fire underwater. The overcharge mechanic is unique to this weapon.

#### 3: MA-75 Assault Rifle / Grenade Launcher (class: dual_function)
- **Primary:** Automatic fire. 52 rounds per magazine. Very fast rate of fire.
- **Secondary:** Grenade launcher. 7 grenades per magazine. Separate ammo type.
- **Notes:** The workhorse weapon. Grenades are affected by gravity and bounce once before detonating. Cannot fire underwater.

#### 4: SPNKR SSM Launcher (class: normal)
- **Primary:** Fires a guided rocket. 2 rockets per magazine.
- **Secondary:** None (or identical to primary in some versions).
- **Guided mechanic:** After firing, the rocket tracks the player's crosshair. The `_guided` projectile flag makes it home toward the player's aim direction.
- **Splash damage:** Large area of effect.
- **Notes:** Slow rate of fire, slow projectile speed. Cannot fire underwater.

#### 5: TOZT-7 Backpack Napalm Unit (Flamethrower) (class: normal)
- **Primary:** Continuous stream of flame projectiles. 7 seconds of fuel.
- **Secondary:** None.
- **Ammo type:** Napalm canister.
- **Notes:** Short range, area denial. Flame projectiles are affected by gravity. Cannot fire underwater. Fire persists on surfaces briefly.

#### 6: Alien Weapon (class: normal)
- **Primary:** Rapid-fire energy bolts. Horizontal spread.
- **Secondary:** None.
- **Ammo type:** Internal (cannot be reloaded from pickups).
- **Notes:** Obtained by killing Enforcers. Disappears when ammo is depleted (`_weapon_disappears_after_use`). Cannot fire underwater.

#### 7: WSTE-M5 Combat Shotgun (class: twofisted_pistol)
- **Primary:** Fires 2 shells (burst of pellets with spread). 2 shells per magazine.
- **Secondary:** When dual-wielding, fires left shotgun.
- **Dual wield:** Can dual-wield two shotguns.
- **Notes:** Devastating close range. Each "shot" fires a burst of multiple projectiles (`burst_count` in `TriggerDefinition`). Very slow reload.

#### 8: KKV-7 10mm SMG Flechette (class: twofisted_pistol, Marathon Infinity only)
- **Primary:** Automatic fire. Fast rate, moderate damage.
- **Secondary:** When dual-wielding, fires left SMG.
- **Dual wield:** Can dual-wield two SMGs.
- **Notes:** Can fire underwater (`_weapon_fires_under_media`). Added in Marathon Infinity.

### TriggerDefinition Structure

Each weapon has primary and secondary `TriggerDefinition`:

```
TriggerDefinition {
    rounds_per_magazine: i16,     // Ammo capacity per reload
    ammunition_type: i16,         // Item type for ammo pickups (-1 = no external ammo)
    ticks_per_round: i16,         // Ticks between shots (fire rate)
    recovery_ticks: i16,          // Recovery after firing before next shot
    charging_ticks: i16,          // Max charge time (fusion overcharge)
    recoil_magnitude: i16,        // Visual kick
    firing_sound: i16,            // Sound index
    click_sound: i16,             // Empty magazine sound
    charging_sound: i16,          // Fusion charge sound
    shell_casing_sound: i16,      
    reloading_sound: i16,         
    charged_sound: i16,           // Fusion fully charged sound
    projectile_type: i16,         // Index into projectile definitions
    theta_error: i16,             // Accuracy spread (larger = less accurate)
    dx: i16,                      // Horizontal offset of projectile spawn
    dz: i16,                      // Vertical offset of projectile spawn
    shell_casing_type: i16,       
    burst_count: i16,             // Number of projectiles per shot (shotgun)
}
```

### Weapon State Machine (Original)

Alephone's `weapons.cpp` drives weapon state through these states:
1. **Idle** -- ready to fire
2. **Raising** -- weapon coming up after switch
3. **Lowering** -- weapon going down for switch
4. **Firing** -- firing animation playing, projectile(s) spawned
5. **Recovering** -- cooldown between shots
6. **Reloading** -- magazine reload animation
7. **Charging** -- fusion pistol charge (secondary trigger held)
8. **Overloaded** -- fusion pistol about to explode

The weapon display is driven by shape descriptors (`idle_shape`, `firing_shape`, `reloading_shape`, `charging_shape`, `charged_shape`) that control the first-person weapon sprite.

## Current State in Rust Rebuild

### Implemented

**Weapon inventory:** `/marathon-sim/src/player/inventory.rs`
- `WeaponInventory` with slot-based storage, current weapon tracking, switch cooldown
- `WeaponSlot` with primary/secondary magazine and reserve ammo
- `WeaponState` enum: `Idle`, `Firing`, `Recovering`, `Reloading`, `Switching`
- `cycle_forward()` / `cycle_backward()` for weapon switching
- `consume_primary()`, `reload_primary()`, `needs_primary_reload()`

**Weapon tick system:** `/marathon-sim/src/combat/weapons.rs`
- `tick_weapon()` -- single weapon state machine (Idle -> Firing -> Recovering -> Idle)
- `tick_weapon_burst()` -- burst fire support, returns `FireResult` with projectile count and spread
- `DualWieldState` -- independent tick of left and right weapons
- Cooldown management and auto-reload trigger

**Weapon definitions parsing:** `/marathon-formats/src/physics.rs`
- Full `WeaponDefinition` struct with all fields parsed
- `TriggerDefinition` with `rounds_per_magazine`, `ticks_per_round`, `recovery_ticks`, `charging_ticks`, `burst_count`, `theta_error`, etc.

### Gaps

1. **No weapon-specific behaviors.** All weapons use the same generic `tick_weapon()` logic. There is no differentiation by weapon class (melee, normal, dual_function, twofisted_pistol, multipurpose).

2. **No melee system.** Fists have no special handling. There is no melee range check or direct damage application. The current system only supports projectile-spawning weapons.

3. **No fusion overcharge.** The `charging_ticks` field is parsed but never used. There is no charging state, no overload timer, no self-damage explosion.

4. **No weapon flags handling.** The `flags` field in `WeaponDefinition` is parsed but never inspected. No support for:
   - `_weapon_is_automatic` (hold to fire)
   - `_weapon_disappears_after_use` (alien weapon removal)
   - `_weapon_fires_under_media` (underwater check)
   - `_weapon_can_be_overloaded` (fusion)
   - `_weapon_triggers_share_ammo`

5. **No secondary trigger as separate fire mode.** The `WeaponSlot` has `secondary_magazine` and `secondary_reserve` fields but `tick_weapon()` only uses primary. The AR grenade launcher secondary is not implemented.

6. **No weapon display sprites.** The weapon state does not drive first-person weapon rendering. There are no `idle_shape`, `firing_shape`, etc. lookups in the render loop.

7. **No weapon switch animation timing.** `ready_ticks`, `await_reload_ticks`, `loading_ticks`, `finish_loading_ticks` from `WeaponDefinition` are parsed but unused.

8. **Player entity has no WeaponInventory.** The `WeaponInventory` struct exists but is never attached to the player ECS entity or initialized from physics data.

9. **No projectile spawning from weapon fire.** When `tick_weapon()` returns `fired=true`, no code actually spawns a projectile entity. The weapon system and projectile system are not connected.

## Implementation Recommendations

### Priority 1: Wire weapon inventory to player

- Add a `WeaponInventory` as a player ECS component
- Initialize from physics data: give player starting weapons per level config
- Connect `ActionFlags::FIRE_PRIMARY` / `FIRE_SECONDARY` to weapon tick
- On fire, spawn a projectile entity at the weapon's `dx`/`dz` offset from player position

### Priority 2: Weapon class dispatch

Implement weapon class-specific behavior:
- **Melee:** Direct damage raycast at `melee_range`, no projectile
- **Normal:** Standard projectile spawn
- **Dual function:** Route primary to primary trigger, secondary to secondary trigger (different projectile types)
- **Twofisted pistol:** Manage dual weapon state, secondary fires left weapon
- **Multipurpose:** Shared ammo, secondary uses charging mechanic

### Priority 3: Fusion overcharge

- Track `charging_ticks` elapsed in weapon state
- If `charging_ticks` exceeded, trigger overload self-damage
- Consume increasing ammo during charge

### Priority 4: Weapon flags

Implement flag checks in the weapon tick:
- Automatic fire: keep firing while `FIRE_PRIMARY` held
- Disappear after use: despawn weapon from inventory when ammo depleted
- Underwater: prevent firing based on player submersion state

## Related Notes

- [[item-pickup-system]] -- How weapons are acquired
- [[projectile-physics]] -- What happens after a weapon fires
- [[full-screen-effects]] -- Weapon firing light flash

## Sources

- [Alephone weapon_definitions.h](https://github.com/Aleph-One-Marathon/alephone/blob/master/Source_Files/GameWorld/weapon_definitions.h)
- [Alephone weapons.cpp](https://github.com/Aleph-One-Marathon/alephone/blob/master/Source_Files/GameWorld/weapons.cpp)
- [Marathon Wiki - Weapons](https://marathongame.fandom.com/wiki/Weapons)
- [Marathon Weapons at bungie.org](https://marathon.bungie.org/story/weapons.html)
- [CyberAcme Marathon Wiki - Trilogy Weapons](https://www.marathonwiki.com/Category:Trilogy_weapons)
