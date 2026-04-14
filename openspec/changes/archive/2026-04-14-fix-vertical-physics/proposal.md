## Why

The player floats in mid-air after walking off a ledge because `apply_player_collision` only snaps the player UP to higher adjacent floors (via `z.max(adj_floor)`) but never adjusts Z downward when crossing to a lower polygon. Additionally, there is no ceiling collision during upward movement, and player spawn Z uses raw map data without validation against the polygon's actual floor height.

## What Changes

- When crossing into a polygon with a lower floor, set the player's Z to that polygon's floor height if the player was grounded (snap down), or leave them airborne so gravity pulls them down naturally
- Add ceiling collision: clamp Z to `ceiling_height - player_height` when the player's head would penetrate the ceiling during upward movement
- Validate player spawn Z against the polygon's floor height, ensuring the player starts on the floor rather than at whatever raw Z the map data provides

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `player-physics`: The "Apply gravity" requirement's floor-crossing behavior needs correction (snap down to lower floors), ceiling collision during upward movement needs implementation, and spawn Z validation needs to be specified

## Impact

- `marathon-sim/src/player/movement.rs` — `apply_player_collision()` lines 317-326 (floor crossing logic) and lines 358-364 (grounding logic); add ceiling clamp
- `marathon-sim/src/world.rs` — `spawn_map_objects()` around line 288-308 (player spawn position)
- `marathon-sim/src/tick.rs` — no changes expected (calls collision correctly already)
- `marathon-sim/src/components.rs` — no changes expected
- `marathon-sim/tests/integration.rs` — new test cases for ledge drop, ceiling collision, spawn Z validation
