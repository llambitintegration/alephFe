## Context

The alephone-rust project has seven workspace crates implementing Marathon's core subsystems: format parsing, simulation, audio, integration (HUD/menus/input/shell/game-modes), a 3D level viewer, a native game binary, and a WASM web binary. The simulation runs at a fixed 30 ticks/second driven by a single `TickInput` per tick, using a deterministic ECS (bevy_ecs) with seeded PRNG. All six multiplayer game modes (Every Man For Himself, King of the Hill, Kill The Man With The Ball, Tag, Cooperative, team variants) are fully implemented in `marathon-integration/src/modes/` with scoring, win conditions, spawn-point selection, and respawn logic parameterized by player ID -- but they are dead code because `SimWorld::tick()` accepts only a single `TickInput`, there is only one `Player` entity, and there is no networking layer.

The existing `SimSnapshot` serialization system (bincode round-trip through `serialize()`/`deserialize()`) proves the sim is fully serializable. The film recording system captures per-tick `ActionFlags`, proving the sim is input-driven and deterministic. These two properties -- determinism and serializability -- are exactly what rollback netcode requires.

## Goals / Non-Goals

**Goals:**
- New `marathon-net` crate that owns all networking concerns, cleanly separated from sim and rendering
- Multi-player input: `SimWorld::tick()` accepts one `TickInput` per player slot, spawning multiple `Player` entities
- Fast in-memory save/restore for rollback (sub-millisecond, no bincode round-trip)
- GGRS rollback netcode integration for responsive gameplay at 100+ ms latency
- Matchbox WebRTC transport enabling native-to-native, browser-to-browser, and cross-platform play
- Pre-game lobby with room creation, player management, settings negotiation, synchronized start
- Per-tick desync detection via world-state checksums on confirmed frames
- Spectator mode that receives confirmed inputs without occupying a player slot
- Wire existing game modes into the multiplayer tick loop
- Docker-deployable signaling server for WebRTC handshakes

**Non-Goals:**
- Competitive ranking, matchmaking rating, or persistent accounts
- Server-authoritative architecture (this is peer-to-peer rollback)
- Voice chat (out of scope; Marathon's MICROPHONE flag is a cosmetic holdover)
- Replay/film of multiplayer games from arbitrary camera angles (film recording captures all players' inputs, but advanced replay UI is deferred)
- LAN discovery or mDNS (initial discovery is via signaling server rooms)
- Anti-cheat (determinism validation via checksums is the extent)

## Decisions

### 1. GGRS rollback netcode over lockstep

**Decision:** Use the `ggrs` crate for rollback-based input synchronization.

**Rationale:** The original Aleph One used peer-to-peer lockstep where every player waits for every other player's input before advancing -- the game runs at the speed of the slowest connection. GGRS implements the GGPO algorithm in Rust: each client predicts remote inputs and rolls back on misprediction, giving responsive local gameplay even with 100+ ms round-trip latency. The sim is already deterministic and serializable, which are the two prerequisites. GGRS is the de-facto standard for Rust rollback networking, is well-maintained, and has first-class support for the Matchbox transport.

**Alternative considered:** Raw lockstep (simpler, no save/restore needed). Rejected because it produces unacceptable input lag on internet connections and eliminates the cross-platform native-to-browser use case where latency is inherently higher.

### 2. Matchbox WebRTC transport for cross-platform P2P

**Decision:** Use `matchbox_socket` as the GGRS transport backend for all platforms.

**Rationale:** Matchbox provides WebRTC data channels that work identically on native (via the `matchbox_socket` native runtime using tokio) and in WASM (via browser WebRTC APIs). This is the only transport layer that enables native-to-browser play without a relay server. After the initial signaling handshake, all game traffic flows peer-to-peer. Matchbox integrates directly with GGRS as a `NonBlockingSocket` implementation. The Matchbox signaling server is a lightweight single binary deployable on the existing Docker infrastructure.

**Alternative considered:** Raw UDP sockets for native, a separate WebSocket relay for WASM. Rejected because it would require two transport backends, a relay server carrying all game traffic, and would not support native-to-browser direct connections.

### 3. Multi-player tick via indexed input slices

**Decision:** Change `SimWorld::tick(input: TickInput)` to `SimWorld::tick(inputs: &[TickInput])` where the slice is indexed by player slot. The `Player` component gains a `slot: usize` field. Single-player passes `&[input]` (a one-element slice).

**Rationale:** This is the minimal interface change that enables N-player simulation while maintaining full backward compatibility for single-player. The tick ordering iterates players in slot order within each physics step, preserving determinism. No new ECS system scheduling is needed -- the existing per-player physics loop simply iterates over multiple `Player` entities instead of one.

**Alternative considered:** Separate `tick_single(TickInput)` and `tick_multi(Vec<TickInput>)` methods. Rejected because it duplicates the tick path and the single-player case is just `tick(&[input])`.

### 4. Fast save/restore via World clone, separate from SimSnapshot

**Decision:** Add `save_state() -> Box<SavedState>` and `load_state(&SavedState)` methods that clone the ECS `World` directly (deep-copy all components and resources). Keep the existing `serialize()`/`deserialize()` path for save files and film recording.

**Rationale:** GGRS may call save/restore multiple times per rendered frame during rollback. The bincode serialization path in `SimSnapshot` round-trips through allocation-heavy serde, which is too slow for per-frame rollback (profiling suggests 5-15 ms per save on a typical level). Direct `World` cloning avoids serialization overhead entirely. The `SavedState` is an opaque struct holding the cloned `World`. Target: < 1 ms save + restore combined.

**Alternative considered:** Component-level `memcpy` snapshot (copy each component storage as raw bytes). More complex to implement and requires unsafe code for bevy_ecs internals. Start with World clone and optimize if profiling shows it is insufficient.

### 5. Lobby state in the game shell state machine

**Decision:** Add a `Lobby` state to the `GameState` enum between `MainMenu` and `Loading`. The lobby handles room creation/joining, player list management, settings negotiation, ready-up, and synchronized countdown. Single-player campaign bypasses `Lobby` entirely.

**Rationale:** Players need to discover each other, agree on map/mode/settings, and synchronize the start before gameplay. This is a distinct UI and networking phase that doesn't fit into any existing state. The lobby communicates over the Matchbox data channel before GGRS session initialization.

**Alternative considered:** External matchmaking (web-based lobby separate from the game). Rejected for complexity and because it doesn't work for the native binary. An in-game lobby provides a unified experience.

### 6. Desync detection via Fletcher checksum on confirmed frames

**Decision:** After each confirmed tick (not predicted), compute a Fletcher-64 checksum over determinism-critical state: all player positions, velocities, health, shield, oxygen, facing, monster positions and states, RNG state, and tick counter. Include this checksum in the GGRS sync payload. On mismatch, log full diagnostic state.

**Rationale:** Cross-platform determinism (native vs WASM) is not guaranteed, especially with floating-point edge cases. Desync detection is critical for validating the deterministic sim actually stays in sync. Fletcher-64 is fast (no allocation, single pass over component data) and sufficient for detecting divergence. Logging the full state on desync enables root-cause analysis.

**Alternative considered:** CRC32 or SHA-256. CRC32 has higher collision risk. SHA-256 is overkill and slower. Fletcher-64 balances speed and reliability.

### 7. Spectator as a GGRS spectator session

**Decision:** Use GGRS's built-in spectator session type. Spectators receive the confirmed input stream, run the sim locally in lockstep (no rollback), and render from a free camera or player-follow camera.

**Rationale:** GGRS natively supports spectator connections that receive confirmed inputs without contributing their own and without occupying a player slot. This means spectators have no impact on rollback behavior and can join/leave freely.

### 8. `marathon-net` crate boundary

**Decision:** The `marathon-net` crate depends on `marathon-sim` (for `ActionFlags`, `TickInput`, `SimWorld`, `SavedState`) and brings in `ggrs` and `matchbox_socket`. It does NOT depend on rendering, audio, or windowing crates. The game binary (`marathon-game`, `marathon-web`) depends on both `marathon-net` and `marathon-sim`, calling `marathon-net::Session::advance()` which internally manages GGRS and returns the authoritative input set to feed into `sim.tick()`.

**Rationale:** Clean separation of concerns. Networking logic is testable without a GPU or audio device. The session abstraction hides GGRS details from the game loop -- the binary just feeds local input in and gets synchronized multi-player inputs out.

## Risks / Trade-offs

**[Save/restore performance]** Direct `World` clone may still be too slow for levels with many entities (hundreds of monsters). Mitigation: Profile early with the largest Marathon 2 levels. If needed, switch to a ring buffer of snapshots and component-level delta copying. GGRS's configurable rollback window (default 8 frames) bounds the worst case.

**[Cross-platform floating-point determinism]** Native x86_64 and WASM may produce different floating-point results for the same operations (WASM uses 32-bit floats but with potentially different rounding). Mitigation: The desync detection system will catch divergences immediately. If divergences occur, the fix is to use fixed-point arithmetic for critical paths (positions, velocities) or ensure all float operations are IEEE 754 compliant with explicit rounding.

**[WebRTC reliability]** WebRTC data channels can fail to establish through symmetric NATs or restrictive firewalls. Mitigation: Matchbox supports TURN relay as a fallback. Document the signaling server deployment with TURN configuration.

**[GGRS input size]** GGRS serializes inputs per frame. `TickInput` contains `ActionFlags` (u32) plus two f32 fields (mouse_yaw, mouse_pitch), totaling 12 bytes per player per frame. With 8 players, this is 96 bytes/frame, well within WebRTC data channel throughput.

**[Lobby complexity]** Robust lobby code (handling disconnects, host migration, late joins) is notoriously tricky. Mitigation: Start with a simple host-based lobby where the room creator is the authority. Host migration and reconnection are deferred to follow-up work.

**[Bevy ECS World::clone]** bevy_ecs `World` does not implement `Clone` out of the box. Mitigation: Implement manual world cloning by iterating all component storages and resources. Alternatively, wrap the snapshot in a serialized form using a fast binary format (e.g., `rkyv` for zero-copy deserialization) rather than bevy's own serialization.

## Open Questions

- Should the signaling server be bundled into the existing Docker compose file or deployed as a separate service? Leaning toward adding it to `docker-compose.marathon.yml` for simplicity.
- What is the right GGRS input delay default? 2 frames (67 ms) is standard for fighting games but Marathon's pace may tolerate 1 frame. Needs playtesting.
- Should the lobby UI be rendered via the existing menu system (text-based screens) or does it need a richer widget set (player list, team color pickers, map preview)? Start with text-based menus matching the existing menu architecture.
- How should the `Player` component marker work with multiple players? The current `Player` is a unit struct. Adding `PlayerSlot(usize)` as a separate component allows querying by slot while keeping `Player` as a marker for "is a player entity" queries.
