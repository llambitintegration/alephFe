## MODIFIED Requirements

### Requirement: Advance simulation by one tick

The system SHALL advance the simulation state by exactly one tick (1/30th of a second) when `tick()` is called with the current frame's `TickInput`. All systems SHALL execute in the defined order: input processing, player physics, **item pickup**, monster AI, weapon/combat, projectile physics, damage resolution, world mechanics, cleanup. The item pickup step SHALL run after the player's position is finalized and before weapon/combat systems, so that newly acquired weapons and ammunition are available in the same tick.

#### Scenario: Single tick advance with item pickup

- **WHEN** `tick()` is called and the player is standing on a health canister with health below 150
- **THEN** the player physics step SHALL update the player's position, the item pickup step SHALL detect proximity and restore health, and the remaining systems SHALL execute in order

#### Scenario: Weapon picked up available for combat in same tick

- **WHEN** `tick()` is called and the player is standing on a weapon item they do not have
- **THEN** after the item pickup step, the weapon SHALL be in the player's inventory, and the weapon/combat step SHALL be able to reference it

#### Scenario: Powerup timers tick down each frame

- **WHEN** `tick()` is called and the player has active powerup timers
- **THEN** all non-zero powerup timers SHALL decrement by 1 during the item pickup / powerup tick-down step

#### Scenario: Item respawn timers advance each frame

- **WHEN** `tick()` is called in multiplayer mode with pending item respawns
- **THEN** all respawn timers SHALL decrement by 1, and any that reach zero SHALL spawn new item entities

### Requirement: Construct simulation world from map and physics data

The system SHALL construct a `SimWorld` from `MapData` and `PhysicsData`. Construction SHALL additionally attach a `PowerupTimers` component (all zeros) and a `WeaponInventory` component (with fists in slot 0) to the player entity. The system SHALL initialize an `ItemRespawnQueue` resource (empty).

#### Scenario: Player spawned with powerup timers and weapon inventory

- **WHEN** `SimWorld::new()` is called with valid map data
- **THEN** the player entity SHALL have a `PowerupTimers` component with all timers at 0, and a `WeaponInventory` component with fists (weapon definition index 0) in slot 0

#### Scenario: Respawn queue initialized empty

- **WHEN** `SimWorld::new()` is called
- **THEN** the world SHALL contain an empty `ItemRespawnQueue` resource

### Requirement: Query player state

The system SHALL additionally expose accessor methods for the player's weapon inventory, active powerup timers, and inventory items.

#### Scenario: Query weapon inventory

- **WHEN** `sim_world.player_weapons()` is called after picking up a shotgun
- **THEN** the system SHALL return the weapon inventory showing the shotgun in its slot

#### Scenario: Query active powerups

- **WHEN** `sim_world.player_powerups()` is called while invincibility is active
- **THEN** the system SHALL return the `PowerupTimers` showing a non-zero invincibility value

### Requirement: Serialize and deserialize simulation state

The system SHALL include `PowerupTimers`, `WeaponInventory`, `InventoryItems`, and `ItemRespawnQueue` in the serialization snapshot. Deserialization SHALL restore all pickup-related state faithfully.

#### Scenario: Round-trip with active powerups

- **WHEN** a `SimWorld` with active invincibility (500 ticks remaining) is serialized then deserialized
- **THEN** the restored world's player SHALL have `invincibility = 500` in `PowerupTimers`

#### Scenario: Round-trip with pending respawns

- **WHEN** a `SimWorld` with 2 pending respawn entries is serialized then deserialized
- **THEN** the restored world's `ItemRespawnQueue` SHALL contain 2 entries with correct remaining ticks
