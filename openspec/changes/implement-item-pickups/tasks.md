## 1. Add New Components and Resources

- [x] 1.1 Add `PowerupTimers` component to `marathon-sim/src/components.rs` with fields: `invincibility: u16`, `invisibility: u16`, `infravision: u16`, `extravision: u16` (all default 0). Derive `Component, Debug, Clone, Copy, Default, Serialize, Deserialize`.
- [x] 1.2 Add `InventoryItems` component to `marathon-sim/src/components.rs` with a `HashMap<i16, u16>` tracking item type to count. Derive `Component, Debug, Clone, Default, Serialize, Deserialize`.
- [x] 1.3 Add `ItemRespawnQueue` resource to `marathon-sim/src/world.rs` as a `Vec<ItemRespawnEntry>` where `ItemRespawnEntry` has `item_type: i16`, `position: Vec3`, `polygon_index: usize`, `remaining_ticks: u16`. Derive `Resource, Debug, Default, Clone, Serialize, Deserialize`.
- [x] 1.4 Add `SimEvent::ItemPickedUp { item_type: i16 }` variant to the `SimEvent` enum in `marathon-sim/src/world.rs`.
- [x] 1.5 Add unit tests: verify `PowerupTimers::default()` has all zeros, verify `InventoryItems::default()` is empty.

## 2. Attach Components at Player Spawn

- [ ] 2.1 In `spawn_map_objects()` in `marathon-sim/src/world.rs`, add `PowerupTimers::default()` and `InventoryItems::default()` to the player entity spawn bundle.
- [ ] 2.2 In `spawn_map_objects()`, create a `WeaponInventory` with fists in slot 0 (weapon definition index 0) and attach it to the player entity as a component (or insert it as a resource, matching existing inventory architecture).
- [ ] 2.3 Insert an empty `ItemRespawnQueue` resource in `SimWorld::new()`.
- [ ] 2.4 Add unit test: construct a `SimWorld` from test data and verify the player entity has `PowerupTimers`, `InventoryItems`, and `WeaponInventory`.

## 3. Fill Missing `item_effect()` Match Arms

- [x] 3.1 In `marathon-sim/src/world_mechanics/items.rs`, add match arms for `ITEM_FLAMETHROWER_AMMO`, `ITEM_ALIEN_AMMO`, and `ITEM_SMG_AMMO` returning `AddAmmo` with appropriate weapon indices and amounts.
- [x] 3.2 Add match arms for `ITEM_INVINCIBILITY`, `ITEM_INVISIBILITY`, `ITEM_INFRAVISION`, `ITEM_EXTRAVISION` returning `AddInventoryItem` (or a new `ActivatePowerup` variant if preferred) with the correct durations.
- [x] 3.3 Add `ActivatePowerup { powerup_type: i16, duration_ticks: u16 }` variant to `ItemEffect` enum if using a dedicated variant for powerups (alternative: handle powerup items as `AddInventoryItem` and check item type in the pickup system). [Chose dedicated `ActivatePowerup` variant; added non-breaking stub arm in `tick.rs` apply-pickup match pending PowerupTimers wiring (box 4.8/5.x).]
- [x] 3.4 Add unit tests for every new match arm: verify correct effect type, amounts, and weapon indices.

## 4. Implement Core Pickup Logic

- [ ] 4.1 Create `marathon-sim/src/world_mechanics/pickup.rs` (or add to items.rs) with a function `can_pickup(player_pos: Vec2, player_poly: usize, item_pos: Vec2, item_poly: usize, geometry: &MapGeometry) -> bool` that checks 2D distance < 1.0 WU and same-or-adjacent polygon.
- [ ] 4.2 Implement `apply_item_effect()` function that takes a mutable player state (Health, Shield, Oxygen, WeaponInventory, PowerupTimers, InventoryItems) and an `ItemEffect`, applies the effect with cap enforcement, and returns `bool` (true if effect applied, false if wasted).
- [ ] 4.3 Health cap logic: clamp `Health.0` at 150 after adding amount. Return false if already at 150.
- [ ] 4.4 Shield cap logic: determine cap from amount (150 -> cap 150, 300 -> cap 300, 450 -> cap 450). Return false if shield >= cap. Set shield to `min(current + amount, cap)`.
- [ ] 4.5 Oxygen cap logic: clamp `Oxygen.0` at 600 after adding amount. Return false if already at 600.
- [ ] 4.6 Weapon grant logic: check `WeaponInventory.weapons[def_index]`. If `Some`, return false. Otherwise insert a new `WeaponSlot` with zeroed magazines/reserves.
- [ ] 4.7 Ammo grant logic: find `WeaponSlot` by `weapon_definition_index`. Add amount to `primary_reserve` or `secondary_reserve`. Cap at maximum. Return false if already at max.
- [ ] 4.8 Powerup activation logic: set `PowerupTimers` field to `max(current, duration)`. Always returns true (powerups are always picked up).
- [ ] 4.9 Inventory item logic: increment count in `InventoryItems` map. Always returns true.
- [ ] 4.10 Add unit tests for `can_pickup()`: same polygon within range, same polygon out of range, adjacent polygon within range, non-adjacent polygon, solid-wall adjacent.
- [ ] 4.11 Add unit tests for `apply_item_effect()`: each effect type with normal application, cap enforcement, and wasted-pickup rejection.

## 5. Wire Pickup System into Tick Loop

- [ ] 5.1 Add `run_item_pickups()` method on `SimWorld` in `marathon-sim/src/tick.rs`. Query player `(Position, PolygonIndex, Health, Shield, Oxygen, PowerupTimers, InventoryItems)` plus `WeaponInventory`. For each `(Entity, Item, Position, PolygonIndex)` item entity, call `can_pickup()` and `apply_item_effect()`. On success, despawn item entity and emit `SimEvent::ItemPickedUp`.
- [ ] 5.2 Call `run_item_pickups()` in `SimWorld::tick()` after `run_player_physics()` and before the tick counter increment.
- [ ] 5.3 Add `run_powerup_countdown()` method that decrements all non-zero `PowerupTimers` fields by 1. Call it in `tick()` after item pickups.
- [ ] 5.4 Add `run_item_respawns()` method that decrements all entries in `ItemRespawnQueue`, spawns new Item entities for entries that reach zero, and removes completed entries. Call it in `tick()`.
- [ ] 5.5 Add integration test: construct world with player near a health item, tick once, verify health increased and item despawned.
- [ ] 5.6 Add integration test: construct world with player near a weapon, tick once, verify weapon in inventory.
- [ ] 5.7 Add integration test: construct world with player at full health near health item, tick once, verify item NOT despawned.
- [ ] 5.8 Add integration test: activate invincibility, tick 1500 times, verify timer reaches 0.

## 6. Update Player State Queries

- [ ] 6.1 Add `player_weapons()` accessor on `SimWorld` returning `Option<&WeaponInventory>`.
- [ ] 6.2 Add `player_powerups()` accessor on `SimWorld` returning `Option<PowerupTimers>`.
- [ ] 6.3 Add `player_inventory()` accessor on `SimWorld` returning inventory item counts.
- [ ] 6.4 Add unit tests for each new accessor.

## 7. Update Serialization

- [ ] 7.1 Add `powerup_timers` and `inventory_items` fields to `PlayerSnapshot` in `marathon-sim/src/world.rs`.
- [ ] 7.2 Add `weapon_inventory` field to `PlayerSnapshot`.
- [ ] 7.3 Add `respawn_queue: Vec<ItemRespawnEntry>` field to `SimSnapshot`.
- [ ] 7.4 Update `SimWorld::snapshot()` to read and include `PowerupTimers`, `InventoryItems`, `WeaponInventory`, and `ItemRespawnQueue`.
- [ ] 7.5 Update `SimWorld::deserialize()` to restore `PowerupTimers`, `InventoryItems`, `WeaponInventory` on the player entity and `ItemRespawnQueue` as a resource.
- [ ] 7.6 Add round-trip serialization test: set powerup timers, serialize, deserialize, verify timers preserved.
- [ ] 7.7 Add round-trip serialization test: add respawn entries, serialize, deserialize, verify entries preserved.

## 8. Combat System Integration

- [ ] 8.1 In the damage application path (when it exists), check `PowerupTimers.invincibility > 0` on the player entity and skip damage if active.
- [ ] 8.2 Add unit test: player with invincibility takes no damage from a projectile hit.

## 9. Full Integration Testing

- [ ] 9.1 Run full `cargo test` suite in Docker and verify all existing + new tests pass.
- [ ] 9.2 Deploy to marathon.llambit.io and verify items can be picked up during gameplay.
- [ ] 9.3 Verify health/shield/oxygen HUD updates on pickup (requires HUD integration reading the new accessors).
- [ ] 9.4 Verify weapons acquired from pickups can be selected and fired.
