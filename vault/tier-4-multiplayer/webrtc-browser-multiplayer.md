---
tags: [multiplayer, networking, webrtc, wasm, browser]
status: research-complete
---

# WebRTC for Browser Multiplayer

The project has a `marathon-web` crate that compiles to WASM and runs in the browser. Adding browser-to-browser multiplayer requires WebRTC data channels since browsers cannot use raw UDP/TCP for game networking.

## Why WebRTC

Browsers provide two options for real-time communication:
1. **WebSocket** -- TCP-based, reliable, ordered. Too much latency for game state sync.
2. **WebRTC Data Channels** -- UDP-like, configurable reliability/ordering. Designed for real-time P2P.

WebRTC data channels provide:
- **Peer-to-peer** connections (data flows directly between browsers, not through a server)
- **Configurable delivery**: unreliable + unordered (UDP-like) OR reliable + ordered
- **Encryption**: Built-in DTLS encryption
- **NAT traversal**: ICE framework with STUN/TURN support
- **Low latency**: No head-of-line blocking on unreliable channels

## WebRTC Architecture

### Connection Establishment

WebRTC connections require a multi-step setup:

```
Browser A                  Signaling Server              Browser B
    |                           |                           |
    |--- "join room X" ------->|                           |
    |                           |<------ "join room X" ----|
    |                           |                           |
    |<-- "peer B joined" ------|                           |
    |                           |                           |
    |--- SDP Offer ----------->|-------- SDP Offer ------->|
    |                           |                           |
    |<-- SDP Answer -----------|<------- SDP Answer -------|
    |                           |                           |
    |--- ICE Candidates ------>|------- ICE Candidates --->|
    |<-- ICE Candidates -------|<------ ICE Candidates ----|
    |                           |                           |
    |<========= Direct P2P Data Channel =================>|
    |                           |                           |
```

### Components

1. **Signaling Server**: Coordinates initial peer discovery. Exchanges SDP offers/answers and ICE candidates. Only needed during connection setup -- no game data passes through it.

2. **STUN Server**: Helps peers discover their public IP address and port (NAT traversal). Free public STUN servers are widely available (Google, Mozilla).

3. **TURN Server**: Relay fallback when direct P2P connection fails (symmetric NAT, restrictive firewalls). All data is relayed through TURN, adding latency. Estimated ~10-15% of connections need TURN relay.

4. **Data Channel**: Once established, the direct P2P channel for game data. Configurable:
   - `ordered: false, maxRetransmits: 0` -- unreliable, unordered (UDP-like, best for action flags)
   - `ordered: true, reliable` -- for chat messages, lobby state

## Matchbox for Rust/WASM

The **Matchbox** project provides a Rust-native WebRTC abstraction that works on both native and WASM targets.

### matchbox_socket

The core crate. Handles:
- Connecting to a matchbox signaling server
- WebRTC peer connection establishment (ICE, SDP exchange)
- Creating unreliable + reliable data channels
- Abstracting away native-vs-WASM differences

### matchbox_server

A lightweight WebSocket-based signaling server. Written in Rust, deployable as a single binary.

**Deployment options**:
- Single $5/mo VPS (handles hundreds of concurrent rooms)
- Serverless (e.g., Cloudflare Workers, AWS Lambda with WebSocket API Gateway)
- Self-hosted alongside the game distribution

The signaling server is stateless per-room -- it just relays SDP/ICE messages between peers in the same "room" URL path.

### Integration with GGRS

Matchbox is designed to work as a transport for GGRS:

```rust
// WASM example (conceptual)
use matchbox_socket::WebRtcSocket;
use ggrs::{P2PSession, SessionBuilder};

// Create WebRTC socket, connect to signaling server
let (socket, message_loop) = WebRtcSocket::new_ggrs("wss://signal.example.com/room_id");

// Spawn the message loop (WASM: wasm_bindgen_futures)
wasm_bindgen_futures::spawn_local(message_loop);

// Wait for peers to connect
// ... poll socket.connected_peers() ...

// Build GGRS session with matchbox socket as transport
let session = SessionBuilder::<GGRSConfig>::new()
    .with_num_players(2)
    .add_player(PlayerType::Local, 0)
    .add_player(PlayerType::Remote(peer_id), 1)
    .start_p2p_session(socket)?;
```

## Current State in marathon-web

The `marathon-web` crate (`marathon-web/src/lib.rs`) currently:
- Compiles to WASM via `wasm-bindgen`
- Initializes console logging and panic hooks
- Accepts pre-fetched scenario data (map, shapes, physics) from JavaScript
- Runs the game renderer via `render::run_web()`

**No networking code exists yet.** The crate is single-player only.

### Key Files

- `marathon-web/src/lib.rs` -- WASM entry point, `start_game()` function
- `marathon-web/src/render.rs` -- WebGL2 rendering loop
- `marathon-web/src/level.rs` -- Level geometry loading
- `marathon-web/src/mesh.rs` -- Mesh generation
- `marathon-web/src/texture.rs` -- Texture atlas
- `marathon-web/src/sprites.rs` -- Sprite rendering

## Proposed Architecture for Browser Multiplayer

### High-Level Flow

```
                       Internet
                          |
              +-----------+-----------+
              |                       |
        Browser A                Browser B
        +-----------+           +-----------+
        | marathon-  |           | marathon-  |
        | web WASM   |           | web WASM   |
        +-----------+           +-----------+
        | matchbox   |<--------->| matchbox   |
        | _socket    | WebRTC   | _socket    |
        | (P2P)      | DataCh.  | (P2P)      |
        +-----------+           +-----------+
        | GGRS       |           | GGRS       |
        | session    |           | session    |
        +-----------+           +-----------+
        | SimWorld   |           | SimWorld   |
        | (determ.)  |           | (determ.)  |
        +-----------+           +-----------+
```

### Implementation Steps

1. **Add dependencies** to `marathon-web/Cargo.toml`:
   ```toml
   [dependencies]
   ggrs = "0.11"
   matchbox_socket = "0.10"
   ```

2. **Lobby/room UI**: Add HTML/JS UI for:
   - Creating a room (generates a room URL)
   - Joining a room (enters room URL)
   - Showing connected peers
   - Starting the game when all peers are ready

3. **Session setup**:
   - On "create room": connect to signaling server, get room ID
   - On "join room": connect to signaling server with room ID
   - When all players connected: negotiate game settings (map, mode, difficulty)
   - All players start `SimWorld` with identical config

4. **Game loop integration**:
   - Each frame: gather local input -> `ActionFlags`
   - Feed flags to GGRS session
   - GGRS returns: which frames to advance, with which inputs
   - Handle rollback (save/load `SimWorld` state)
   - Render current frame

5. **Film recording**: Optionally record confirmed inputs for post-game film

### Signaling Server Deployment

The `matchbox_server` binary needs to be hosted and accessible via WSS (WebSocket Secure):

```
marathon-web client  --wss://signal.marathon-rust.org/room_abc123-->  matchbox_server
```

Options:
- **Fly.io / Railway**: Easy Rust binary hosting, ~$5/mo
- **Self-hosted VPS**: Any Linux box with a TLS certificate
- **Cloudflare Workers**: Not directly compatible (needs persistent WebSocket), but possible with Durable Objects

### NAT Traversal Considerations

- **STUN**: Use free public STUN servers (e.g., `stun:stun.l.google.com:19302`)
- **TURN**: ~10-15% of players may need TURN relay. Options:
  - Self-hosted `coturn` server
  - Cloudflare TURN (free tier available)
  - Metered.ca TURN-as-a-service
- TURN adds latency but is necessary for players behind symmetric NAT

### Performance Considerations for WASM

- **State snapshots**: WASM `SimWorld` clone/restore must be fast. Profile in browser DevTools.
- **Re-simulation**: WASM is typically 1.5-3x slower than native Rust. Ensure 7-frame rollback re-sim fits in budget.
- **Memory**: Each snapshot is ~100KB-1MB depending on world complexity. Ring buffer of 7 snapshots = ~1-7MB.
- **Garbage collection**: WASM linear memory doesn't have GC, but frequent allocation/deallocation of snapshots could fragment. Use a pool allocator.

## Native Desktop Multiplayer

For native builds, the same GGRS session works but can use **UDP sockets** instead of WebRTC:

```rust
#[cfg(not(target_arch = "wasm32"))]
use ggrs::UdpNonBlockingSocket;

#[cfg(target_arch = "wasm32")]
use matchbox_socket::WebRtcSocket;
```

This means the `marathon-net` crate (see [[network-sync-for-rust]]) can abstract over both transports with a feature flag.

### Cross-Platform Play (Native <-> Browser)

For native players to connect to browser players, the native client would also need to use Matchbox/WebRTC. This is supported -- Matchbox works on native Rust too, not just WASM.

## Open Questions

1. **Signaling server hosting**: Who pays, where deployed?
2. **TURN relay**: Self-host or use a service?
3. **Room discovery**: How do players find each other? (Share room URL? Lobby browser?)
4. **Player limit**: WebRTC mesh scales poorly beyond 4-6 peers. Use SFU for 8-player games?
5. **Mobile browser support**: Does marathon-web target mobile? WebRTC works on mobile browsers.

## See Also

- [[network-sync-for-rust]] -- GGRS + Matchbox architecture decision
- [[alephone-network-architecture]] -- Original networking for comparison
- [[film-replay-system]] -- Recording multiplayer games in browser
- [Matchbox GitHub](https://github.com/johanhelsing/matchbox)
- [WebRTC Data Channels (MDN)](https://developer.mozilla.org/en-US/docs/Games/Techniques/WebRTC_data_channels)
- [WebRTC P2P Gaming (webrtchacks)](https://webrtchacks.com/datachannel-multiplayer-game/)
- [Using WebRTC for browser multiplayer (DEV.to)](https://dev.to/bornfightcompany/using-webtrc-for-a-browser-multiplayer-game-in-theory-59dk)
