## ADDED Requirements

### Requirement: Detect item proximity and trigger pickup

The system SHALL check all Item entities against the player's position each tick. An item is eligible for pickup when: (a) the 2D (XY-plane) Euclidean distance between the player position and the item position is less than the pickup radius (1.0 WU), and (b) the item occupies the same polygon as the player or a polygon adjacent via a non-solid line. When an eligible item is found and its effect is not wasted, the system SHALL apply the effect, despawn the item entity, and emit a `SimEvent::ItemPickedUp` event.

#### Scenario: Player walks over a health canister

- **WHEN** the player's position is within 1.0 WU of a minor health item AND the player's health is below 150 AND both are in the same polygon
- **THEN** the player's health SHALL increase by 20 (capped at 150), the item entity SHALL be despawned, and an `ItemPickedUp` event SHALL be emitted

#### Scenario: Item in adjacent polygon

- **WHEN** the player is in polygon A and an item is in polygon B, where B is adjacent to A via a non-solid line, and the 2D distance is within 1.0 WU
- **THEN** the item SHALL be eligible for pickup

#### Scenario: Item across a solid wall

- **WHEN** the player is in polygon A and an item is in polygon B, where B is adjacent to A only via a solid line (or not adjacent at all), even if the 2D distance is within 1.0 WU
- **THEN** the item SHALL NOT be picked up

#### Scenario: Item too far away

- **WHEN** the 2D distance between the player and an item is greater than or equal to 1.0 WU
- **THEN** the item SHALL NOT be picked up regardless of polygon adjacency

### Requirement: Apply weapon pickup effects

The system SHALL grant a weapon to the player's `WeaponInventory` when an `AddWeapon` effect is resolved. The weapon SHALL be inserted into the inventory at the slot corresponding to its `weapon_definition_index`. If the player already holds a weapon in that slot, the pickup SHALL be skipped and the item SHALL remain in the world.

#### Scenario: Pick up pistol (item type 1)

- **WHEN** the player picks up ITEM_PISTOL and does not already have a pistol
- **THEN** a `WeaponSlot` with `definition_index: 1` SHALL be inserted into `WeaponInventory.weapons[1]`

#### Scenario: Pick up fusion pistol (item type 2)

- **WHEN** the player picks up ITEM_FUSION_PISTOL and does not already have a fusion pistol
- **THEN** a `WeaponSlot` with `definition_index: 2` SHALL be inserted into the inventory

#### Scenario: Pick up assault rifle (item type 3)

- **WHEN** the player picks up ITEM_ASSAULT_RIFLE and does not already have an assault rifle
- **THEN** a `WeaponSlot` with `definition_index: 3` SHALL be inserted into the inventory

#### Scenario: Pick up missile launcher (item type 4)

- **WHEN** the player picks up ITEM_MISSILE_LAUNCHER and does not already have a missile launcher
- **THEN** a `WeaponSlot` with `definition_index: 4` SHALL be inserted into the inventory

#### Scenario: Pick up flamethrower (item type 5)

- **WHEN** the player picks up ITEM_FLAMETHROWER and does not already have a flamethrower
- **THEN** a `WeaponSlot` with `definition_index: 5` SHALL be inserted into the inventory

#### Scenario: Pick up alien weapon (item type 6)

- **WHEN** the player picks up ITEM_ALIEN_WEAPON and does not already have an alien weapon
- **THEN** a `WeaponSlot` with `definition_index: 6` SHALL be inserted into the inventory

#### Scenario: Pick up shotgun (item type 7)

- **WHEN** the player picks up ITEM_SHOTGUN and does not already have a shotgun
- **THEN** a `WeaponSlot` with `definition_index: 7` SHALL be inserted into the inventory

#### Scenario: Pick up SMGs (item type 8)

- **WHEN** the player picks up ITEM_SMGS and does not already have SMGs
- **THEN** a `WeaponSlot` with `definition_index: 8` SHALL be inserted into the inventory

#### Scenario: Duplicate weapon rejected

- **WHEN** the player picks up a weapon type already present in the inventory
- **THEN** the pickup SHALL be skipped, the item SHALL remain in the world, and no event SHALL be emitted

### Requirement: Apply ammunition pickup effects

The system SHALL add ammunition to the player's weapon reserves when an `AddAmmo` effect is resolved. Ammunition SHALL be added to the `primary_reserve` or `secondary_reserve` of the `WeaponSlot` matching the `weapon_definition_index`. If the player does not hold the corresponding weapon, the ammo SHALL still be added (to a reserve counter or the pickup SHALL be skipped, matching original Marathon behavior where ammo requires the weapon). If reserves are at maximum capacity, the pickup SHALL be skipped.

#### Scenario: Pick up pistol ammo (item type 10)

- **WHEN** the player picks up ITEM_PISTOL_AMMO and holds a pistol with room in reserves
- **THEN** the pistol's `primary_reserve` SHALL increase by 8

#### Scenario: Pick up fusion ammo (item type 11)

- **WHEN** the player picks up ITEM_FUSION_AMMO and holds a fusion pistol with room in reserves
- **THEN** the fusion pistol's `primary_reserve` SHALL increase by 20

#### Scenario: Pick up AR ammo (item type 12)

- **WHEN** the player picks up ITEM_AR_AMMO and holds an assault rifle with room in reserves
- **THEN** the assault rifle's `primary_reserve` SHALL increase by 52

#### Scenario: Pick up AR grenades (item type 13)

- **WHEN** the player picks up ITEM_AR_GRENADES and holds an assault rifle with room in secondary reserves
- **THEN** the assault rifle's `secondary_reserve` SHALL increase by 7

#### Scenario: Pick up missile ammo (item type 14)

- **WHEN** the player picks up ITEM_MISSILE_AMMO and holds a missile launcher with room in reserves
- **THEN** the missile launcher's `primary_reserve` SHALL increase by 2

#### Scenario: Pick up flamethrower ammo (item type 15)

- **WHEN** the player picks up ITEM_FLAMETHROWER_AMMO and holds a flamethrower with room in reserves
- **THEN** the flamethrower's `primary_reserve` SHALL increase by the defined amount

#### Scenario: Pick up alien ammo (item type 16)

- **WHEN** the player picks up ITEM_ALIEN_AMMO and holds an alien weapon with room in reserves
- **THEN** the alien weapon's `primary_reserve` SHALL increase by the defined amount

#### Scenario: Pick up shotgun ammo (item type 17)

- **WHEN** the player picks up ITEM_SHOTGUN_AMMO and holds a shotgun with room in reserves
- **THEN** the shotgun's `primary_reserve` SHALL increase by 2

#### Scenario: Pick up SMG ammo (item type 18)

- **WHEN** the player picks up ITEM_SMG_AMMO and holds SMGs with room in reserves
- **THEN** the SMGs' `primary_reserve` SHALL increase by the defined amount

#### Scenario: Ammo at maximum rejected

- **WHEN** the player picks up ammo but the weapon's reserve is already at maximum capacity
- **THEN** the pickup SHALL be skipped and the item SHALL remain in the world

### Requirement: Apply health restoration effects

The system SHALL restore the player's health when a `RestoreHealth` effect is resolved. Health SHALL be capped at 150. If the player's health is already at 150, the pickup SHALL be skipped and the item SHALL remain in the world.

#### Scenario: Pick up minor health (item type 20)

- **WHEN** the player has 100 health and picks up ITEM_HEALTH_MINOR
- **THEN** the player's health SHALL become 120

#### Scenario: Pick up major health (item type 21)

- **WHEN** the player has 100 health and picks up ITEM_HEALTH_MAJOR
- **THEN** the player's health SHALL become 140

#### Scenario: Health capped at 150

- **WHEN** the player has 140 health and picks up ITEM_HEALTH_MINOR (restores 20)
- **THEN** the player's health SHALL become 150 (not 160)

#### Scenario: Health at cap rejected

- **WHEN** the player has 150 health and a health item is within pickup range
- **THEN** the pickup SHALL be skipped and the item SHALL remain

### Requirement: Apply shield restoration effects with overshielding

The system SHALL restore the player's shield when a `RestoreShield` effect is resolved. Each shield canister tier has its own maximum cap: 1x canister caps at 150, 2x canister caps at 300, 3x canister caps at 450. If the player's shield is already at or above the canister's cap, the pickup SHALL be skipped.

#### Scenario: Pick up 1x shield (item type 23)

- **WHEN** the player has 50 shield and picks up ITEM_SHIELD_1X
- **THEN** the player's shield SHALL become 150

#### Scenario: Pick up 2x shield (item type 24)

- **WHEN** the player has 100 shield and picks up ITEM_SHIELD_2X
- **THEN** the player's shield SHALL become 300

#### Scenario: Pick up 3x shield (item type 25)

- **WHEN** the player has 200 shield and picks up ITEM_SHIELD_3X
- **THEN** the player's shield SHALL become 450

#### Scenario: 1x shield rejected when at cap

- **WHEN** the player has 150 shield and ITEM_SHIELD_1X is within range
- **THEN** the pickup SHALL be skipped (150 >= 1x cap of 150)

#### Scenario: 2x shield allowed when above 1x cap

- **WHEN** the player has 150 shield and picks up ITEM_SHIELD_2X
- **THEN** the player's shield SHALL become 300 (150 < 2x cap of 300, so pickup allowed)

### Requirement: Apply oxygen restoration effects

The system SHALL restore the player's oxygen when a `RestoreOxygen` effect is resolved. Oxygen SHALL be capped at 600. If the player's oxygen is already at 600, the pickup SHALL be skipped.

#### Scenario: Pick up oxygen canister (item type 22)

- **WHEN** the player has 200 oxygen and picks up ITEM_OXYGEN
- **THEN** the player's oxygen SHALL become 600 (200 + 600, capped at 600)

#### Scenario: Oxygen at cap rejected

- **WHEN** the player has 600 oxygen and an oxygen canister is within range
- **THEN** the pickup SHALL be skipped

### Requirement: Apply powerup effects with duration tracking

The system SHALL activate powerup timers when a powerup item is picked up. The `PowerupTimers` component on the player entity tracks four independent countdown timers. On pickup, the corresponding timer SHALL be set to the powerup's duration (or the maximum of the current remaining time and the new duration, whichever is greater). Each tick, all non-zero timers SHALL decrement by 1. When a timer reaches zero, the powerup effect expires.

#### Scenario: Pick up invincibility (item type 26)

- **WHEN** the player picks up ITEM_INVINCIBILITY
- **THEN** the player's `PowerupTimers.invincibility` SHALL be set to 1500

#### Scenario: Pick up invisibility (item type 27)

- **WHEN** the player picks up ITEM_INVISIBILITY
- **THEN** the player's `PowerupTimers.invisibility` SHALL be set to 2100

#### Scenario: Pick up infravision (item type 28)

- **WHEN** the player picks up ITEM_INFRAVISION
- **THEN** the player's `PowerupTimers.infravision` SHALL be set to 1800

#### Scenario: Pick up extravision (item type 29)

- **WHEN** the player picks up ITEM_EXTRAVISION
- **THEN** the player's `PowerupTimers.extravision` SHALL be set to 1800

#### Scenario: Powerup timer counts down each tick

- **WHEN** the player has `invincibility = 100` and a tick elapses with no new powerup pickup
- **THEN** `invincibility` SHALL become 99

#### Scenario: Powerup timer expires

- **WHEN** the player has `invincibility = 1` and a tick elapses
- **THEN** `invincibility` SHALL become 0 and the invincibility effect SHALL no longer apply

#### Scenario: Duplicate powerup pickup refreshes timer

- **WHEN** the player has `invisibility = 500` and picks up another ITEM_INVISIBILITY
- **THEN** `invisibility` SHALL become 2100 (max of 500 and 2100)

### Requirement: Apply inventory item effects

The system SHALL add inventory items (keys, balls) to the player's inventory counter when an `AddInventoryItem` effect is resolved. Inventory items are always picked up (no cap check). The system SHALL track a count per item type.

#### Scenario: Pick up uplink chip (item type 30)

- **WHEN** the player picks up ITEM_UPLINK_CHIP
- **THEN** the player's inventory SHALL contain 1 uplink chip

#### Scenario: Pick up ball items (item types 31-38)

- **WHEN** the player picks up any ball item (ITEM_LIGHT_BLUE_BALL through ITEM_GREEN_BALL)
- **THEN** the player's inventory counter for that ball type SHALL increment by 1

### Requirement: Item respawn in multiplayer

The system SHALL support item respawn timers for multiplayer mode. When an item is picked up and the simulation is in multiplayer mode, the system SHALL enqueue a respawn entry with the item's type, position, polygon index, and a delay of 300 ticks (10 seconds). Each tick, the system SHALL decrement all respawn timers. When a timer reaches zero, the system SHALL spawn a new Item entity at the recorded position with the recorded type.

#### Scenario: Item respawns after delay

- **WHEN** an item is picked up in multiplayer mode
- **THEN** after 300 ticks, a new item entity of the same type SHALL be spawned at the original position

#### Scenario: No respawn in single-player

- **WHEN** an item is picked up in single-player mode
- **THEN** no respawn entry SHALL be created and the item SHALL be permanently removed

#### Scenario: Multiple items respawning concurrently

- **WHEN** three items are picked up on the same tick in multiplayer
- **THEN** all three SHALL respawn after 300 ticks, each at their original positions

### Requirement: Emit pickup events for sound and HUD

The system SHALL emit a `SimEvent::ItemPickedUp` event containing the item type whenever an item is successfully picked up. This event is consumed by the integration layer for pickup sound playback and HUD notifications.

#### Scenario: Pickup event emitted

- **WHEN** the player picks up any item successfully
- **THEN** a `SimEvent::ItemPickedUp { item_type }` event SHALL be emitted with the correct item type

#### Scenario: No event on skipped pickup

- **WHEN** a pickup is skipped because the effect would be wasted
- **THEN** no `SimEvent::ItemPickedUp` event SHALL be emitted
