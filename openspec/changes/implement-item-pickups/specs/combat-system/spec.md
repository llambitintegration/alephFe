## MODIFIED Requirements

### Requirement: Weapon switching and inventory

The system SHALL maintain the player's weapon inventory as a list of held weapons. Weapons MAY be added to the inventory at any time during the tick (not only at initial loadout), including by the item pickup system. `CYCLE_WEAPON_FWD` and `CYCLE_WEAPON_BACK` action flags SHALL cycle through available weapons. Weapon switching SHALL take `ready_ticks` before the new weapon can fire. The system SHALL support dual-wielded weapons (weapon class `twofisted_pistol`).

#### Scenario: Weapon added by pickup is available for cycling

- **WHEN** the player picks up a shotgun during the item pickup step and then presses `CYCLE_WEAPON_FWD`
- **THEN** the weapon cycle SHALL include the shotgun as an available weapon

#### Scenario: Weapon added by pickup can fire in same tick

- **WHEN** the player picks up a weapon, it becomes the only weapon besides fists, and `FIRE_PRIMARY` is set
- **THEN** the weapon system SHALL recognize the new weapon in the inventory (though it may require a weapon switch first)

### Requirement: Ammunition management

The system SHALL track ammunition separately per weapon trigger. Magazines have a capacity of `rounds_per_magazine`. When a magazine is depleted, the system SHALL auto-reload from reserve ammunition over `loading_ticks` + `finish_loading_ticks`. Ammunition reserves MAY be increased at any time during the tick by the item pickup system (via `AddAmmo` effects), not only from initial loadout. The maximum reserve per weapon trigger SHALL be enforced based on `maximum_reserve` from the weapon's physics definition (or a default cap if not specified).

#### Scenario: Ammo added by pickup feeds next reload

- **WHEN** the player's assault rifle magazine is empty, reserves are 0, and the player picks up ITEM_AR_AMMO (adding 52 to reserves)
- **THEN** the weapon system SHALL auto-reload from the newly added reserves

#### Scenario: Reserve cap enforced on ammo pickup

- **WHEN** the player picks up ammo that would push reserves beyond the weapon's maximum reserve capacity
- **THEN** the reserves SHALL be capped at the maximum, and if reserves were already at maximum, the pickup SHALL be skipped

### Requirement: Damage calculation and application

The system SHALL calculate damage amounts using `DamageDefinition`. When the player has an active invincibility powerup (non-zero `PowerupTimers.invincibility`), all incoming damage to the player SHALL be reduced to zero. Immunities and weaknesses still apply to non-player entities as before.

#### Scenario: Invincible player takes no damage

- **WHEN** a projectile hits the player and `PowerupTimers.invincibility > 0`
- **THEN** the player SHALL take zero damage

#### Scenario: Invincibility expired, damage applies normally

- **WHEN** a projectile hits the player and `PowerupTimers.invincibility == 0`
- **THEN** damage SHALL be calculated and applied normally (shield first, then health)
