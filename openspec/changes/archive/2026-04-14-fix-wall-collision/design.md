## Context

Wall collision in `apply_player_collision()` currently uses `segment_intersection(old_pos, new_pos, wall_a, wall_b)` to detect when the player's movement ray crosses a wall line. This is a point-based test -- the player is treated as a zero-width point. The `PlayerPhysicsParams.radius` field (sourced from Marathon's `PhysicsConstants.radius`, typically 0.25 WU) is loaded but never consulted during collision.

The result: the player can stand flush against walls, and the camera (offset from player center by `EYE_HEIGHT` vertically) clips into geometry. The original Aleph One engine uses radius-based collision to keep the player's circular cross-section away from walls.

## Goals / Non-Goals

**Goals:**
- Collision checks use `params.radius` so the player's center stays at least `radius` distance from solid wall segments
- Sliding behavior preserved -- movement parallel to walls still works, just offset by radius
- Camera clipping eliminated as a side effect (camera derives from player position)
- New `point_to_segment_distance` primitive added to collision.rs for reuse (monster AI, projectiles, etc.)

**Non-Goals:**
- Camera-specific collision (the camera is not an independent collider; fixing player radius is sufficient)
- Entity-vs-entity radius collision (separate concern, uses existing `circles_overlap`)
- Changing the collision radius value itself (that comes from physics data)
- Continuous collision detection (swept circle) -- the per-tick displacement is small enough that discrete radius checks suffice

## Decisions

### 1. Distance-based check instead of swept-circle

**Decision**: For each wall segment in the current polygon, compute `point_to_segment_distance(player_center, wall_a, wall_b)`. If distance < radius and the wall is impassable, push the player out and slide.

**Rationale**: The current iteration loop already handles multi-wall slides (up to 3 iterations). Replacing the intersection test with a distance test fits the same structure. A swept-circle approach (Minkowski sum) would be more accurate for high-speed movement but is unnecessary -- Marathon physics has low velocities relative to wall thickness, and the 3-iteration loop catches corner cases.

**Alternatives considered**: Minkowski sum expansion (expand walls outward by radius, then use point intersection). This works but requires pre-computing expanded walls for every polygon and complicates the adjacency/passability logic. Distance-based is simpler and equivalent for the discrete case.

### 2. Push-out along wall normal

**Decision**: When `distance < radius`, compute the push direction as the vector from the closest point on the segment to the player center, and move the player to `closest_point + push_direction.normalize() * radius`.

**Rationale**: Using the closest-point direction (not the wall normal) handles corners correctly -- when the player is near a wall endpoint, the push direction naturally curves around the corner rather than popping the player to the wrong side.

### 3. Keep segment_intersection for passable line crossing detection

**Decision**: The distance check handles solid walls. For passable lines (transparent sides with adjacent polygons), keep using `segment_intersection` to detect when the player actually crosses into the adjacent polygon and needs a polygon index update.

**Rationale**: The radius check answers "am I too close to this wall?" while the intersection test answers "did I cross this boundary?" These are different questions. A player can be within radius of a passable line without having crossed it.

## Risks / Trade-offs

- **[Corner wedging]** -> The 3-iteration multi-slide loop should handle acute corners. If a player gets pushed into a second wall, the next iteration catches it. The existing iteration count is sufficient.
- **[Passable line radius interaction]** -> Player should not be pushed away from passable lines. The distance check is only applied when `can_pass` is false, so passable lines are unaffected.
- **[Performance]** -> `point_to_segment_distance` is a trivial computation (dot products + clamp). No measurable impact compared to the existing `segment_intersection` calls.
