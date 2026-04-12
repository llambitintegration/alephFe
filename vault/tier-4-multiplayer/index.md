---
tags: [multiplayer, networking, index]
---

# Tier 4: Multiplayer & Advanced

Research notes for networking, replays, game modes, and input configuration in the Rust Marathon engine rebuild.

## Documents

- [[alephone-network-architecture]] -- Original alephone peer-to-peer lockstep, ring & star topologies, action flag exchange, network distribution buffer
- [[network-sync-for-rust]] -- Modern networking architectures for a deterministic sim: lockstep vs rollback vs client-server, Rust crate comparison (quinn, ggrs, matchbox, naia, laminar)
- [[film-replay-system]] -- Film recording & playback: action flags per tick, deterministic replay, file format, fast-forward, multiplayer film support
- [[game-mode-implementations]] -- All Marathon multiplayer modes: Every Man For Himself, King of the Hill, Kill The Man With The Ball, Tag, Cooperative. Scoring rules, spawn logic, win conditions
- [[webrtc-browser-multiplayer]] -- WebRTC data channels for browser-to-browser multiplayer via the marathon-web crate, matchbox signaling, STUN/TURN, architecture
- [[control-remapping]] -- Input binding system, preferences persistence, key/mouse/gamepad configuration, rebinding UI

## Architecture Decision

The single biggest open question for Tier 4 is the **networking model**. The Rust rebuild's 30 tick/s deterministic simulation with seeded RNG is a perfect fit for either lockstep or rollback. See [[network-sync-for-rust]] for the full comparison.

**Recommended path**: Start with GGRS rollback + Matchbox WebRTC transport. This gives:
1. Native desktop P2P with low perceived latency (rollback hides network delay)
2. Browser-to-browser play via WebRTC data channels (marathon-web)
3. Film recording "for free" since the sim is already deterministic and action-flag driven

## Dependency Graph

```
                    +---------------------------+
                    | network-sync-for-rust     |
                    | (architectural decision)  |
                    +---------------------------+
                       /           |           \
                      v            v            v
  +------------------+  +-----------------+  +------------------------+
  | alephone-network |  | webrtc-browser  |  | film-replay-system     |
  | -architecture    |  | -multiplayer    |  | (already implemented!) |
  +------------------+  +-----------------+  +------------------------+
                                               |
                      +------------------------+
                      v                        v
          +---------------------+   +-------------------+
          | game-mode-           |   | control-remapping |
          | implementations     |   | (already started) |
          +---------------------+   +-------------------+
```

## Current Status

| Topic | Status | Key Files |
|-------|--------|-----------|
| Network Architecture | Research only | (no networking code yet) |
| Network Sync | Research only | (no networking code yet) |
| Film/Replay | Implemented (single-player) | `marathon-integration/src/shell/film.rs` |
| Game Modes | Implemented (4 modes + campaign/coop) | `marathon-integration/src/modes/` |
| WebRTC Browser | Research only | `marathon-web/src/lib.rs` (no networking) |
| Control Remapping | Partially implemented | `marathon-integration/src/input/` |

## Related Vault Notes

- [[architecture/ecs-architecture]] -- How bevy_ecs is used; relevant to rollback state save/restore
- [[architecture/game-loop-and-state-machine]] -- Tick accumulation, state transitions
- [[architecture/data-flow]] -- How action flags flow from input to simulation
