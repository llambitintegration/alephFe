## Context

Items spawn in the world and render correctly, but the player walks through them with no effect. The `item_effect()` function in `world_mechanics/items.rs` maps all 39 item types to their effects (`AddWeapon`, `AddAmmo`, `RestoreHealth`, `RestoreShield`, `RestoreOxygen`, `AddInventoryItem`), but nothing in the tick loop calls it. Without pickups there is no ammo, no health recovery, no shields, no weapons beyond fists. The game is a walking simulator.

The `ItemRespawnState` struct exists with tick-based countdown logic but is never wired into a world resource. The `WeaponInventory` component exists in `player/inventory.rs` with slot management, but is not attached to the player entity at spawn. Items are spawned with `CollisionRadius(0.25)` and `PolygonIndex`, giving us the spatial data needed for proximity checks.

Marathon's original pickup system (from `items.cpp`) uses a 2D distance check against the player's collision radius, validates same-or-adjacent polygon occupancy, applies effects, despawns the item, plays a sound, and starts a respawn timer in multiplayer.

## Goals / Non-Goals

**Goals:**
- Per-tick proximity detection between the player and all Item entities, matching Marathon's pickup radius (~1 WU)
- Apply all 39 item effects: weapons into inventory, ammo into reserves, health/shield/oxygen restoration with correct caps, inventory items (keys/balls)
- Shield overshielding: 1x cap at 150, 2x cap at 300, 3x cap at 450
- Health cap at 150, oxygen cap at 600
- Skip pickup when effect would be wasted (health at cap, weapon already held, ammo at max)
- Powerup duration tracking for invincibility (1500 ticks), invisibility (2100 ticks), infravision (1800 ticks), extravision (1800 ticks) with per-tick countdown
- Item despawn on successful pickup
- Emit `SimEvent::ItemPickedUp` for sound/HUD feedback
- Multiplayer item respawn timers (drive the existing `ItemRespawnState`)
- Deterministic: no randomness in pickup logic, consistent with sim's deterministic guarantee

**Non-Goals:**
- Monster item drops on death (separate change; requires monster death handling)
- Pickup animations or visual feedback beyond the SimEvent
- Item bobbing/rotation animations
- Difficulty-based item placement filtering
- Projectile `becomes_item_on_detonation` spawning (separate system)
- Networked multiplayer synchronization (deterministic sim handles this implicitly)

## Decisions

### Decision 1: 2D distance check with polygon validation

**Choice:** Use a 2D (XY-plane) Euclidean distance check between the player position and each item position, with a pickup radius of 1.0 WU. Additionally require that the item is in the same polygon as the player or in a polygon adjacent via a non-solid line.

**Alternative considered:** 3D distance check -- rejected because Marathon's original engine uses 2D distance. Height differences within a polygon are handled by the polygon/adjacency check (items on a different floor won't be in an adjacent polygon).

**Alternative considered:** Spatial partitioning (grid, BVH) -- rejected as premature optimization. Marathon levels typically have fewer than 100 items. A linear scan over all items per tick is negligible at 30 Hz.

**Rationale:** Matches original behavior. The polygon adjacency check prevents picking up items through walls or on different floors. The 1.0 WU radius is generous enough to feel responsive while matching the original engine's feel (the original used the player's collision radius of ~0.25 WU, but the Rust rebuild's movement step size at 30Hz means a larger radius provides equivalent reachability).

### Decision 2: Shield overshielding via per-canister caps

**Choice:** Each shield canister type has its own maximum cap: 1x caps at 150, 2x caps at 300, 3x caps at 450. Pickup is allowed if current shield is below the canister's cap. Shield is set to `min(current + amount, cap)`.

**Alternative considered:** Single global shield cap -- rejected because Marathon explicitly supports overshielding via higher-tier canisters.

**Rationale:** Matches original Marathon behavior exactly. A player at 150 shield can pick up a 2x canister to reach 300, but cannot pick up a 1x canister (already at its cap).

### Decision 3: PowerupTimers as a single ECS component on the player

**Choice:** Add a `PowerupTimers` component with four `u16` fields (invincibility, invisibility, infravision, extravision) to the player entity. Each tick, non-zero values decrement by 1. On pickup, set the field to the powerup's duration (or add to it if stacking is desired -- Marathon does not stack, so set to max of current and new).

**Alternative considered:** Four separate components (`Invincibility(u16)`, etc.) -- rejected because they always apply to the same entity (player) and are always ticked together. A single struct reduces query complexity.

**Alternative considered:** A HashMap or Vec of active powerups -- rejected as over-engineered for exactly 4 known powerup types.

**Rationale:** Simple, cache-friendly, serializable. The component is cheap to add to the player spawn and to include in snapshots.

### Decision 4: Respawn queue as a world resource

**Choice:** Add an `ItemRespawnQueue` resource (a `Vec<ItemRespawnEntry>`) that holds despawned item metadata (type, position, polygon, remaining ticks). Each tick, all entries are decremented; when an entry reaches zero, a new Item entity is spawned at the original position. This is only active in multiplayer mode (gated by a config flag).

**Alternative considered:** Keep despawned items as invisible entities with a timer component -- rejected because it pollutes the entity query for rendering and requires filtering out "pending respawn" items everywhere.

**Rationale:** Matches the existing `ItemRespawnState` design. The resource is a simple list that gets ticked linearly. In single-player, items simply despawn permanently (the queue is empty).

### Decision 5: Pickup system runs between player physics and weapon/combat

**Choice:** Insert `run_item_pickups()` in the tick loop after player physics (step 2) and before monster AI (step 3). This ensures the player's position is finalized for the current tick before proximity checks, and any newly acquired weapons are available for the combat step.

**Alternative considered:** Run after all systems as a cleanup step -- rejected because weapon acquisition should be available to the combat system in the same tick (player picks up a weapon and can immediately switch to it).

**Rationale:** Natural ordering. The player moves, then picks up items at the new position, then combat systems can reference the updated inventory.

### Decision 6: Wasted pickup prevention

**Choice:** Before applying an effect, check if it would be wasted:
- `RestoreHealth`: skip if health >= 150
- `RestoreShield`: skip if shield >= canister cap
- `RestoreOxygen`: skip if oxygen >= 600
- `AddWeapon`: skip if weapon slot already occupied
- `AddAmmo`: skip if reserve ammo at maximum (use physics data `maximum_reserve` if available, otherwise a generous cap like 999)
- `AddInventoryItem`: always pick up (keys/balls are always useful)

The item remains in the world if pickup is skipped.

**Rationale:** Matches original Marathon behavior. Items you don't need stay on the ground for later.

## Risks / Trade-offs

- **[Pickup radius tuning]** The 1.0 WU radius may feel too generous or too tight compared to original Marathon. Mitigation: the constant is easily adjustable, and playtesting will reveal the right value. The original used ~0.25 WU but had higher tick rates for item checks.
- **[Polygon adjacency false negatives]** If the map geometry has unusual polygon layouts, the same-or-adjacent check might prevent legitimate pickups. Mitigation: this matches the original engine's behavior, so any map that worked in Marathon will work here.
- **[Ammo cap enforcement]** Without per-weapon `maximum_reserve` values from physics data, we may allow unlimited ammo stacking. Mitigation: use the physics data values when available; fall back to a large-but-bounded default.
- **[Powerup stacking]** Marathon does not stack powerup durations (picking up a second invincibility resets the timer). If we use `max(current, new_duration)`, a second pickup while active extends nothing unless the player is near expiry. This matches original behavior but may surprise players. Mitigation: document the behavior; it matches Marathon faithfully.
- **[Serialization]** Adding `PowerupTimers` and `ItemRespawnQueue` requires updating `SimSnapshot` and the serialize/deserialize path. Mitigation: both are simple structs with serde derives; the snapshot format is not yet stabilized.
