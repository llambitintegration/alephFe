## 1. Per-polygon data texture infrastructure (marathon-web)

- [x] 1.1 Decide and document the texel packing layout (resolve design Open Question 1): define a `PolyDynData` struct (floor_h, ceiling_h, media_h, floor_light, ceiling_light) and a function `pack_poly_data(&[PolyDynData]) -> Vec<f32>` with a unit test asserting the packed offsets for a known 2-polygon input
- [ ] 1.2 Add a `Rgba32Float` (or chosen format) data texture + sampler + bind group layout entry sized for `map.polygons.len()`, created in `run_web`/`load_level_into`; unit/integration test asserting texture dimensions match polygon count under `downlevel_webgl2_defaults` limits
- [ ] 1.3 Add `write_poly_data_texture(queue, &[PolyDynData])` helper that uploads the packed buffer via `queue.write_texture`; test that a round-trip pack→(layout)→unpack yields the input values

## 2. Mesh / vertex changes (marathon-web)

- [ ] 2.1 Add `polygon_index: u32` to `mesh::Vertex` and its `layout()`; update the vertex WGSL input struct; test that `build_level_mesh` assigns each emitted vertex the polygon index of its source polygon
- [ ] 2.2 Stop baking height into `position.y` and light into `light` in `build_floor`/`build_ceiling`/`build_media_surface`; emit geometry at the height-zero reference; test that two polygons with different `floor_height` now produce identical vertex Y and differ only by `polygon_index`
- [ ] 2.3 Populate the initial `PolyDynData` array at load from `evaluate_light_intensity` + polygon floor/ceiling/media heights; test that initial packed data reproduces the pre-change baked values for a sample level

## 3. Shader changes (marathon-web)

- [ ] 3.1 In `shader.wgsl` vertex stage, sample the data texture by `polygon_index` and add the per-polygon floor/ceiling/media height offset to vertex Y (selecting floor vs ceiling vs media by an existing surface discriminator); add a shader-compile test in the web test suite
- [ ] 3.2 In `shader.wgsl` fragment stage, replace the baked `light` attribute usage with the per-polygon light sampled from the data texture; verify the render pipeline still builds under WebGL2 limits

## 4. Frame-loop synchronization (marathon-web)

- [ ] 4.1 Add a method on `SimWorld` (or reuse existing accessors) returning current per-polygon floor/ceiling heights, media heights, and animated light intensities for all polygons; unit test against a sim with one moving platform
- [ ] 4.2 In `GameState::frame()`, after sim ticks and before `queue.submit`, gather per-polygon data and call `write_poly_data_texture`; assert vertex/index buffers are never recreated in `frame()` (code review checkbox + test that buffer handles are unchanged across frames)
- [ ] 4.3 Integration test (headless wgpu): tick a sim with an opening door for N ticks and assert the data-texture entry for the door polygon changes while the vertex buffer contents do not

## 5. Starting weapon loadout (marathon-sim)

- [ ] 5.1 Write a failing test: a freshly created `SimWorld` has a weapon inventory containing fists and the magnum, magnum equipped with full primary magazine and positive reserve, `current()` resolves to a weapon with `projectile_type >= 0`
- [ ] 5.2 Extend the starting `WeaponInventory` build in `SimWorld::new` (`world.rs:158`) to insert the magnum slot with magazine/reserve sourced from physics `WeaponDefinition`/`TriggerDefinition`; make 5.1 pass
- [ ] 5.3 Write a test: with the starting loadout, a `FIRE_PRIMARY` tick spawns a projectile entity and decrements the magnum primary magazine by 1; make it pass
- [ ] 5.4 Update any existing tests that assumed a fists-only starting inventory (combat-system delta is now authoritative)

## 6. Edge-triggered action key (marathon-sim)

- [ ] 6.1 Write a failing test: holding ACTION for 5 consecutive ticks facing a door activates the platform exactly once
- [ ] 6.2 Add previous-ACTION state to the sim and gate `process_action_key` on a clear→set transition; make 6.1 pass
- [ ] 6.3 Write a test: release then re-press ACTION re-activates (second activation occurs); make it pass

## 7. Verification & regression

- [ ] 7.1 Regenerate `marathon-web` visual-regression baselines; assert a fully static scene is pixel-equivalent (within tolerance) to pre-change output
- [ ] 7.2 Add an e2e test (Playwright, against the proxy-net container): load a level, walk to a door, press the action key, screenshot before/after, assert the door-region pixels change
- [ ] 7.3 Functionally verify and tick `fix-weapon-sprites-interactions` task 5.4 (door opens; light switch toggles a visible light) — record evidence
- [ ] 7.4 `cargo test` workspace-green in Docker (`rust:slim`); `wasm-pack build` for `marathon-web` succeeds; redeploy and capture a moving-door screenshot via the live endpoint
