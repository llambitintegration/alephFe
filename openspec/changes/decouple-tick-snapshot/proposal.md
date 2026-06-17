## Why

`marathon-sim` already runs the whole game correctly and rendering never reaches into its ECS — but there is **no single serializable render-state type**. Each frontend stitches a frame from ~10 separate `&mut self` accessor calls (`poly_dynamic_data`, `entities`, `player_position`, `player_weapon_state`, `drain_events`, …), so no frontend can be fed one serialized blob and the two existing clients have **drifted**: web drives all five per-polygon dynamic values through a clean data-texture path, while native still uses an older `snapshot()` + GPU-byte-offset path whose light update is a **dead no-op** (`marathon-game/src/render.rs:1282-1288`) — so native lights never animate. The long-blamed "static web mesh" bug is also narrower than thought: floors, ceilings, media, and lights are already dynamic in web; only **wall quad heights** are still baked-absolute (`marathon-web/src/mesh.rs:432-433,456-457,482-483`), so when a door moves its side walls don't stretch and gaps appear.

This is the keystone "Phase 0" refactor: it is a prerequisite for every later frontend (dashboard, MCP, headless server) and it closes the standing dynamic-geometry bug at the same time.

## What Changes

- Introduce a serializable `WorldSnapshot` render DTO and a pure `render_snapshot(&mut self) -> WorldSnapshot` aggregator on `SimWorld` that bundles the existing render DTOs (`PolyDynamicData`, `EntityRenderState`, `WeaponRenderState`) plus a new `PlayerView` and the events drained this frame. This is the only type-level churn (adding `serde` derives to the render DTOs).
- Migrate `marathon-web` to consume exactly one `render_snapshot()` per frame instead of the scattered accessor calls. Pixel-identical output.
- Migrate `marathon-game` (native) to consume `render_snapshot().poly_dynamic`, writing all five per-polygon fields **including the dead light field** — fixing frozen native lights as a side effect of decoupling.
- Fix the residual wall-height bug by driving wall quad top/bottom Y from the per-polygon data texture using the **same surface-discriminator trick floors/ceilings already use** (`marathon-web/src/shader.wgsl:71-80`), in both web and native. The vertex/index buffers stay immutable; only the data texture changes.
- Add a headless determinism harness test: tick a GPU-less `SimWorld` N times, serialize each `render_snapshot()`, assert deterministic bytes — proving the "any frontend consumes serialized state" goal without building a server.
- Leave `SimSnapshot` (`world.rs:867`, save/load full-ECS reconstruction) **untouched** — it is a different concern from the lossy per-frame render snapshot.

## Capabilities

### New Capabilities
- `render-snapshot`: A single serializable per-frame render DTO (`WorldSnapshot`) plus a pure read-only `render_snapshot()` aggregator over the existing sim accessors, the canonical interface every frontend consumes.
- `web-dynamic-walls`: Wall quad heights in `marathon-web` driven dynamically from the per-polygon data texture (matching floors/ceilings) so moving platforms/doors stretch their side walls instead of leaving gaps.
- `native-dynamic-geometry`: `marathon-game` consumes `render_snapshot().poly_dynamic` for all five per-polygon fields (including the previously dead light field) and applies the same wall-height discriminator, fixing frozen native lights and native wall gaps.
- `headless-tick`: A GPU-free path that ticks the sim and serializes `render_snapshot()` deterministically, validating that the engine runs headless and emits byte-stable render state.

### Modified Capabilities
<!-- None: this change adds new render-side capabilities and does not alter any existing spec-level requirement. SimSnapshot save/load behavior is unchanged. -->

## Impact

- `marathon-sim/src/world.rs`: add `WorldSnapshot`/`PlayerView`/`render_snapshot`; add `serde` derives to `PolyDynamicData`. `SimSnapshot` untouched.
- `marathon-sim/src/tick.rs`: add `serde` derives to `EntityRenderState`/`RenderEntityType`/`WeaponRenderState`; audit `SimEvent`.
- `marathon-web/src/render.rs` + `mesh.rs` + `shader.wgsl`: per-frame path (`render.rs:451-459`) consumes `render_snapshot()`; wall emission carries a height-source discriminator; vertex shader resolves wall Y from the data texture.
- `marathon-game/src/render.rs` + `mesh.rs` + `shader.wgsl`: replace the `snapshot()`/byte-offset update path (`render.rs:1250-1289`) with `render_snapshot().poly_dynamic` (all five fields incl. lights); apply the wall discriminator. The `size_of::<PolygonGpuData>() == 48` assert (`render.rs:1593`) remains the guardrail.
- New dev-dependency footprint only: `bincode` round-trip test (already proven in PR #8, commit `3fa7dad`).
- Out of scope: agent/dashboard/netcode layers (Phase A+), `SimSnapshot` save/load, making `MapGeometry` the geometry source-of-truth.
