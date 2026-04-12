## Why

Wall and floor geometry in the web renderer has visible gaps — missing polygons and walls that should be solid. The root cause is a vertex index mismatch in floor/ceiling triangulation: when Marathon map polygons have `-1` endpoint sentinel values (valid per the format), the vertex loop skips those entries but the triangulation fan still assumes all `vert_count` vertices were emitted, referencing non-existent indices. A secondary issue is that walls with `ShapeDescriptor = 0xFFFF` (no texture) are emitted as geometry but have no loaded texture, causing rendering artifacts.

## What Changes

- Fix floor/ceiling triangulation to track actual emitted vertex count and triangulate based on that, not the declared `vert_count`
- Skip wall quad emission when the side's texture descriptor is `0xFFFF` (none)
- Apply the same triangulation fix in `marathon-game` and `marathon-viewer` mesh builders (shared bug)
- Add unit tests verifying correct triangulation with `-1` endpoint gaps and correct wall filtering for none-texture sides

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities

- `mesh-generation`: Floor/ceiling triangulation handles sparse endpoint arrays; wall generation filters none-texture sides

## Impact

- `marathon-web/src/mesh.rs` — Primary fix site for floor/ceiling fan and wall texture filtering
- `marathon-game/src/mesh.rs` — Same triangulation bug (shared code pattern)
- `marathon-viewer/src/mesh.rs` — Same triangulation bug (shared code pattern)
- No API changes — mesh vertex/index buffers are internal
- Visual correctness improvement: previously missing floors, ceilings, and walls will render
