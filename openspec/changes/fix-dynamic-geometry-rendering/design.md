## Context

The `mesh-generation` and `level-rendering` specs already describe the correct dynamic-geometry architecture: static vertex buffer + a per-polygon GPU resource (heights, light) that the shader reads and the frame loop updates each tick. `marathon-viewer` (native/desktop, wgpu Vulkan/Metal) implements this with a **storage buffer**.

`marathon-web` targets the browser via the wgpu **GL backend on WebGL2**, configured with `wgpu::Limits::downlevel_webgl2_defaults()`. WebGL2 has **no shader storage buffer objects** (SSBO is GLES 3.1 / WebGPU only). So the web renderer's authors took a shortcut: bake floor/ceiling height into absolute vertex `position.y` and bake light into a vertex `light` attribute at `build_level_mesh` time, upload once, and never update. This is why doors, platforms, elevators, and light switches are simulated correctly by `marathon-sim` but are invisible in the deployed game (see exploration findings; this is the root cause of "Space does nothing").

Separately, the player spawns with fists only (`world.rs:158-165`), so weapon firing is not observable, and `process_action_key` is level-triggered (fires every tick ACTION is held).

## Goals / Non-Goals

**Goals:**
- `marathon-web` reflects per-tick sim state for dynamic floor/ceiling/media heights and animated light intensities, within WebGL2 limits, without rebuilding the vertex/index buffers each frame.
- Bring `marathon-web`'s architecture into conformance with the existing `mesh-generation` "Platform geometry animation" and `level-rendering` "Per-polygon storage buffer" intent.
- Player spawns with a functional ranged weapon (magnum + fists) so firing is observable.
- Action-key activation is edge-triggered.

**Non-Goals:**
- No changes to `marathon-viewer` (already conformant).
- No new visual features (liquids semitransparency, faders, bloom) — those remain separate changes.
- No change to the simulation's platform/light/media algorithms — only how their output reaches the GPU.
- Not addressing the sprite-collection-index cap (collections 13/20/25) — separate concern.

## Decisions

### Decision 1: Per-polygon data **texture**, not storage or uniform buffer

WebGL2 cannot bind an SSBO, so the viewer's storage-buffer approach cannot be ported verbatim. Options considered:

- **A. Uniform-buffer array.** Rejected: WebGL2 guarantees only 64 KB max uniform block. At ~10 f32/polygon that caps at ~640 polygons; real Marathon levels exceed 1000 polygons. Hard ceiling, silent breakage on large maps.
- **B. Per-polygon data texture (chosen).** Encode per-polygon dynamic data (floor h, ceiling h, media h, floor light, ceiling light) into rows of an `Rgba32Float` texture; the vertex shader samples it by `polygon_index` to offset Y, the fragment shader samples it for the light multiplier. WebGL2 supports vertex-shader texture sampling and float textures (core in WebGL2). Scales to tens of thousands of polygons; updated each frame via `queue.write_texture` (or a small dirty-region write). Matches the spec's "vertices static, shader reads per-polygon" intent most closely.
- **C. Targeted vertex sub-buffer rewrite.** Only animated polygons (doors/platforms — a small subset) get their vertex Y/light rewritten each frame via `queue.write_buffer` into sub-ranges; shader unchanged. Simplest, no shader work, but diverges from the spec architecture, complicates the index/batch layout, and does not cover animated lighting on static-height polygons. Kept as a documented fallback if texture sampling in the vertex stage proves problematic on a target browser.

Chosen: **B**. It is the spec-aligned, scalable option and isolates the WebGL2 difference to "storage buffer (viewer) vs data texture (web)" while keeping identical shader semantics.

### Decision 2: Vertex carries `polygon_index`; light/height un-baked

`mesh::Vertex` gains a `polygon_index: u32` attribute. `build_level_mesh` stops writing absolute `position.y` from height and stops baking `light`; instead it emits geometry at a height-zero reference and lets the shader add the per-polygon height offset (consistent with viewer/`mesh-generation`). Texture descriptor and UVs are unchanged.

### Decision 3: Frame-loop sync point

In `GameState::frame()`, after the sim tick(s) and before `queue.submit`, gather per-polygon `(floor_h, ceiling_h, media_h, floor_light, ceiling_light)` from `SimWorld` (platforms by `polygon_index`, media by `media_index`, lights via the same `evaluate_light_intensity` path currently used at load) and `queue.write_texture` the packed buffer. Static polygons keep their initial values (written once); only the data texture changes, never vertex/index buffers.

### Decision 4: Starting loadout in `SimWorld::new`

Extend the starting `WeaponInventory` build (`world.rs:158`) to also insert the magnum slot with magazine/reserve sourced from its `WeaponDefinition`/`TriggerDefinition`, and equip it as `current`. Fists remain index 0. Driven by physics data so it stays scenario-correct rather than hard-coded counts.

### Decision 5: Edge-trigger via previous-ACTION state

Store the previous tick's ACTION flag (a `bool` resource or field on the sim). `process_action_key` proceeds only on a clear→set transition. Mirrors Marathon's original `ACTION_TRIGGER_TIME`/debounce behavior without porting its full timing model.

## Risks / Trade-offs

- **Vertex-shader texture sampling unsupported/slow on some WebGL2 stacks** → Mitigation: float-texture + vertexTextureImageUnits are WebGL2 core; verify on the SwiftShader CI path (already the e2e environment) and on a real GPU; Decision 1 option C is the documented fallback.
- **Float texture precision / packing layout bugs** → Mitigation: spec scenarios are written as concrete tests (elevator 0→1024, pulsating light); add a unit test asserting the packed texel for a known polygon.
- **Re-baking removal breaks existing `marathon-web` visual-regression baselines** → Mitigation: regenerate baselines deliberately as part of the change; static-scene appearance must match pre-change (heights/light identical when nothing animates).
- **BREAKING: tests asserting single starting weapon** → Mitigation: update those tests as part of the combat-system delta; the `combat-system` spec change makes the new contract explicit.
- **Edge-trigger could drop an activation if input sampling races tick boundaries** → Mitigation: edge detection is computed inside the sim tick on the authoritative `TickInput`, not in the browser input layer.

## Open Questions

- Exact texel packing: one RGBA32F texel per polygon (4 floats) is insufficient for 5+ values — use 2 texels/polygon (row-major, width = 2·N or a 2-wide texture) or an `R32Float` Nx5 layout? Resolve during task 1.
- Should media height live in the same data texture or a separate small one, given media polygons are sparse? Default: same texture, sparse rows unused.
- Whether to also un-bake transfer mode now or leave it baked (it is static per polygon today) — default: leave transfer mode baked, out of scope.
