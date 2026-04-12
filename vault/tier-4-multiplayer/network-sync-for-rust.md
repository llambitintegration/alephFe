---
tags: [multiplayer, networking, architecture-decision, rust-crates]
status: research-complete
---

# Network Sync for Rust

The Rust rebuild runs a **30 tick/s deterministic simulation** with seeded RNG (`StdRng::seed_from_u64`). All game state advances through `SimWorld::tick(TickInput)` which takes action flags as input. This makes the project a textbook candidate for deterministic networking.

This note compares networking architectures and evaluates Rust crates.

## Architecture Comparison

### 1. Deterministic Lockstep

The original Marathon approach. Every player sends inputs; simulation waits for all inputs before advancing.

**How it works**:
1. Each player gathers local input into `ActionFlags`
2. Flags are sent to all other players (or to hub in star topology)
3. Simulation blocks until all players' flags for tick N arrive
4. All players advance tick N simultaneously
5. Repeat

**Pros**:
- Simplest to implement
- Zero state divergence (all players always in sync)
- Minimal bandwidth (only send ~4 bytes per player per tick)
- Perfect for film recording (flags ARE the recording)
- No need for state rollback/snapshot machinery

**Cons**:
- Input latency = network RTT (100ms RTT = 100ms input lag)
- One slow player lags everyone
- Game freezes when packets are late (stuttering)
- Poor Internet experience (Marathon's original problem)

**Best for**: LAN play, games tolerant of input delay (RTS, turn-based)

### 2. Rollback (GGPO-style)

The modern standard for latency-sensitive deterministic games. Local input is applied immediately; remote input is predicted, then corrected via rollback if wrong.

**How it works**:
1. Local player's input is applied immediately (zero local latency)
2. Remote players' inputs are predicted (typically: repeat last known input)
3. Simulation advances with predicted inputs
4. When actual remote inputs arrive, if they differ from predictions:
   a. Roll back game state to the divergence point
   b. Re-simulate forward with correct inputs
   c. Render the corrected current frame
5. If predictions were correct (common case), no rollback needed

**Pros**:
- Zero perceived input latency for local player
- Handles packet jitter gracefully (predictions smooth over gaps)
- Industry standard for fighting games, action games
- GGRS provides a mature Rust implementation
- Compatible with deterministic film recording

**Cons**:
- Requires fast state save/restore (snapshot per frame within rollback window)
- Re-simulation cost: must re-tick N frames when rolling back
- Visual corrections can cause "teleporting" artifacts on high latency
- More complex implementation than lockstep
- State snapshot size matters (bevy_ecs `World` can be large)

**Best for**: Action games with < 8 players, FPS games, fighting games

### 3. Client-Server (Authoritative)

One machine runs the simulation; clients send inputs and receive state updates.

**How it works**:
1. Server runs the authoritative simulation
2. Clients send inputs to server
3. Server applies inputs, advances simulation
4. Server sends state snapshots/deltas to clients
5. Clients interpolate between received states

**Pros**:
- Server is authoritative (anti-cheat)
- Scales to many players (clients don't need full simulation)
- No determinism requirement (server is single source of truth)
- Natural for hosted/cloud deployment

**Cons**:
- High bandwidth (full state or delta snapshots every tick)
- Server costs (someone must host)
- All players have input latency (client -> server -> client)
- Prediction/interpolation complexity for smooth visuals
- Doesn't leverage the existing deterministic sim design
- Film recording requires server-side infrastructure

**Best for**: Large-scale games (MMO, battle royale), competitive anti-cheat needs

## Recommendation: Rollback (GGRS + Matchbox)

For the Marathon Rust rebuild, **rollback is the clear winner**:

1. The sim is already deterministic with action-flag input -- perfect for rollback
2. Marathon is an FPS where input latency matters greatly
3. Player counts are small (2-8 players) -- rollback overhead is manageable
4. GGRS + Matchbox gives native + WASM support in one architecture
5. Film recording naturally falls out of the action flag stream

### State Snapshot Strategy for Rollback

The main technical challenge is fast save/restore of `SimWorld`. The current `SimWorld::snapshot()` method serializes via `bincode`, which is too slow for per-frame rollback snapshots.

**Proposed approach**:
- Implement a lightweight clone-based snapshot (clone the bevy_ecs `World`)
- Or: maintain a ring buffer of `SimSnapshot` structs (already defined in `world.rs`)
- GGRS typically needs ~7 frames of rollback window (at 30fps, that's 233ms)
- Profile to ensure re-simulation of 7 ticks completes within 33ms budget

## Rust Crate Evaluation

### GGRS (Good Game Rollback System)

**Crate**: [`ggrs`](https://crates.io/crates/ggrs) | **GitHub**: [gschup/ggrs](https://github.com/gschup/ggrs)

GGRS is a pure-Rust reimagination of the GGPO network SDK.

| Aspect | Details |
|--------|---------|
| **Architecture** | P2P rollback networking |
| **API** | Synchronous, non-callback (improvement over GGPO's C callback API) |
| **Transport** | Pluggable -- works with any transport that sends/receives bytes |
| **WASM** | Yes, via Matchbox WebRTC transport |
| **Bevy integration** | `bevy_ggrs` crate available |
| **Maturity** | Actively maintained, used in production games |
| **License** | MIT / Apache 2.0 |

**Pros**:
- Purpose-built for deterministic game rollback
- Clean Rust API (no unsafe, no callbacks)
- Transport-agnostic: works with UDP, WebRTC, or any custom transport
- `bevy_ggrs` provides direct bevy_ecs integration (world save/restore)
- Active community, good documentation
- Works with Matchbox for browser play

**Cons**:
- Requires the game to implement `Clone`-able game state or snapshot/restore
- Limited to P2P topology (no dedicated server mode)
- Rollback window is finite; very high latency still causes issues

**Verdict**: **Strong recommendation.** GGRS is the right abstraction for this project.

### Matchbox

**Crate**: [`matchbox_socket`](https://crates.io/crates/matchbox_socket) | **GitHub**: [johanhelsing/matchbox](https://github.com/johanhelsing/matchbox)

Matchbox provides WebRTC data channel networking for both native and WASM targets.

| Aspect | Details |
|--------|---------|
| **Architecture** | P2P via WebRTC data channels |
| **Transport** | WebRTC (unreliable + reliable channels) |
| **WASM** | First-class support (this is its primary use case) |
| **Signaling** | Requires a `matchbox_server` for initial peer discovery |
| **Bevy integration** | `bevy_matchbox` crate available |
| **License** | MIT / Apache 2.0 |

**Pros**:
- Seamless native + WASM support from one codebase
- Designed to work with GGRS (the two projects are companions)
- Handles WebRTC complexity (ICE, STUN, signaling)
- Both unreliable (UDP-like) and reliable channels
- Lightweight signaling server (can be hosted cheaply)

**Cons**:
- Requires hosting a signaling server (matchbox_server)
- WebRTC connection establishment can be slow (ICE negotiation, ~1-3 seconds)
- NAT traversal isn't 100% reliable (may need TURN relay fallback)
- Adds WebRTC dependency to the native build (can be feature-gated)

**Verdict**: **Strong recommendation for the WebRTC transport layer**, especially for marathon-web browser play.

### Quinn (QUIC)

**Crate**: [`quinn`](https://crates.io/crates/quinn) | **GitHub**: [quinn-rs/quinn](https://github.com/quinn-rs/quinn)

Quinn is a pure-Rust async QUIC implementation. 86M+ total downloads.

| Aspect | Details |
|--------|---------|
| **Architecture** | Client-server or P2P over QUIC (UDP-based) |
| **Transport** | QUIC protocol (multiplexed, encrypted, congestion-controlled) |
| **WASM** | No native WASM support (QUIC requires raw UDP) |
| **Maturity** | Very mature, battle-tested |
| **License** | MIT / Apache 2.0 |

**Pros**:
- Extremely mature and well-maintained
- Built-in encryption (TLS 1.3), congestion control, multiplexing
- Multiple independent streams per connection
- Fast connection establishment (0-RTT resumption)
- Bevy integration via `bevy_quinnet`

**Cons**:
- No WASM support (browsers cannot do raw UDP/QUIC)
- Designed for reliable transport; unreliable datagrams are an extension
- Overkill for the simple action-flag exchange Marathon needs
- Async-heavy API adds complexity

**Verdict**: Good for a **dedicated server** approach or native-only LAN play. Not suitable for the browser multiplayer target.

### Laminar

**Crate**: [`laminar`](https://crates.io/crates/laminar) | **GitHub**: [TimonPost/laminar](https://github.com/TimonPost/laminar)

Semi-reliable UDP protocol for multiplayer games.

| Aspect | Details |
|--------|---------|
| **Architecture** | UDP with configurable reliability/ordering |
| **Maturity** | Last updated ~2021, appears unmaintained |
| **WASM** | No |
| **License** | MIT |

**Pros**:
- Simple API for unreliable/reliable/ordered UDP packets
- Connection tracking, fragmentation, RTT estimation
- Link conditioner for testing

**Cons**:
- Appears **unmaintained** (no updates since 2021)
- No WASM support
- Low-level -- would need to build rollback on top
- Originally designed for Amethyst engine (now defunct)

**Verdict**: **Not recommended.** Unmaintained and superseded by GGRS + transport.

### Naia

**Crate**: [`naia-server`](https://crates.io/crates/naia-server) / [`naia-client`](https://crates.io/crates/naia-client) | **GitHub**: [naia-lib/naia](https://github.com/naia-lib/naia)

Cross-platform networking library inspired by Nengi.js and Colyseus.

| Aspect | Details |
|--------|---------|
| **Architecture** | Client-server with entity synchronization |
| **Transport** | UDP (native) + WebRTC (WASM) |
| **WASM** | Yes |
| **Bevy integration** | `naia-bevy-server` / `naia-bevy-client` |
| **License** | MIT |

**Pros**:
- Cross-platform including WASM
- Entity-based sync with "rooms" and "scopes" (Tribes 2 model)
- Built-in client-server architecture
- Bevy integration available

**Cons**:
- Client-server model (not P2P) -- would need a hosted server
- Entity sync is overkill for action-flag exchange
- Doesn't leverage the deterministic sim (synchronizes state, not inputs)
- More complex API than needed

**Verdict**: Good for a client-server game, but **not the right fit** for Marathon's deterministic lockstep/rollback model.

## Proposed Architecture

```
+---------------------------------------------+
|              marathon-net crate              |
|  (new crate, feature-gated for native/wasm) |
+---------------------------------------------+
|                                             |
|  +---------------------------------------+  |
|  |           GGRS Session                |  |
|  |  - P2PSession or SpectatorSession     |  |
|  |  - handles rollback logic             |  |
|  |  - calls save_game_state / load_...   |  |
|  +---------------------------------------+  |
|         |                    |               |
|  +------+------+    +-------+--------+      |
|  | UDP Socket  |    | Matchbox Socket|      |
|  | (native)    |    | (WebRTC/WASM)  |      |
|  +-------------+    +----------------+      |
+---------------------------------------------+
         |                      |
    Native Desktop         marathon-web
    (direct UDP P2P)       (browser WebRTC)
```

### Integration Points

1. **`marathon-sim`**: Add `SimWorld::save_state() -> SimState` and `SimWorld::load_state(SimState)` for rollback snapshots
2. **`marathon-integration`**: Multi-player `ActionFlags` collection (one per player per tick)
3. **`marathon-net`** (new): GGRS session management, transport abstraction
4. **`marathon-web`**: Matchbox WebRTC transport for browser play
5. **Film recording**: Tap into the GGRS confirmed-input stream for recording

### Multiplayer Tick Flow

```
1. Gather local input -> ActionFlags
2. Feed local flags to GGRS session
3. GGRS returns: advance frame(s) with these inputs
   - May include rollback: load snapshot, re-sim with corrected inputs
4. For each frame to advance:
   a. SimWorld::tick(multi_player_inputs[tick])
   b. Film recorder records confirmed inputs
5. Render current frame
```

## Open Questions

1. **Snapshot performance**: Can `SimWorld` be cloned/restored fast enough for 7-frame rollback at 30fps? Profile needed.
2. **Spectator support**: GGRS supports spectator sessions -- useful for tournaments?
3. **NAT traversal reliability**: What percentage of players will need TURN relay?
4. **Signaling server hosting**: Where to host `matchbox_server`? (Simple WebSocket server, could be on a $5/mo VPS)
5. **Desync detection**: Implement checksum comparison (like alephone's world checksum)?

## See Also

- [[alephone-network-architecture]] -- How the original engine does it
- [[webrtc-browser-multiplayer]] -- WebRTC specifics for marathon-web
- [[film-replay-system]] -- How film recording integrates with networking
- [GGRS GitHub](https://github.com/gschup/ggrs)
- [Matchbox GitHub](https://github.com/johanhelsing/matchbox)
- [Deterministic Lockstep - Gaffer On Games](https://gafferongames.com/post/deterministic_lockstep/)
- [Netcode Architectures Part 1: Lockstep](https://www.snapnet.dev/blog/netcode-architectures-part-1-lockstep/)
- [Netcode Concepts Part 3: Lockstep and Rollback](https://meseta.medium.com/netcode-concepts-part-3-lockstep-and-rollback-f70e9297271)
