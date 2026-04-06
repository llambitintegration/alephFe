## Context

The alephone-rust project has five crates (~14,500 LoC) implementing Marathon's core subsystems: format parsing, simulation, audio, integration (HUD/menus/input/shell), and a 3D level viewer. Every subsystem is implemented and tested in isolation, but no binary wires them together into a playable game. The only runnable binary is `marathon-viewer`, a free-fly camera level viewer using wgpu + winit.

The `marathon-integration` crate defines a complete shell state machine, HUD rendering pipeline, menu structures, input system, and terminal interface — none of which are invoked from a `main()` function. The `marathon-sim` crate runs a deterministic 30Hz ECS simulation with player physics, monster AI, and combat. The `marathon-audio` crate provides spatial sound via Kira.

The goal is a new binary crate that wires everything together so a user can load Marathon 2 scenario files and play through levels.

## Goals / Non-Goals

**Goals:**
- Create a playable single-player game binary that loads Marathon 2 scenarios
- First-person rendering from the player's perspective (not free-fly camera)
- Monsters, items, projectiles, and scenery objects visible as sprites in the 3D world
- Input drives simulation, simulation drives rendering and audio
- Smooth rendering decoupled from the 30Hz tick rate via interpolation
- Testable via Docker (headless CI for non-GPU paths, scenario loading, sim integration)

**Non-Goals:**
- Multiplayer networking (game modes exist in sim, but network transport is out of scope)
- WASM compilation (future work — get native working first)
- Full menu system with preferences UI (can start with a hardcoded "load level 0" path)
- Film recording/playback (the sim supports it, but the binary doesn't need it yet)
- Save/load (same — sim serializes, but the binary can defer persistent storage)
- HUD rendering (can be added incrementally after the core loop works)

## Decisions

### 1. New `marathon-game` crate rather than extending `marathon-viewer`

**Decision:** Create a new workspace member `marathon-game` with its own `main.rs`.

**Rationale:** The viewer is a standalone debugging/exploration tool with a fundamentally different architecture (free-fly camera, no simulation, no audio). Trying to bolt the game loop onto it would entangle two different use cases. The viewer remains useful as-is for level designers and debugging.

**Alternative considered:** Extending `marathon-viewer` with a `--play` flag. Rejected because the event loop structure, camera system, and frame logic are completely different between viewing and playing.

### 2. Shared rendering foundation extracted from marathon-viewer

**Decision:** Extract GPU initialization, texture pipeline, mesh generation, and the per-polygon storage buffer from `marathon-viewer` into a shared module within `marathon-integration` (or a new `marathon-render` crate if the code volume warrants it). Both `marathon-viewer` and `marathon-game` consume this shared rendering layer.

**Rationale:** The viewer's render pipeline (wgpu device setup, texture array management, mesh generation from map geometry, per-polygon storage buffer, lighting evaluation) is exactly what the game needs for level geometry. Duplicating ~900 lines of GPU code would be a maintenance burden.

**Alternative considered:** Having `marathon-game` depend directly on `marathon-viewer` as a library. Rejected because the viewer's `render.rs` is tightly coupled to its own event loop and camera, making it hard to reuse cleanly.

### 3. Entity sprites rendered as billboarded quads in a second render pass

**Decision:** Render entity sprites (monsters, items, projectiles, effects) as camera-facing billboarded quads in a separate render pass after level geometry, sharing the same depth buffer. Sprites are sourced from the Shapes data (collections/sequences/frames) already parsed by `marathon-formats`.

**Rationale:** Marathon's original renderer draws sprites after world geometry using a painter's algorithm with depth clipping. Using the GPU depth buffer achieves the same occlusion without manual sorting. A separate pass keeps the sprite vertex format and shader simple (position + UV + tint) without complicating the level geometry pipeline.

**Alternative considered:** Interleaving sprites into the level geometry render pass. Rejected because sprites need alpha blending while level geometry is opaque, requiring different pipeline state.

### 4. Double-buffered simulation state for interpolation

**Decision:** Maintain two snapshots of renderable state: `previous_tick` and `current_tick`. Each frame, compute an interpolation factor `alpha = accumulator / tick_duration` and lerp entity positions between the two snapshots. The camera (player position/facing) is also interpolated.

**Rationale:** The simulation runs at fixed 30Hz. Without interpolation, rendering at 60Hz+ would show stuttery movement. Double-buffering is the standard approach for fixed-timestep games (cf. Glenn Fiedler's "Fix Your Timestep").

**Alternative considered:** Extrapolation (predicting forward from current state). Rejected because it can overshoot and requires correction, adding complexity for little benefit at 30Hz → 60Hz ratios.

### 5. First-person camera derived from simulation player state

**Decision:** Each tick, read the player's position (Vec3), facing angle (yaw), and look angle (pitch) from `marathon-sim`. Construct the view matrix from these values. The camera FOV matches Marathon's original (90° horizontal equivalent). Vertical look is clamped to Marathon's original range.

**Rationale:** The simulation owns the player's position and facing — it accounts for physics, collision, and movement. The renderer just consumes it. This keeps the boundary clean: input → sim → camera, with no rendering code influencing player position.

### 6. Audio events dispatched from simulation tick results

**Decision:** After each `sim.tick()`, query `sim.pending_audio_events()` for a list of (sound_id, position, type) tuples. Pass these to `marathon-audio`'s engine as one-shot spatial sounds. Ambient sounds and music are managed by the shell state machine based on the current level's ambient sound definitions.

**Rationale:** The simulation already knows when weapons fire, monsters take damage, doors open, etc. Exposing these as audio events keeps the sim → audio coupling minimal and the audio engine purely reactive.

**Alternative considered:** Having the sim directly call audio. Rejected because it would add a runtime dependency from `marathon-sim` to `marathon-audio`, breaking the clean library boundary and making headless testing harder.

### 7. Minimal initial shell: skip menus, go straight to gameplay

**Decision:** For the initial implementation, the binary loads a hardcoded level (level 0 or CLI-specified) and transitions directly to `Playing` state. The main menu, preferences, save/load, and film systems are wired later. The shell state machine is instantiated but only exercises `Loading → Playing` and `Playing → Paused → Playing`.

**Rationale:** The fastest path to "test playing the game" skips the menu system. All the menu and save/load infrastructure exists in `marathon-integration` and can be wired in incrementally once the core gameplay loop is solid.

## Risks / Trade-offs

**[Rendering extraction complexity]** → Extracting shared rendering code from `marathon-viewer` may require significant refactoring of `render.rs` (923 lines). Mitigation: Start by copying the relevant pieces into the game binary, then refactor into a shared module once both binaries work.

**[Sprite rendering performance]** → Marathon levels can have dozens of visible entities. Billboarded quads with individual draw calls could be slow. Mitigation: Instance the sprite draws into a single buffer. Marathon's entity counts are small enough (~50 max visible) that even naive approaches should work at 60fps.

**[Audio backend on CI]** → Kira/cpal requires an audio device. Docker CI environments may not have one. Mitigation: Gate audio initialization behind a feature flag or runtime check; the sim and rendering can be tested headless without audio.

**[Shapes data complexity]** → Marathon sprite rendering requires resolving collection → sequence → frame → bitmap lookups, plus 8-angle rotation for monsters. The format parsing exists but the rendering-side lookup may have edge cases. Mitigation: Start with single-angle sprites (items, projectiles) before tackling multi-angle monsters.

**[Missing entity query API]** → The game-loop spec defines `entities()` returning positions and shape descriptors, but the actual implementation may need extension to include animation state, facing relative to viewer, and active transfer modes. Mitigation: Extend the query API as needed during implementation.

## Open Questions

- Should the shared rendering code live in `marathon-integration` (which already depends on wgpu) or in a new `marathon-render` crate? Depends on code volume after extraction.
- What's the right approach for the Shapes texture atlas — one large atlas per collection, or a texture array indexed by frame? Texture array matches the existing level texture approach.
- Should the initial binary support sounds files loading, or should audio be deferred to a second pass? Leaning toward including it since the audio crate is complete and the integration is lightweight.
