## MODIFIED Requirements

### Requirement: Weapon switching and inventory

The system SHALL maintain the player's weapon inventory as a list of held weapons. At spawn, the player's inventory SHALL contain at least the fists (melee, infinite use) and the magnum pistol, with the magnum equipped with a full primary magazine and a non-zero starting reserve drawn from its `WeaponDefinition`/`TriggerDefinition` ammunition values. `CYCLE_WEAPON_FWD` and `CYCLE_WEAPON_BACK` action flags SHALL cycle through available weapons. Weapon switching SHALL take `ready_ticks` before the new weapon can fire. The system SHALL support dual-wielded weapons (weapon class `twofisted_pistol`).

#### Scenario: Starting loadout

- **WHEN** a new `SimWorld` is created and the player entity is spawned
- **THEN** the weapon inventory SHALL contain the fists and the magnum pistol, the magnum SHALL have a full primary magazine and a positive primary reserve, and `current()` SHALL return a weapon whose `definition_index` resolves to a ranged weapon with `projectile_type >= 0`

#### Scenario: Firing the starting weapon produces a projectile

- **WHEN** the player presses `FIRE_PRIMARY` with the starting loadout and the equipped weapon is ready
- **THEN** the system SHALL spawn a projectile entity and decrement the magnum's primary magazine by 1

#### Scenario: Cycle to next weapon

- **WHEN** `CYCLE_WEAPON_FWD` is set
- **THEN** the system SHALL switch to the next weapon in the inventory and begin the `ready_ticks` transition

#### Scenario: Dual-wielded weapon

- **WHEN** the player holds two pistols (twofisted class)
- **THEN** `FIRE_PRIMARY` SHALL fire the right weapon and `FIRE_SECONDARY` SHALL fire the left weapon, each with independent ammunition tracking
