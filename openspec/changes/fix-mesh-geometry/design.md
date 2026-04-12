## Context

Marathon map polygons declare `vertex_count` (up to 8) and an array of `endpoint_indexes`. Some entries in this array can be `-1` (sentinel for "no endpoint"), which is valid per the Marathon data format. The current mesh builders use fan triangulation assuming all `vertex_count` entries produce vertices, but then skip `-1` entries in the vertex emission loop. This mismatch causes the triangulation to reference non-existent vertex buffer indices.

Additionally, walls with `ShapeDescriptor = 0xFFFF` (no texture) are emitted as geometry but have no loaded texture, causing rendering artifacts or invisible geometry consuming draw calls.

The bug exists identically in `marathon-web/src/mesh.rs`, `marathon-game/src/mesh.rs`, and `marathon-viewer/src/mesh.rs` (shared code pattern, not shared code).

## Goals / Non-Goals

**Goals:**
- Fix floor/ceiling triangulation to correctly handle polygons with `-1` endpoint sentinel values
- Filter out wall geometry for sides with no texture (`0xFFFF`)
- Fix the bug in all three crate mesh builders
- Add tests that verify correct behavior with sparse endpoint arrays

**Non-Goals:**
- Refactoring the three mesh builders into a shared crate (desirable but separate scope)
- Fixing media surface generation (separate issue if needed)
- Optimizing triangulation algorithm (fan triangulation is correct for convex Marathon polygons)

## Decisions

### Decision 1: Track actual emitted vertex count separately from declared vertex_count

**Choice:** Introduce a local counter (`actual_verts`) that increments only when a vertex is actually emitted (i.e., endpoint index is valid and ≥ 0). Use `actual_verts` for the triangulation fan loop instead of `vert_count`.

**Alternative considered:** Pre-filter the endpoint array to remove `-1` entries before the loop — slightly cleaner but requires an allocation. The counter approach is zero-cost and minimal change.

**Rationale:** Smallest possible fix. The vertex emission loop already has the `continue` for invalid endpoints; we just need the triangulation loop to know the real count.

### Decision 2: Skip wall quad emission when texture is none

**Choice:** Add an `is_none()` check on the side's texture descriptor before calling `emit_wall_quad()` for each wall type (full, high, low, split sections).

**Alternative considered:** Emit the quad with a fallback texture — rejected because "no texture" in Marathon means "this surface is not visible," not "use default texture." Emitting invisible geometry wastes draw calls.

**Rationale:** Matches Marathon semantics. A side with no texture is intentionally invisible (e.g., the backside of a one-sided wall).

### Decision 3: Apply fix to all three mesh builders independently

**Choice:** Apply the same fix pattern to `marathon-web`, `marathon-game`, and `marathon-viewer` mesh.rs files separately.

**Alternative considered:** Extract a shared mesh-building crate — desirable but out of scope. The three files have diverged slightly (web uses `pad_layer_count_for_webgl`, game/viewer don't). A shared crate is a separate refactoring change.

**Rationale:** Fix the bug now in all locations. Deduplication is a separate improvement.

## Risks / Trade-offs

- [Over-filtering] Some polygons might have all `-1` endpoints (degenerate) → Mitigation: if `actual_verts < 3`, skip triangulation entirely. No triangles from degenerate polygons.
- [Winding order] Skipping vertices could change the winding of the fan triangulation → Mitigation: The fan base vertex is always the first valid vertex, and subsequent valid vertices maintain their relative order, so winding is preserved.
- [None-texture walls that should render] If any side has `0xFFFF` but was intended to show (format quirk) → Mitigation: Marathon format documentation confirms `0xFFFF` means no texture/invisible. Original engine skips these.
