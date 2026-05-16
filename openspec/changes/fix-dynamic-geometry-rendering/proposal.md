## Why

Marathon is a game about moving geometry — doors, elevators, platforms, crushers — and dynamic lighting. The simulation (`marathon-sim`) computes all of this correctly every tick, but the deployed web renderer (`marathon-web`) builds the level mesh **once** at load (`build_level_mesh` → static vertex/index buffers, lighting baked into vertices) and never reflects sim state changes afterward. The result: pressing the action key opens a door in the sim but nothing moves on screen, light switches toggle a light the player can never see change, and the world feels dead. Additionally, the player spawns with only fists (`world.rs:158-165`), so weapon firing produces no visible result. These are the two concrete reasons interaction "does nothing" for a player today.

The `mesh-generation` and `level-rendering` specs already describe the correct architecture (static vertices + per-polygon storage buffer + shader-side height offset + per-frame animation update). `marathon-viewer` implements it; `marathon-web` does not — it regressed to a baked static mesh, most likely as a WebGL2 storage-buffer compatibility shortcut.

## What Changes

- Introduce a WebGL2-compatible per-polygon dynamic data path in `marathon-web` so floor/ceiling heights, media heights, and light intensities are updated every frame from `SimWorld` state instead of baked at load.
- The level mesh vertices remain static; the vertex/fragment shader reads current per-polygon height and light values from a uniform/storage resource and offsets geometry and shades accordingly (mirroring the existing `mesh-generation` "Platform geometry animation" requirement).
- The `marathon-web` frame loop SHALL push updated per-polygon animation state (platforms, media, animated lights) to the GPU each frame.
- Grant the player a functional starting loadout (fists **and** the magnum pistol) at spawn so weapon firing is demonstrably observable. **BREAKING** for any test asserting the player starts with exactly one weapon.
- Edge-trigger the action key (activate on key-down transition, not every tick it is held) so a door does not re-toggle every tick once geometry is visibly dynamic.

## Capabilities

### New Capabilities
- `web-dynamic-geometry`: The WebGL2/WASM-compatible mechanism by which `marathon-web` keeps rendered floor/ceiling/media heights and light intensities in sync with per-tick `SimWorld` state without rebuilding the vertex buffer.

### Modified Capabilities
- `combat-system`: The player's initial weapon inventory changes from fists-only to fists + magnum, with starting ammunition, so firing is functional from spawn.
- `world-mechanics`: Action-key activation becomes edge-triggered (once per press) rather than re-triggering every tick the key is held.

## Impact

- `marathon-web/src/render.rs`: frame loop must update per-polygon GPU data each frame; `load_level_into` stops baking light/height into vertices.
- `marathon-web/src/mesh.rs`: `Vertex` carries a polygon index; per-polygon dynamic data moves to a GPU buffer; lighting no longer baked.
- `marathon-web/src/shader.wgsl`: vertex shader applies per-polygon height offset; fragment shader reads per-polygon light multiplier. Must stay within `downlevel_webgl2_defaults` (uniform buffer or supported storage buffer per existing WebGL2 compat work, commit 50a30d4).
- `marathon-sim/src/world.rs`: starting `WeaponInventory` gains the magnum + ammo.
- `marathon-sim/src/tick.rs`: `process_action_key` gains key-down edge detection (track previous-tick ACTION flag).
- No changes to `marathon-viewer` (already conformant); `marathon-formats` unaffected.
- Closes the long-blocked `fix-weapon-sprites-interactions` task 5.4 (verify door opens / light switch toggles), which cannot pass against a static mesh.
