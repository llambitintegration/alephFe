# fix-wall-collision Tasks

## Implementation Tasks

- [x] Add `point_to_segment_distance(point, seg_a, seg_b) -> (f32, Vec2)` function to `marathon-sim/src/collision.rs` that returns the distance from a point to a line segment and the closest point on the segment
- [x] Add unit tests for `point_to_segment_distance` in `marathon-sim/src/collision.rs`
- [x] Update `apply_player_collision()` in `marathon-sim/src/player/movement.rs` to use radius-based collision instead of point-based: for each solid wall, check if player center is within `radius` distance of the wall segment, and push player outward if so
- [x] Update existing collision tests in `marathon-sim/src/player/movement.rs` to validate radius-based behavior
- [x] Verify the build compiles via Docker
