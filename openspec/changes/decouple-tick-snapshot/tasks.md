## 1. Serde on render DTOs (step 1 — lowest risk, types only)

- [x] 1.1 Add `serde::Serialize`/`Deserialize` derives to `PolyDynamicData` (`marathon-sim/src/world.rs:374`); test that a `PolyDynamicData` value round-trips through bincode unchanged
- [x] 1.2 Add `serde` derives to `EntityRenderState` and `RenderEntityType` (`marathon-sim/src/tick.rs:2512,2522`); test bincode round-trip on a populated `EntityRenderState`
- [x] 1.3 Add `serde` derives to `WeaponRenderState` (`marathon-sim/src/tick.rs:2531`); test bincode round-trip
- [x] 1.4 Audit `SimEvent` for serializability (resolve design Open Question on `Entity`/handle fields); add `serde` derives (with a custom representation if a handle field requires it) and a round-trip test for each variant
- [x] 1.5 Confirm `marathon-sim` builds and its test suite is green with the new derives (no behavior change)

## 2. WorldSnapshot + render_snapshot aggregator (step 2)

- [x] 2.1 Define `PlayerView` (position, facing, vertical_look, polygon_index, health, shield, oxygen) with `serde` derives; unit test a bincode round-trip
- [x] 2.2 Define `WorldSnapshot` (tick_count, `Option<PlayerView>`, `poly_dynamic`, `entities`, `Option<WeaponRenderState>`, `events`) with `serde` derives; unit test a bincode round-trip on a `WorldSnapshot` built by hand
- [x] 2.3 Implement `render_snapshot(&mut self) -> WorldSnapshot` as a pure aggregator over `poly_dynamic_data` + `entities` + `player_*` + `player_weapon_state` + `drain_events`; test that each snapshot field equals the corresponding individual accessor after N ticks
- [x] 2.4 Test that `render_snapshot` is read-only: calling it twice without a tick yields the same `tick_count` and identical poly/entity data (no sim mutation)
- [x] 2.5 Test the no-player case: `render_snapshot` on a world with no player yields `player == None` and still produces the rest of the snapshot
- [x] 2.6 Confirm `SimSnapshot` save/load is untouched: existing `serialize`/`deserialize` round-trip test still passes

## 3. Web migration (step 3 — web lane, parallel with step 4 after step 2)

- [x] 3.1 Replace the scattered per-frame accessor calls in `marathon-web/src/render.rs` (`~245-265,309,451-459,467-473`) with a single `render_snapshot()` per frame, feeding the data-texture upload from `snapshot.poly_dynamic`
- [ ] 3.2 Assert the web vertex/index buffers are still never recreated in the frame path (buffer-stability invariant, `render.rs:447-450`); test that buffer handles are unchanged across frames
- [ ] 3.3 Web E2E visual-regression: a static scene renders pixel-equivalent (within tolerance) to pre-migration output
- [ ] 3.4 `wasm-pack build` for `marathon-web` succeeds; web tests green

## 4. Native migration + frozen-light fix (step 4 — native lane, riskiest)

- [ ] 4.1 Replace the `snapshot()` + byte-offset update path in `marathon-game/src/render.rs:1250-1289` with whole-`PolygonGpuData`-struct writes built from `render_snapshot().poly_dynamic` for all polygons (floor/ceiling/media height + floor/ceiling light)
- [ ] 4.2 Remove the dead light no-op stub (`render.rs:1282-1288`); confirm no `let _ = light;` or equivalent remains in the polygon-update path
- [ ] 4.3 Keep/verify the `size_of::<PolygonGpuData>() == 48` assertion (`render.rs:1593`) as the layout guardrail; test it holds
- [ ] 4.4 Headless/integration test: tick a sim with an animated light for N ticks and assert the native polygon buffer's floor/ceiling light entries change (lights no longer frozen)
- [ ] 4.5 Native binary builds and its render/integration tests are green

## 5. Wall-height fix, web (step 5 — web lane)

- [ ] 5.1 Resolve the neighbor-index design question: decide whether wall vertices need one extra `u32` (neighbor polygon) or can derive both source polygons from `polygon_index` + side topology; document the choice
- [ ] 5.2 In `marathon-web/src/mesh.rs` wall emission (`~432-433,456-457,482-483`), replace baked absolute top/bottom Y with a height-source discriminator + source polygon index (mirroring `SURFACE_FLOOR/CEILING/MEDIA`); test that wall vertices carry the discriminator and source index instead of absolute Y
- [ ] 5.3 In `marathon-web/src/shader.wgsl` `vs_main`, extend the discriminator branch (`~71-80`) to resolve wall Y from the data texture for the source polygon; verify the pipeline still builds under `downlevel_webgl2_defaults`
- [ ] 5.4 Web E2E on a moving-platform/door level (door-anim scenario): assert the wall quads bordering the moving polygon stretch with it and leave no gap, while vertex/index buffers stay immutable

## 6. Wall-height fix, native (step 6 — native lane)

- [ ] 6.1 Apply the same wall height-source discriminator in `marathon-game/src/mesh.rs`; test wall vertices carry the discriminator + source index
- [ ] 6.2 Extend `marathon-game/src/shader.wgsl` to resolve wall Y from the polygon buffer for the source polygon (consistent with the floor/ceiling read at `shader.wgsl:189`)
- [ ] 6.3 Native integration/visual check: a moving platform stretches its native walls with no gap, buffers immutable

## 7. Headless determinism harness (step 7)

- [x] 7.1 Add a headless test that constructs a `SimWorld` with no GPU, ticks N times with a fixed `TickInput` sequence, and calls `render_snapshot()` after each tick without initializing any graphics backend
- [x] 7.2 Serialize each frame's `render_snapshot()`; assert two runs with the same seed/level/input sequence produce byte-identical per-tick streams
- [x] 7.3 Assert calling `render_snapshot()` between ticks does not perturb the deterministic tick sequence vs. a reference run that omits the snapshot calls

## 8. Workspace verification

- [ ] 8.1 `cargo test` workspace-green in Docker (`rust:slim`); `cargo fmt --check` and `cargo clippy -D warnings` clean
- [ ] 8.2 Redeploy web and capture a moving-door screenshot via the live endpoint confirming walls stretch (closes the residual static-mesh bug end-to-end)
