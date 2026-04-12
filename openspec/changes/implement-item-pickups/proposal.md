## Why

The game is unplayable. Items spawn in the world and render correctly, but the player walks straight through them -- the `item_effect()` function in `world_mechanics/items.rs` maps all 39 item types to their effects, yet nothing in the tick loop ever calls it. Without pickups there is no ammo, no health recovery, no shields, no weapons beyond fists. Every other system (movement, rendering, combat) is wired up; item pickup is the gap that makes the game a walking simulator.

## What Changes

- Add a proximity check in the tick loop that tests player position against all Item entities each tick (~1 world unit pickup radius, matching Alephone)
- When overlap is detected, resolve `item_effect()` for the item's type and apply it to the player: mutate Health/Shield/Oxygen components (with caps), add weapons to inventory, add ammo to reserves, grant inventory items
- Despawn the item entity on successful pickup; skip pickup if the effect would be wasted (health at cap, already holding weapon, ammo at max)
- Add powerup duration tracking for invincibility (1500 ticks), invisibility (2100 ticks), infravision, and extravision -- these need a new component or resource to count down each tick
- Support item respawn timers for multiplayer (the `ItemRespawnState` struct already exists but is never driven)

## Capabilities

### New Capabilities
- `item-pickup-system`: Per-tick proximity detection between the player entity and all Item entities, effect resolution via `item_effect()`, component mutation (health/shield/oxygen/ammo/weapons/inventory), item despawn, and cap enforcement. Includes powerup duration countdown and multiplayer respawn timer integration.

### Modified Capabilities
- `game-loop`: The `SimWorld::tick()` method gains a new system step (between player physics and cleanup) that runs item pickup checks
- `combat-system`: Ammunition and weapon inventory must accept external additions from pickup effects, not only from initial loadout

## Impact

- **marathon-sim/src/tick.rs**: Add `run_item_pickups()` call in `tick()` after player physics; implement the proximity query and effect application
- **marathon-sim/src/components.rs**: Likely new `PowerupTimers` component (or resource) tracking active invincibility/invisibility/infravision/extravision countdowns; possibly `Inventory` component for key items
- **marathon-sim/src/world_mechanics/items.rs**: `item_effect()` is already complete -- no changes expected; `ItemRespawnState` may need to be wired into a world resource
- **marathon-sim/src/world.rs**: `SimWorld` may need a respawn queue resource for multiplayer item respawn
- **No new crates or dependencies**
- **Existing tests**: The `item_pickup_gives_correct_effects` integration test validates `item_effect()` return values; new tests needed for the actual pickup-during-tick behavior, cap enforcement, and powerup expiry
