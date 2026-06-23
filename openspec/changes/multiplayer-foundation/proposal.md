## Why

The engine's networking story today is a single design (`implement-networking-multiplayer`) that bolts **GGRS peer-to-peer rollback over WebRTC/Matchbox** onto the existing single-process game shell. That model is excellent for a small, symmetric, deterministic deathmatch — but it is the wrong foundation for the project's stated north star (the [[mode-b-multiclient-access]] / [[roadmap-mode-a-to-b]] vision): a **single headless authoritative server** that both human browser clients *and* slow AI agents inhabit as equal "submit-intent / receive-scoped-observation" clients. P2P rollback has no authoritative arbiter, no server-side validation, no observation-scoping trust boundary, and "gracefully ingests slow AI agents" is precisely what rollback cannot do.

This change establishes the **client-server authoritative** spine (the Quake/Source model) the roadmap calls for, in two phases: Phase 1 splits the sim into a headless server (`loop { ingest; tick; broadcast }`) plus thin clients with client-side prediction/reconciliation over a WebSocket browser path; Phase 2 adds the **per-client LOS-scoped / interest-managed delta** — *the security boundary that all of Mode B later rests on* — plus WebTransport datagrams, auth/sessions, and roles. Authoritative-with-scoped-obs is the only model that later admits untrusted agents safely, so doing it now avoids a second rewrite.

## What Changes

### Phase 1 — Headless authoritative server + single remote human

- **Headless server loop.** Build a standalone server binary/crate running `loop { ingest_intents(); tick(); broadcast_snapshot(); }` at the fixed 30Hz clock, owning the world clock and reusing `marathon-sim`'s `tick(inputs)` + serializable `render_snapshot()` (the prerequisite delivered by `decouple-tick-snapshot`).
- **Netcode transport.** Adopt a standalone (non-Bevy) transport — **renet/renet2 (recommended) or aeronet** with quinn/wtransport underneath — providing reliable + unreliable channels and secure connect. Reserve lightyear/bevy_replicon only as a turnkey path *if* the engine ever migrates to Bevy.
- **WebSocket browser path first.** A WASM thin client connects over WebSocket (TCP/TLS, port 443, no NAT) as the universal, easiest-to-host transport before WebTransport/WebRTC.
- **Server validates all intents.** Clients send **intent only** (`ActionFlags` / high-level commands), never asserted state; the server is the sole authority on resulting world state. **BREAKING** for any assumption that the game shell owns an in-process authoritative `SimWorld`.
- **Client-side prediction + reconciliation.** The thin client predicts the local player from local input, then reconciles against authoritative server snapshots (Gambetta model: input sequence numbers, replay-unacknowledged-inputs on correction), with snapshot interpolation for remote entities.

### Phase 2 — Multi-human shared world (the security boundary)

- **Per-client interest management / LOS-scoped deltas.** Each client receives only what its avatar can see (LOS/AOI) computed against `marathon-sim` spatial data — never a full-world dump. **This is the load-bearing security boundary** (VALORANT server-side Fog-of-War threat model): an over-broad observation is instantly weaponizable (wallhacks, perfect radar), and once Mode B agents arrive they consume the observation payload directly as structured input, so the obs payload — not just the action API — must be a trust boundary from day one.
- **Changed-state deltas.** Broadcast per-client *deltas* against the prior acknowledged snapshot rather than full snapshots, scoped by the interest set above.
- **WebTransport datagram path.** Add WebTransport (HTTP/3/QUIC) for unreliable datagrams (movement/position updates) alongside reliable streams, as a higher-performance browser path over the Phase 1 WebSocket fallback.
- **Auth + sessions.** Humans authenticate via server-side sessions (HttpOnly/Secure cookies); the transport reserves a slot for short-lived scoped tokens (consumed fully by Mode B agents later). Per-connection rate limiting (`governor` / `tower-governor`) keyed by authenticated identity, fronted by Caddy for TLS.
- **Roles (RBAC).** `spectator` (receives scoped read-only obs, input ignored) vs `participant` (full intent submission); `admin/referee` reserved. Spectators consume no player slot.

## Capabilities

### New Capabilities

- `headless-authoritative-server`: The standalone, render-free server that owns the world clock and runs `loop { ingest; tick; broadcast }` over `marathon-sim`, validating every client intent and producing authoritative snapshots. The single source of world truth for all client types.
- `netcode-transport`: The standalone client-server transport layer (renet/renet2 or aeronet over quinn/wtransport) with reliable + unreliable channels, secure connect, and pluggable browser paths (WebSocket in Phase 1, WebTransport in Phase 2). Intent-up / snapshot-down message contract.
- `client-prediction`: Thin-client client-side prediction and server reconciliation — local input prediction, input-sequence acknowledgement, replay-on-correction, and snapshot interpolation for remote entities.
- `interest-management`: Per-client LOS/AOI scoping and changed-state delta generation against `marathon-sim` spatial data. The observation-payload trust boundary; clients receive only avatar-visible state, never a full-world dump.
- `client-auth-roles`: Authentication (human sessions; scoped-token slot), per-identity rate limiting, and RBAC roles (spectator / participant / admin-referee) governing what a connection may submit and receive.

### Modified Capabilities

- `game-loop`: `SimWorld` is driven by an external authoritative server loop (`ingest → tick → broadcast`) rather than an in-process game-shell loop; the multiplayer tick consumes one intent per connected participant slot. (Builds on the `tick(inputs)` / `render_snapshot()` shape from `decouple-tick-snapshot`.)
- `input-system`: In server mode the local client's input becomes a *predicted intent* submitted to the server and acknowledged by sequence number, instead of being applied directly to a local authoritative sim.
- `game-shell`: The shell gains a remote-client mode that connects to a headless server (and a server-launch path), parallel to the existing single-player in-process path.

## Impact

- **Depends on `decouple-tick-snapshot` (hard prerequisite).** Phase 1 cannot begin until `marathon-sim` exposes a clean headless `tick(inputs)` + serializable, scopeable `render_snapshot()` decoupled from rendering. The bincode round-trip landed for snapshots is the broadcast substrate; interest-management requires the snapshot to be *scopeable* by spatial visibility.
- **New crate(s):** a `marathon-server` (headless authoritative loop + transport host) and a client netcode module shared by `marathon-web`/`marathon-game`. New dependencies: `renet`/`renet2` (or `aeronet`) + `quinn`/`wtransport`, `axum` + `tokio-tungstenite` (WS host), `governor`/`tower-governor`, session/JWT crates; Caddy for TLS deployment.
- **`marathon-sim`:** must support multi-participant intent ingestion per tick and expose spatial data sufficient for LOS/AOI scoping; no rendering coupling.
- **`marathon-web` / `marathon-game`:** gain thin-client mode (connect, predict, reconcile, interpolate) feeding input to the server instead of an in-process sim.
- **Deployment:** new server process fronted by Caddy (auto-HTTPS, WS proxy) on the existing Docker infra; WebTransport adds an HTTP/3 UDP path in Phase 2.

### Scope reconciliation vs `implement-networking-multiplayer`

These are **two different network models and must not be merged**:

- `implement-networking-multiplayer` = **peer-to-peer GGRS rollback over WebRTC/Matchbox**, in-process, symmetric players, with lobby/matchmaking, desync-checksumming, and GGRS-style spectators. No authoritative server, no server-side intent validation, no observation scoping.
- `multiplayer-foundation` (this change) = the **Mode-A/B-aligned client-server *authoritative* refactor**: a headless server owns the clock and validates all intents, clients predict/reconcile, and Phase 2 introduces the **LOS-scoped-observation security boundary** that Mode B's untrusted agents later require.

Reconciliation rules:
- This change is the **canonical foundation** for the roadmap (Mode B depends on authoritative + scoped-obs, which rollback cannot provide). The existing change's lobby/matchmaking, desync-detection, and roles concepts are reusable references but are re-homed onto the authoritative server here (auth/roles → `client-auth-roles`; desync-detection becomes server-side validation).
- `implement-networking-multiplayer` should be **superseded or re-scoped to a P2P-deathmatch sub-mode** layered *on top of* this authoritative foundation (rollback as an optional client-side prediction strategy for symmetric matches), not as the base transport. Where the two overlap (multi-player `tick(inputs)`, save/restore, game-mode wiring), this change **defers to / shares** the `game-loop` modifications rather than redefining them. No capability names are duplicated: `netcode-transport` (client-server) is distinct from that change's `networking-transport` (WebRTC P2P); `client-prediction` is distinct from `rollback-netcode`.
