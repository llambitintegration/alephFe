## Context

The Marathon sim layer has vertical (Z-axis) physics implemented in `apply_player_collision()` in `marathon-sim/src/player/movement.rs`. The current implementation correctly handles:
- Gravity pulling the player down when airborne (lines 160-165)
- Grounding the player when Z reaches the floor (lines 358-364)
- Stepping up to higher adjacent floors within step_delta (lines 322-325)

However, three bugs exist:
1. When crossing to a polygon with a **lower** floor, the player's Z is never adjusted downward. The code at line 324 only does `z = z.max(adj_floor)`, which is a no-op when adj_floor < z. The grounding check at line 360 only fires if `z <= floor + EPSILON`, but after crossing to a lower polygon the player's Z is still at the old (higher) floor, so they float.
2. No ceiling collision exists during upward movement (e.g., from gravity reversal or future jumping).
3. Player spawn at line 308 uses raw `obj.location.z` from map data without clamping to the polygon's actual floor height.

## Goals / Non-Goals

**Goals:**
- Fix ledge drop: player falls (or snaps) to the floor when crossing to a lower polygon
- Add ceiling collision: clamp player Z so they cannot exceed `ceiling_height - player_height`
- Validate spawn Z: ensure player starts at or above the polygon floor height

**Non-Goals:**
- Adding jumping mechanics (no jump action flag exists yet)
- Modifying gravity constants or terminal velocity
- Platform/elevator ride physics (separate system)
- Falling damage

## Decisions

### Decision 1: Snap down vs. gravity fall when crossing to lower polygon

**Choice**: When the player is grounded and crosses to a lower polygon, immediately set Z to the lower floor height (snap down). When the player is airborne, let gravity handle it naturally.

**Rationale**: The original Aleph One engine snaps grounded players to the floor of whatever polygon they occupy. This matches expected Marathon behavior where walking off a small ledge is instant, not a slow gravity fall. Gravity already works correctly for truly airborne players.

**Alternative considered**: Always let gravity pull the player down. Rejected because it creates a visible "floating" frame or two even for tiny floor differences (e.g., stair steps going down), which feels wrong.

### Decision 2: Ceiling collision placement

**Choice**: Add ceiling collision check in `apply_player_collision()` right after the grounding logic, as a final Z clamp: `z = z.min(ceiling - player_height)`.

**Rationale**: This catches all cases of ceiling penetration regardless of source (gravity reversal, platform push, etc.) in one place. Keeps the ceiling logic co-located with the floor grounding logic.

### Decision 3: Spawn Z validation location

**Choice**: Clamp spawn Z in `spawn_map_objects()` in `world.rs`, right where the Position is constructed, using `z.max(floor_heights[polygon])`.

**Rationale**: This is the simplest fix. The floor_heights data is already available via `MapGeometry` at that point. Fixing it at the source prevents any downstream issues.

## Risks / Trade-offs

- **[Risk] Snap-down changes feel for maps with large drops** -- For very large drops (e.g., 5+ WU), the instant snap might look jarring. Mitigation: this matches original engine behavior; if needed later, a threshold can trigger airborne state instead. For now, keep it simple.
- **[Risk] Ceiling clamp could interact with future elevator mechanics** -- Mitigation: Elevator/platform systems will need to update both floor and ceiling heights dynamically. The clamp logic works with whatever heights are in `MapGeometry`, so it will compose correctly.
