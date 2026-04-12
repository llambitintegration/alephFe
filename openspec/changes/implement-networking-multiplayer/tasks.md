## 1. Crate Setup and Dependencies

- [ ] 1.1 Create `marathon-net/` directory with `Cargo.toml` and `src/lib.rs`, add to workspace members in root `Cargo.toml`
- [ ] 1.2 Add dependencies to `marathon-net`: `ggrs`, `matchbox_socket`, `marathon-sim` (path), `serde`, `bytemuck`, `bincode`, `log`
- [ ] 1.3 Create module structure in `marathon-net/src/`: `session.rs`, `transport.rs`, `lobby.rs`, `sync.rs`, `spectator.rs`, `input.rs`, `types.rs`
- [ ] 1.4 Add `marathon-net` as a dependency of `marathon-game` and `marathon-web`
- [ ] 1.5 Verify workspace builds with `cargo check` for all crates (no code yet, just scaffolding)

## 2. Multi-Player Tick Input

- [ ] 2.1 Add `PlayerSlot(pub usize)` component to `marathon-sim/src/components.rs` with `Component`, `Debug`, `Clone`, `Copy`, `Serialize`, `Deserialize` derives
- [ ] 2.2 Change `SimWorld::tick(input: TickInput)` signature to `tick(inputs: &[TickInput])` in `marathon-sim/src/tick.rs`
- [ ] 2.3 Update `run_player_physics()` to iterate over all `Player` entities with `PlayerSlot`, consuming the `TickInput` at the matching slot index
- [ ] 2.4 Add `num_players` parameter to `SimWorld::new()` in `marathon-sim/src/world.rs`; spawn multiple `Player` entities with distinct `PlayerSlot` values at spawn points from the `GameMode`
- [ ] 2.5 Update `SimSnapshot` to replace `player: Option<PlayerSnapshot>` with `players: Vec<PlayerSnapshot>` and update `serialize()`/`deserialize()` accordingly
- [ ] 2.6 Add `player_position_for_slot(slot: usize)`, `player_facing_for_slot(slot: usize)`, etc. accessor methods; keep existing no-argument versions returning slot 0
- [ ] 2.7 Update all existing call sites of `sim.tick(input)` to `sim.tick(&[input])` in `marathon-game`, `marathon-web`, and tests
- [ ] 2.8 Add unit test: two `SimWorld` instances with `num_players = 4`, identical seeds and inputs, verify identical state after 100 ticks

## 3. Fast Save/Restore for Rollback

- [ ] 3.1 Define `SavedState` struct in `marathon-sim/src/world.rs` that holds a clone of all ECS component storages, resources (RNG, tick counter), and entity metadata
- [ ] 3.2 Implement `SimWorld::save_state() -> Box<SavedState>` that deep-copies the ECS world state
- [ ] 3.3 Implement `SimWorld::load_state(&SavedState)` that restores the ECS world from the saved state
- [ ] 3.4 Add unit test: save at tick 100, run 50 ticks, restore, replay same 50 inputs, verify identical state to running 150 ticks straight
- [ ] 3.5 Add benchmark test: `save_state()` + `load_state()` on a level with 50 monsters, verify combined time < 1 ms
- [ ] 3.6 Verify existing `serialize()`/`deserialize()` path still works after save/restore refactoring

## 4. GGRS Integration

- [ ] 4.1 Define `GGRSInput` as a 12-byte `bytemuck::Pod` struct in `marathon-net/src/input.rs` (u32 flags + f32 yaw + f32 pitch)
- [ ] 4.2 Implement `From<TickInput> for GGRSInput` and `From<GGRSInput> for TickInput` conversions
- [ ] 4.3 Implement `NetSession` struct in `marathon-net/src/session.rs` wrapping a `ggrs::P2PSession<GGRSInput>`
- [ ] 4.4 Implement `NetSession::new()` that creates a GGRS session with configurable input delay, max rollback frames, and number of players
- [ ] 4.5 Implement `NetSession::add_local_input(input: TickInput)` that submits the local player's input to GGRS
- [ ] 4.6 Implement `NetSession::advance(sim: &mut SimWorld)` that processes GGRS requests: save state on `SaveGameState`, load state on `LoadGameState`, advance sim on `AdvanceFrame` with the multi-player input slice
- [ ] 4.7 Implement `SyncTestSession` wrapper for local determinism validation (no transport needed)
- [ ] 4.8 Add integration test: `SyncTestSession` runs 500 ticks with random inputs, verifies no desync

## 5. Matchbox WebRTC Transport

- [ ] 5.1 Implement `create_matchbox_socket(signaling_url: &str, room_id: &str)` in `marathon-net/src/transport.rs` that creates a `WebRtcSocket` for the GGRS session
- [ ] 5.2 Implement native transport setup using `matchbox_socket`'s tokio-based runtime
- [ ] 5.3 Implement WASM transport setup using `matchbox_socket`'s browser WebRTC backend
- [ ] 5.4 Wire the Matchbox socket as the GGRS `NonBlockingSocket` transport in `NetSession`
- [ ] 5.5 Add `Dockerfile.signaling` for the Matchbox signaling server binary
- [ ] 5.6 Add signaling server to `docker-compose.marathon.yml` with appropriate network configuration
- [ ] 5.7 Add integration test: two `NetSession` instances connect via signaling server and exchange inputs (requires signaling server running)

## 6. Lobby and Session Management

- [ ] 6.1 Define `LobbyMessage` enum in `marathon-net/src/lobby.rs`: `PlayerJoined`, `PlayerLeft`, `PlayerReady`, `SettingsChanged`, `StartCountdown`, `CountdownTick`, `GameStart`
- [ ] 6.2 Implement `LobbyState` struct tracking connected players, ready status, team assignments, and game settings (map, mode, kill limit, time limit)
- [ ] 6.3 Implement lobby message serialization/deserialization over the Matchbox data channel (bincode)
- [ ] 6.4 Implement host-side lobby logic: broadcast settings changes, validate ready status, initiate start countdown
- [ ] 6.5 Implement client-side lobby logic: receive and apply settings, toggle ready, display countdown
- [ ] 6.6 Add `Lobby` variant to `GameState` enum in `marathon-integration/src/types.rs`
- [ ] 6.7 Add `PostGame` variant to `GameState` enum in `marathon-integration/src/types.rs`
- [ ] 6.8 Wire `MainMenu` -> `Lobby` transition on "Multiplayer" menu selection in the shell state machine
- [ ] 6.9 Wire `Lobby` -> `Loading` transition on game start; initialize GGRS session after countdown
- [ ] 6.10 Wire `Playing` -> `PostGame` transition on win condition met or all remote players disconnected
- [ ] 6.11 Add unit tests for `LobbyState`: player join/leave, ready toggling, settings changes, start validation

## 7. Desync Detection

- [ ] 7.1 Implement Fletcher-64 checksum function in `marathon-net/src/sync.rs` operating on a byte slice
- [ ] 7.2 Implement `compute_world_checksum(sim: &SimWorld) -> u64` that hashes all determinism-critical state: player components, monster components, projectile components, RNG state, tick counter
- [ ] 7.3 Wire checksum computation into `NetSession::advance()` on confirmed frames (not predicted)
- [ ] 7.4 Include the checksum in GGRS sync data via the `GGRSInput` sync payload
- [ ] 7.5 Implement desync detection: compare local and remote checksums for confirmed frames, log ERROR with full state dump on mismatch
- [ ] 7.6 Add unit test: two identical SimWorlds produce identical checksums after N ticks
- [ ] 7.7 Add unit test: perturb one world's state, verify checksum mismatch is detected

## 8. Spectator Mode

- [ ] 8.1 Implement `SpectatorSession` wrapper in `marathon-net/src/spectator.rs` using GGRS's spectator session type
- [ ] 8.2 Implement spectator input stream processing: receive confirmed inputs, advance sim in lockstep
- [ ] 8.3 Implement spectator camera modes: free camera (WASD + mouse) and player-follow (track selected player slot)
- [ ] 8.4 Implement player cycling for player-follow mode (keybind to switch to next player)
- [ ] 8.5 Add spectator connection handling to the lobby: spectators join without occupying a player slot
- [ ] 8.6 Add integration test: spectator receives confirmed inputs and produces identical sim state to players

## 9. Game Mode Wiring

- [ ] 9.1 Wire `GameMode::on_kill()` calls into the multiplayer tick loop when damage resolution results in a player kill
- [ ] 9.2 Wire `GameMode::check_win_condition()` into the post-tick phase; on `Winner` or `TimeLimitReached`, signal game end to the session
- [ ] 9.3 Wire `GameMode::respawn_delay()` and `GameMode::get_spawn_point()` into player death/respawn logic for multiplayer
- [ ] 9.4 Implement respawn timer per player slot: on death, start countdown, on expiry, respawn at mode-selected spawn point
- [ ] 9.5 Wire `GameMode::scores()` to the PostGame scoreboard display
- [ ] 9.6 Add integration test: 4-player deathmatch with scripted kills, verify scores match expected values

## 10. Game Binary Integration

- [ ] 10.1 Add multiplayer branch to `marathon-game/src/main.rs`: if multiplayer, create `NetSession` and use `session.advance(sim)` instead of direct `sim.tick()`
- [ ] 10.2 Add multiplayer branch to `marathon-web/src/lib.rs`: same architectural change for WASM game loop
- [ ] 10.3 Wire local player slot assignment: `NetSession` reports which slot is the local player; camera uses that slot's state
- [ ] 10.4 Update film recording to capture all players' `TickInput` values per tick in multiplayer mode
- [ ] 10.5 Update `FilmHeader` with `num_players` and per-player metadata fields
- [ ] 10.6 Update film playback to feed multi-player input slices when playing back multiplayer films
- [ ] 10.7 Add CLI/config options to `marathon-game`: `--multiplayer`, `--signaling-server`, `--room-id`, `--spectator`

## 11. Testing and Validation

- [ ] 11.1 Add determinism regression test: run 1000-tick multi-player sequences on two SimWorld instances, verify identical state
- [ ] 11.2 Add rollback correctness test: simulate misprediction by providing wrong inputs, then corrected inputs, verify final state matches straight-through playback
- [ ] 11.3 Add desync detection test: deliberately perturb one world's RNG state, verify checksum mismatch is detected and logged
- [ ] 11.4 Add `SyncTestSession` test in CI: configurable tick count and input pattern, validates determinism with zero network
- [ ] 11.5 Add lobby protocol test: simulate host and two clients exchanging lobby messages, verify settings propagation and start synchronization
- [ ] 11.6 Add Docker CI: ensure `marathon-net` compiles and unit tests pass in the existing Docker build pipeline
- [ ] 11.7 Add end-to-end network test (requires signaling server): two clients connect, play 100 ticks, verify both reach identical final state
