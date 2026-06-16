## Context

`marathon-sim` is already mostly decoupled from rendering: the ECS `world` field is `pub(crate)` (`world.rs:120`) and no renderer reaches into it. The sim exposes a clean, web-free set of render accessors — `tick(TickInput)` (`tick.rs:102`), `entities()` (`tick.rs:2424`), the `player_*` getters (`tick.rs:2246-2305`), `player_weapon_state()` (`tick.rs:2305`), `poly_dynamic_data()` (`world.rs:304`), and `drain_events()` (`world.rs:277`). The render DTOs `EntityRenderState`/`RenderEntityType` (`tick.rs:2512,2522`), `WeaponRenderState` (`tick.rs:2531`), and `PolyDynamicData` (`world.rs:374`) are already clean.

Three problems remain, and they are *interface shape*, not encapsulation:

1. **No single render-snapshot type.** Each frontend rebuilds a frame from ~10 `&mut self` calls; nothing serializable can be handed to a non-rendering consumer.
2. **The two clients have drifted.** Web pushes all five per-polygon dynamic values (floor/ceiling/media height + floor/ceiling light) through a per-poly **data texture** updated every frame from `poly_dynamic_data()` (`render.rs:451-459`). Native uses an older `snapshot()` + GPU-byte-offset path (`render.rs:1250-1289`) whose **light update is a dead no-op** (`render.rs:1282-1288`, literally `let _ = light;`) — native lights are frozen.
3. **The residual mesh bug is narrow.** Floors/ceilings/media are already dynamic in web (data texture; surface discriminators in `shader.wgsl:71-80`). Only **wall quad heights** are still baked absolute into the static vertex buffer (`mesh.rs:432-433,456-457,482-483`). When a platform/door moves, its floors/ceilings track it but its side walls don't stretch → visible gaps.

Constraint: web targets the wgpu GL backend on WebGL2 (`downlevel_webgl2_defaults`), which has no SSBO — hence the existing data-texture path, which this change reuses rather than replaces.

## Goals / Non-Goals

**Goals:**
- One serializable `WorldSnapshot` DTO + a pure `render_snapshot(&mut self) -> WorldSnapshot` aggregator that both frontends consume.
- `render_snapshot` is a thin read-only aggregator over the existing accessors — reuse, do not reinvent.
- Both frontends consume only `render_snapshot()`; web stays pixel-identical, native gets its frozen lights fixed.
- Close the wall-height gap in both clients via the existing data-texture discriminator pattern, with the vertex/index buffers staying immutable.
- A headless determinism test proves the sim ticks + serializes render state with no GPU.

**Non-Goals:**
- No agent/dashboard/MCP/netcode layers (those are Phase A and later — keep their scope out).
- No change to `SimSnapshot` (`world.rs:867`), the save/load full-ECS snapshot. It is a different concern; conflating it bloats the per-frame path with physics state the renderer doesn't need.
- No change to the sim's platform/light/media/monster algorithms — only how their output reaches the GPU.
- Not making the sim's parallel `MapGeometry` collision copy the geometry source-of-truth (later phase). Static geometry (vertices, topology, texture descriptors, transfer modes, index maps) keeps flowing from `MapData` at load.

## Decisions

### Decision 1: `WorldSnapshot` aggregates existing DTOs; only `PlayerView` is new

```rust
// new module, e.g. marathon-sim/src/render_snapshot.rs (or in world.rs)
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WorldSnapshot {
    pub tick_count: u64,
    pub player:   Option<PlayerView>,        // camera + HUD source
    pub poly_dynamic: Vec<PolyDynamicData>,  // existing, world.rs:374
    pub entities: Vec<EntityRenderState>,    // existing, tick.rs:2512
    pub weapon:   Option<WeaponRenderState>, // existing, tick.rs:2531
    pub events:   Vec<SimEvent>,             // drained this frame
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PlayerView {
    pub position: glam::Vec3,
    pub facing: f32,
    pub vertical_look: f32,
    pub polygon_index: usize,
    pub health: i16,
    pub shield: i16,
    pub oxygen: i16,
}

pub fn render_snapshot(&mut self) -> WorldSnapshot;  // &mut self: bevy QueryState caching
```

`render_snapshot` calls the existing accessors once each and packs the results. `&mut self` is required because bevy caches `QueryState` in `&mut World` — but `render_snapshot` *reduces* borrow pressure (1 borrow vs ~10) and is read-only over the ECS, so it cannot perturb sim state. Rejected: a `&self` snapshot would need interior mutability — out of scope.

### Decision 2: Serde on the render DTOs is the only type-level churn

Derive `Serialize`/`Deserialize` on `WorldSnapshot`, `PlayerView`, `PolyDynamicData`, `EntityRenderState`/`RenderEntityType`, `WeaponRenderState`, and audit `SimEvent`. The PR #8 bincode round-trip (commit `3fa7dad`) already shows these derives compose. No behavioral change.

### Decision 3: Migrate web first, then native — parallel lanes after the aggregator lands

After `render_snapshot()` exists (step 2), web (steps 3, 5) and native (steps 4, 6) are **independent parallel lanes** that touch disjoint crates. Web is migrated first because it is already on the clean data-texture path, so the migration is a pure call-site collapse with a pixel-identical result — the lowest-risk way to validate the aggregator end-to-end behind the existing E2E visual-regression gate (PR #2 WIG gate).

### Decision 4: Wall heights via the existing surface-discriminator trick, not a vertex rebuild

Floors/ceilings already encode a surface discriminator (`SURFACE_FLOOR=0/CEILING=1/MEDIA=2`) in `position.y` and resolve the real height from `poly_texel0(polygon_index)` in `vs_main` (`shader.wgsl:71-80`). Extend the same pattern to walls:

1. In `build_level_mesh` wall emission (`mesh.rs:432-433` etc.), replace the baked absolute top/bottom Y with a height-source discriminator: tag each wall vertex "Y comes from polygon P's floor/ceiling" or "neighbor P's floor/ceiling," storing the source polygon index in the existing `polygon_index` attribute (`mesh.rs:24`) plus a second index for the neighbor case.
2. In `vs_main`, extend the discriminator branch to resolve wall Y from the data texture for the source polygon instead of `in.position.y`.

This is the same validated pattern as floors/ceilings → low risk; the per-frame upload path is unchanged. **Do NOT rebuild the vertex buffer per frame** — that breaks the buffer-stability invariant (`render.rs:447-450`) and re-bakes geometry every frame. Native already has a `polygon_buffer` with `floor_light`/`ceiling_light` fields and a shader reading `poly.floor_light` (`game/src/shader.wgsl:189`), so the same trick ports.

### Decision 5: Native light fix falls out of the migration

Native's frozen lights are fixed by replacing the byte-offset update path (`render.rs:1250-1289`) with whole-struct writes of `render_snapshot().poly_dynamic`, which carries `floor_light`/`ceiling_light` for every polygon. Prefer whole-`PolygonGpuData`-struct writes over hand-computed byte offsets; the `size_of::<PolygonGpuData>() == 48` assert (`render.rs:1593`) is the guardrail that catches layout drift.

### Decision 6: `SimSnapshot` stays separate

`SimSnapshot` (`world.rs:867`) is the lossy-free save/load snapshot whose `rng_seed` is *derived* from `tick_count` (`world.rs:1035`), so save/load is not bit-identical — irrelevant to `WorldSnapshot`, which has no RNG in the render path. Keep them separate types.

## Risks / Trade-offs

- **Native migration is the riskiest step (live native loop + GPU byte-offset writes + behavior change).** The light fix may surface latent light bugs that were masked by the dead stub → Mitigation: prefer whole-struct `PolygonGpuData` writes over hand offsets; lean on the `size_of == 48` assert (`render.rs:1593`); migrate native only after the aggregator and web lane are green.
- **Wall discriminator overloads vertex attributes; a packing mistake silently misplaces walls** → Mitigation: reuse the exact floor/ceiling discriminator constants; gate on the E2E visual-regression test on a moving-platform level (door-anim scenario, PR #3 box 4.3); honor the `look_to_lh`/`perspective_lh` invariant (project memory) so Y/orientation stays correct.
- **Per-frame serialization/clone cost** — `WorldSnapshot` clones `poly_dynamic` + entities per frame; web already pays this. Step 4 makes native clone the full per-poly set each tick (vs. only-changed today) → Mitigation: acceptable at level scale; `bytemuck`/`Cow` optimization is a later concern; the server pays bincode only when actually serializing.
- **Removing baked wall Y could shift web visual-regression baselines** → Mitigation: a fully static scene must be pixel-equivalent to pre-change (heights identical when nothing animates); regenerate baselines deliberately only if a static-scene diff is confirmed expected.
- **Determinism** — tick order is fixed (`tick.rs:106-129`) and RNG is seeded (`world.rs:138`); `render_snapshot()` is read-only over the ECS so cannot perturb it. The headless test (step 7) is the regression sentinel.

## Migration Plan

Seven steps, each independently green and buildable. Steps 1–4 = decoupling + native-light fix; 5–6 = wall fix; 7 = proves headless. After step 2, web (3, 5) and native (4, 6) are independent parallel lanes.

1. **Add serde to render DTOs** — `PolyDynamicData`, `EntityRenderState`/`RenderEntityType`, `WeaponRenderState`, new `PlayerView`; audit `SimEvent`. Test: bincode round-trip (copy PR #8). *Lowest risk; types only.*
2. **Introduce `WorldSnapshot` + `render_snapshot()`** as a pure aggregator. No frontend changes. Test: snapshot fields equal individual accessors after N ticks.
3. **Migrate web** to one `render_snapshot()`/frame (replace `render.rs:245-265,309,451-459,467-473`). Pixel-identical. Test: web E2E visual-regression. Web only.
4. **Migrate native + fix native lights** — replace `render.rs:1250-1289` with `render_snapshot().poly_dynamic`, writing all five per-poly fields incl. lights. Native only. *Riskiest step.*
5. **Wall-height fix, web** — discriminator in `marathon-web/src/mesh.rs` + `shader.wgsl`. Test: web E2E on a moving-platform level.
6. **Wall-height fix, native** — same trick in `marathon-game/src/mesh.rs` + `shader.wgsl`.
7. **Headless harness test** — construct `SimWorld`, tick N times with no GPU, serialize each `render_snapshot()`, assert deterministic bytes.

Rollback: each step is an isolated, independently-green commit on disjoint surfaces; reverting any single step leaves the rest compiling. The aggregator (step 2) is additive — frontends keep working on the old accessors until their migration step lands.

## Open Questions

- **`SimEvent` serde** — does every variant already serialize cleanly, or does an entity-handle/`Entity` field need a custom representation? Resolve in step 1.
- **Neighbor index for walls** — does a wall vertex need one extra `u32` attribute for the neighbor polygon, or can both source polygons be derived from the existing `polygon_index` + side topology? Resolve in step 5.
- **Whether `events` belongs in `WorldSnapshot`** — including drained events makes the snapshot the single frame interface, but events are consumed (not a pure read-model). Default: include them; the headless test asserts determinism with events present.
