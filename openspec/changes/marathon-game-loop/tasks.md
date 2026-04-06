## 1. Crate Scaffolding and Shared Rendering Extraction

- [x] 1.1 Create `marathon-game` crate directory with `Cargo.toml` and empty `src/main.rs`, add to workspace members
- [x] 1.2 Add dependencies: `marathon-formats`, `marathon-sim`, `marathon-audio`, `marathon-integration`, `wgpu`, `winit`, `clap`, `env_logger`, `glam`
- [x] 1.3 Implement CLI argument parsing (--map, --shapes, --sounds, --level) with clap in `main.rs`
- [x] 1.4 Copy GPU initialization, texture pipeline, mesh generation, and level loading into marathon-game (shared extraction deferred to avoid wgpu 22/24 version conflict with viewer)
- [x] 1.5 Copy texture pipeline (texture array building from Shapes bitmaps, bind group creation) into marathon-game
- [x] 1.6 Copy mesh generation and per-polygon storage buffer management into marathon-game
- [ ] 1.7 Refactor `marathon-viewer` to consume the shared rendering module instead of its own copy (deferred — viewer continues working as-is)

## 2. First-Person Camera and Level Geometry Rendering

- [x] 2.1 Implement first-person camera struct that takes player position, facing yaw, look pitch, and eye-height offset, producing a view-projection matrix
- [x] 2.2 Wire the first-person camera into the render pipeline (replacing the viewer's free-fly camera for the game binary)
- [x] 2.3 Implement camera interpolation: store previous-tick and current-tick camera state, lerp by alpha factor each render frame
- [x] 2.4 Render level geometry using the shared pipeline with the first-person camera — verify a level is visible from the player spawn position

## 3. Simulation Integration and Game Loop

- [x] 3.1 Initialize `SimWorld` from parsed map data, physics data, and a seed in the game binary
- [x] 3.2 Implement the fixed-timestep tick accumulator: track elapsed time, run sim ticks at 30Hz, compute interpolation alpha
- [x] 3.3 Wire input capture (winit keyboard + mouse events) through the input system to produce `ActionFlags` each tick
- [x] 3.4 Feed `ActionFlags` into `sim.tick()` each simulation step
- [x] 3.5 After each tick, read player state (position, facing, pitch) and update the camera
- [x] 3.6 After each tick, read platform heights and media heights from sim and update the per-polygon storage buffer
- [ ] 3.7 Verify the player can move through a level with WASD + mouse look, colliding with walls and stepping over ledges

## 4. Entity State Collection and Snapshot System

- [x] 4.1 Define a `RenderableEntity` struct (id, position, facing, entity_type, collection, sequence, frame)
- [x] 4.2 After each tick, query `sim.entities()` and collect into a `Vec<RenderableEntity>` as the current-tick snapshot
- [x] 4.3 Implement double-buffered snapshots: swap current to previous at each tick, store new current
- [x] 4.4 Implement entity interpolation: for each entity present in both snapshots, lerp position by alpha; entities only in current render at current position; entities only in previous are skipped

## 5. Entity Sprite Rendering

- [x] 5.1 Build a sprite texture atlas/array from Shapes collections (extract bitmaps for monster, item, projectile, scenery collections)
- [x] 5.2 Create the sprite render pipeline: vertex shader for billboarded quads (camera-facing), fragment shader with alpha test, depth test using shared depth buffer
- [x] 5.3 Implement single-angle sprite rendering for items and projectiles (billboard at entity position, select frame from collection/sequence/frame)
- [x] 5.4 Implement multi-angle sprite selection for monsters: compute relative angle between camera and monster facing, select from up to 8 views
- [x] 5.5 Implement scenery object rendering: read map object placements, render static sprites at map-defined positions
- [x] 5.6 Implement effect/explosion sprite rendering with frame advancement based on effect age
- [ ] 5.7 Verify entities are visible in-game: monsters, items, and scenery render at correct positions with correct occlusion

## 6. Audio Integration

- [x] 6.1 Initialize `AudioEngine` from sounds WAD data in the game binary (with graceful fallback if no audio device)
- [x] 6.2 After each tick, query `sim.pending_audio_events()` and dispatch to the audio engine as one-shot spatial sounds
- [x] 6.3 Update the audio listener position to match the player's position and facing each tick
- [x] 6.4 Wire ambient sound definitions from the map to the audio engine's ambient loop system
- [ ] 6.5 Verify sounds play spatially: weapon fire, monster alerts, door sounds positioned correctly in 3D

## 7. Shell State Machine Wiring

- [x] 7.1 Instantiate the shell state machine in the game binary, starting in Loading state
- [x] 7.2 Implement the Loading → Playing transition: after subsystem init completes, transition to Playing
- [x] 7.3 Implement Playing → Paused → Playing transitions: Escape key pauses (stops sim ticks, continues rendering), Escape again resumes
- [x] 7.4 Implement level transition: when sim signals a level teleport event, transition through Loading to load the target level and resume Playing
- [ ] 7.5 Verify basic flow: launch binary → level loads → gameplay → pause → resume → level teleport → next level loads

## 8. Docker and CI Integration

- [x] 8.1 Add `marathon-game` to the Dockerfile build and test stages (already included as workspace member)
- [x] 8.2 Add headless integration tests: scenario loading, sim initialization, tick + entity state collection (no GPU required)
- [x] 8.3 Verify `cargo test` passes for `marathon-game` in the Docker CI pipeline
- [x] 8.4 Update the Dockerfile to also build the `marathon-game` binary in a release stage for distribution
