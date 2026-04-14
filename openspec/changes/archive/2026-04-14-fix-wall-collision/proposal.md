## Why

Wall collision detection uses a point-based intersection test (segment_intersection on the player's center point), allowing the player to get within zero distance of walls before collision triggers. The `CollisionRadius` component (0.25 WU) and `PlayerPhysicsParams.radius` field exist but are never referenced in collision checks, so the player's circular body is treated as an infinitely small point. This causes wall clipping and camera penetration through geometry.

## What Changes

- Replace point-to-line collision with circle-to-line (radius-aware) collision in `apply_player_collision`. For each wall segment, compute the shortest distance from the player's center to the line segment; if that distance is less than `radius`, push the player outward along the wall normal until they are exactly `radius` away.
- Add a `point_to_segment_distance` helper function to `collision.rs` that returns the shortest distance (and closest point) from a point to a line segment.
- Update the wall slide response to account for the radius offset -- the slide origin is the pushed-back position, not the raw intersection point.
- Wire `params.radius` into the collision loop where it is currently unused.

## Capabilities

### New Capabilities

_(none -- this is a fix to an existing capability)_

### Modified Capabilities

- `player-physics`: The "Collision with walls" requirement must specify that collision uses the player's collision radius, not a point test. The player center must stay at least `radius` distance from any solid wall segment.

## Impact

- `marathon-sim/src/collision.rs` -- new `point_to_segment_distance()` function + tests
- `marathon-sim/src/player/movement.rs` -- `apply_player_collision()` rewritten to use radius-based distance check instead of segment_intersection for wall detection
- `marathon-sim/src/components.rs` -- no changes (CollisionRadius already defined)
- `marathon-sim/tests/integration.rs` -- existing collision tests may need updated expected positions; new tests for radius-based behavior
