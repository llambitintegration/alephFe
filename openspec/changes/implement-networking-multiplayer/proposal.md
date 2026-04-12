## Why

Multiplayer was a defining feature of the original Marathon -- it launched in 1994 with 8-player network play across six game modes, two years before Quake. The Rust rebuild has all the prerequisites sitting idle: a 30 tick/s deterministic simulation with seeded RNG, an `ActionFlags` bitfield consumed per-tick, a `SimSnapshot` serialization system, `GameMode` traits with scoring logic for all six modes (Every Man For Himself, King of the Hill, Kill The Man With The Ball, Tag, Cooperative, and team variants), `SpawnPoint` with team assignment, and a film recording system that already captures per-tick action flags -- proving the sim is input-driven and deterministic. But there is zero networking code. `SimWorld::tick()` accepts a single `TickInput`, there is no concept of multiple player tracks, no transport layer, no session management, and no way for two instances of the game to exchange data. The game modes exist as dead code because there is no way to have more than one player.

The architecture choice matters here. The original Aleph One used peer-to-peer lockstep networking -- every player must receive every other player's input before the simulation can advance, meaning the game runs at the speed of the slowest connection. Modern rollback netcode (GGRS) eliminates this: each client predicts remote inputs and rolls back when predictions are wrong, giving responsive gameplay even with 100+ ms latency. Combined with Matchbox for WebRTC transport, this enables native-to-native, browser-to-browser, and native-to-browser cross-platform play -- marathon-web players can play against marathon-game players with no relay server.

## What Changes

- **New `marathon-net` crate**: A workspace member that owns all networking concerns -- session management, transport abstraction, input synchronization, rollback orchestration, and lobby/matchmaking. Depends on `marathon-sim` (for `ActionFlags`, `SimWorld`, `SimSnapshot`) and brings in `ggrs` (rollback networking) and `matchbox_socket` (WebRTC transport). No rendering or platform code.

- **Multi-player action flag tracks**: `SimWorld::tick()` currently accepts a single `TickInput`. Change the tick interface to accept a `Vec<TickInput>` (or `[TickInput; N]`) indexed by player slot, so each player's action flags drive their own entity. The single-player path passes a one-element vec. `SimWorld::new()` gains a `num_players` parameter that spawns multiple `Player` entities at distinct spawn points selected by the active `GameMode`.

- **SimWorld state save/restore for rollback**: GGRS requires snapshotting and restoring the full simulation state on every rollback. The existing `SimSnapshot` and `serialize()`/`deserialize()` pipeline provides the foundation, but it round-trips through bincode which is too slow for per-frame rollback (potentially multiple times per tick). Add a fast in-memory `save_state()` -> opaque handle and `load_state(handle)` path that clones the ECS world directly (or uses a component-level memcpy snapshot) without serialization overhead. Target < 1 ms for save/restore on a typical level.

- **GGRS integration in marathon-net**: Wrap the simulation loop in a GGRS `P2PSession` (or `SyncTestSession` for local testing). Each tick, GGRS tells the session which inputs are confirmed vs predicted; `marathon-net` translates between GGRS's `GGRSInput` type and `ActionFlags`. On rollback, `marathon-net` calls `load_state()`, replays the corrected inputs forward, then resumes normal play. The GGRS frame delay parameter (typically 2 frames / 67 ms) is configurable.

- **Matchbox WebRTC transport**: Use `matchbox_socket` as the GGRS transport backend. Matchbox provides WebRTC data channels that work in both native (via the `matchbox_socket` native runtime) and WASM (via browser WebRTC APIs). A lightweight signaling server (the Matchbox signaling server, deployable as a single binary or on the existing Docker infrastructure) handles the initial WebRTC handshake. After connection, all data flows peer-to-peer.

- **Lobby and session management**: Before gameplay begins, players need to discover each other, agree on map/mode/settings, and synchronize the start. Implement a `Lobby` state in the game shell state machine (between `MainMenu` and `Loading`) that handles: room creation/joining via the signaling server, player list with ready status, game settings (map, mode, kill limit, time limit, team assignments), and a synchronized countdown to game start. The lobby exchanges these settings over the Matchbox data channel before GGRS session initialization.

- **Desync detection via world checksum**: After each confirmed tick (not predicted), each client computes a checksum of the simulation state (hash of player positions, monster states, RNG state, tick counter) and includes it in the GGRS sync data. If checksums diverge, the session detects the desync and can log diagnostic state for debugging. This is critical for validating that the deterministic sim actually stays in sync across platforms (especially native vs WASM floating-point edge cases).

- **Spectator mode**: GGRS supports spectator sessions that receive confirmed inputs without contributing their own. Add a `Spectator` connection type that receives the full input stream, runs the sim locally, and renders from a free camera or follows a selected player. Spectators do not occupy a player slot and do not affect rollback.

- **Wire game modes to multiplayer session**: Connect the existing `GameMode` trait implementations (scoring, win conditions, respawn) to the multiplayer tick loop. On player death, the `GameMode` determines respawn delay and spawn point. On kill, the `GameMode` updates scores. On win condition met, the session ends and all clients transition to a post-game scoreboard.

## Capabilities

### New Capabilities

- `networking-transport`: WebRTC peer-to-peer data channel transport via Matchbox, working on both native (tokio runtime) and WASM (browser WebRTC). Signaling server for initial handshake. Handles connection, disconnection, and reconnection. Abstracts transport so alternative backends (direct UDP, Steam networking) can be added later.

- `rollback-netcode`: GGRS-based rollback networking session that synchronizes inputs across 2-8 players. Manages input prediction, rollback on misprediction, state save/restore, and frame advantage. Configurable input delay and rollback window. Includes `SyncTestSession` mode for determinism validation during development.

- `lobby-matchmaking`: Pre-game lobby for room creation/discovery, player management (join, leave, ready, team assignment), game settings negotiation (map, mode, limits), and synchronized game start. Uses Matchbox signaling for room discovery. Lobby state integrated into the game shell state machine.

- `desync-detection`: Per-tick world state checksumming for confirmed frames. Checksum included in GGRS sync payloads. Desync triggers diagnostic logging with full state dump of the divergent tick. Provides the foundation for cross-platform determinism validation (native vs WASM).

- `spectator-mode`: Non-participating observer connections that receive confirmed input streams and run the simulation locally. Free camera and player-follow camera modes. No player slot consumed, no rollback impact.

### Modified Capabilities

- `game-loop`: `SimWorld::tick()` accepts multi-player input (one `TickInput` per player slot). `SimWorld::new()` spawns multiple player entities when `num_players > 1`. The tick ordering remains identical -- all players' physics run in player-slot order within the same step, preserving determinism.

- `game-loop`: `SimWorld` gains `save_state()` and `load_state()` methods for fast in-memory snapshotting, separate from the existing `serialize()`/`deserialize()` path which remains for save files and film recording. The rollback path calls these potentially multiple times per rendered frame.

- `game-shell`: The state machine gains a `Lobby` state between `MainMenu` and `Loading`. Transitions: `MainMenu` -> `Lobby` (on "Multiplayer" selected), `Lobby` -> `Loading` (on game start), `Playing` -> `PostGame` (on win condition or disconnect). Single-player campaign bypasses `Lobby` entirely.

- `input-system`: In multiplayer, the local player's `TickInput` is fed to GGRS rather than directly to `SimWorld::tick()`. GGRS returns the authoritative input set (local + remote, confirmed or predicted) which is then passed to the sim. The input capture pipeline itself is unchanged.

- `game-shell` (film recording): Multiplayer film recording captures all players' action flags per tick (not just the local player). Film playback of multiplayer games feeds all tracks into the sim. The film header gains `num_players` and per-player metadata.

## Impact

- **New crate: `marathon-net/`** -- New workspace member. Contains `session.rs` (GGRS session wrapper), `transport.rs` (Matchbox socket setup), `lobby.rs` (pre-game lobby logic), `sync.rs` (checksum and desync detection), `spectator.rs` (spectator session), and `input.rs` (ActionFlags <-> GGRSInput conversion). Dependencies: `ggrs`, `matchbox_socket`, `marathon-sim`, `serde`, `bytemuck`.

- **marathon-sim/src/tick.rs** -- `SimWorld::tick()` signature changes from `tick(TickInput)` to `tick(inputs: &[TickInput])` where the slice is indexed by player slot. The single-player call site passes `&[input]`. Player physics loop iterates over player entities in slot order, each consuming their own `TickInput`. The `TickInput` struct is unchanged.

- **marathon-sim/src/world.rs** -- `SimWorld::new()` gains `num_players: usize` parameter. `spawn_map_objects()` spawns multiple `Player` entities at different spawn points (selected via `GameMode::get_spawn_point()`). New `save_state() -> Box<SavedState>` and `load_state(&SavedState)` methods for fast rollback snapshots. `SimSnapshot` gains a `players: Vec<PlayerSnapshot>` field (replacing the single `player: Option<PlayerSnapshot>`).

- **marathon-sim/src/components.rs** -- `Player` component gains a `slot: usize` field identifying which input slot drives this entity. New `PlayerSlot(usize)` component for querying by slot index.

- **marathon-integration/src/modes/** -- No structural changes to `GameMode` trait or implementations. The existing `on_kill()`, `check_win_condition()`, `get_spawn_point()`, and `respawn_delay()` methods are already parameterized by player ID. They will be called from the multiplayer tick loop.

- **marathon-game/src/main.rs** -- The game loop gains a branch: in multiplayer, instead of calling `sim.tick(&[local_input])` directly, it calls `marathon_net::Session::advance()` which internally manages GGRS and calls `sim.tick()` with the synchronized inputs. The rendering path is unchanged -- it still reads `sim.player_position()` etc., but needs to know which player slot is the local player for camera placement.

- **marathon-web/src/lib.rs** -- Same architectural change as marathon-game: the WASM game loop feeds local input to the `marathon-net` session rather than directly to the sim. Matchbox's WASM WebRTC transport is used. The signaling server URL is configured at build time or passed from the hosting page.

- **Cargo.toml (workspace)** -- Add `marathon-net` to workspace members. New dependencies across the workspace: `ggrs` (rollback library), `matchbox_socket` (WebRTC transport), `bytemuck` (for input serialization to GGRS).

- **Signaling server deployment** -- The Matchbox signaling server needs to be deployed alongside the existing Docker infrastructure. A new `Dockerfile.signaling` or addition to `docker-compose.marathon.yml` for the `matchbox_server` binary. Lightweight -- handles only WebRTC signaling handshakes, not game traffic.

- **Existing tests unaffected** -- All current tests operate on single-player `SimWorld` with a single `TickInput`. They continue to work with a one-element input slice. New integration tests validate multi-player tick determinism (two SimWorlds with identical seeds and identical multi-player input sequences produce identical state), rollback correctness (save/load/replay produces identical state), and desync detection (deliberately perturbing one world triggers checksum mismatch).
