---
tags: [multiplayer, networking, alephone-reference, architecture]
status: research-complete
---

# Alephone Network Architecture

How the original alephone engine handles multiplayer networking. This is the reference point for the Rust rebuild's networking design.

## Overview

Marathon's multiplayer is built on **deterministic lockstep** networking. Every player runs the identical simulation. Instead of synchronizing world state, players exchange only their **action flags** (a compact bitmask of inputs) each tick. If all players receive the same inputs in the same order, their simulations stay in sync.

## Two Protocol Topologies

Alephone supports two distinct network topologies:

### Ring Protocol (Original Bungie)

The original Marathon 2 networking used AppleTalk in a **ring topology**. Alephone ported this to IP.

- Players are arranged in a logical ring
- Each player sends their action flags to the next player in the ring
- Flags propagate around the ring until every player has every other player's flags for that tick
- **Latency**: All players experience equal lag (the full ring round-trip time)
- **Bandwidth**: Each station has equal, fairly low throughput requirements
- **Failure mode**: If any player disconnects, the ring breaks

**How it works per tick**:
1. Player gathers local action flags from input
2. Player sends a packet containing their flags to the next node in the ring
3. Each node appends its own flags and forwards the growing packet
4. When the packet returns to the originator, all players' flags are known
5. Simulation advances one tick with the complete flag set

### Star Protocol (Alephone Addition)

Alephone added a **star topology** as the default protocol, dramatically improving Internet play:

- One player acts as the **hub** (gatherer/host)
- All other players are **spokes** that communicate only with the hub
- The hub collects action flags from all spokes, then broadcasts the combined flags back
- **Latency**: Each player's lag depends only on their RTT to the hub (not the worst player's connection)
- **Bandwidth**: Spokes need minimal bandwidth; the hub needs more (estimated to support 5-6 players on consumer DSL/cable)
- **Failure mode**: Hub failure kills the game; spoke failure only removes that player

**Star protocol advantages**:
- Much lower latency for most players vs ring
- Better tolerance of one player having a bad connection (only they lag)
- Simpler NAT traversal (all connections are to/from one host)

## Action Flags

The fundamental unit of network exchange is the **action flag set** -- a bitmask representing all player inputs for one simulation tick.

### Original Marathon Action Flags (from `vbl.cpp`)

```
action_flags_t (uint32):
  bit 0:  move forward
  bit 1:  move backward
  bit 2:  turn/sidestep left
  bit 3:  turn/sidestep right
  bit 4:  look/glance left (or strafe modifier)
  bit 5:  look/glance right (or strafe modifier)
  bit 6:  look up
  bit 7:  look down
  bit 8:  primary trigger (fire)
  bit 9:  secondary trigger (alt-fire)
  bit 10: action key
  bit 11: cycle weapons forward
  bit 12: cycle weapons backward
  bit 13: toggle map
  bit 14: microphone button
  bits 15-31: encoded delta yaw/pitch for mouse look
```

### Network Distribution Buffer

The **network distribution buffer** (`action_queue`) is a circular buffer per player that accumulates action flags:

- During network play, remote players' flags arrive and are enqueued
- The simulation reads flags from all players' queues each tick
- If a remote player's queue is empty (flags haven't arrived yet), the simulation **stalls** (lockstep wait)
- The buffer provides a small amount of jitter absorption

Key C++ structures in alephone:
- `action_queue` -- per-player circular buffer of `action_flags_t`
- `recording_queues` -- used for film recording, stores flags as they're consumed
- `network_distribution_buffer` -- the shared buffer for exchanging flags across the network

### Recording Integration

The network and recording systems share the same action flag pipeline:
1. Input is gathered into local `action_flags_t`
2. Flags are sent to network AND recorded to film buffer
3. Remote flags arrive from network and are recorded too
4. Simulation consumes flags from the combined queue

This means **films are inherently multiplayer-aware** -- a multiplayer film records all players' flags.

## Packet Format

Alephone network packets carry:
1. **Packet type** header (ring-specific or star-specific)
2. **Tick number** for synchronization
3. **Player index** identifying the source
4. **Action flags** payload (one or more ticks' worth, for bandwidth efficiency)
5. **CRC/checksum** for integrity

In the ring protocol, packets grow as they traverse the ring (each node appends their flags). In the star protocol, spokes send small packets (just their own flags) and the hub broadcasts combined packets.

## Synchronization Mechanism

Marathon enforces **strict lockstep**:
- The simulation cannot advance tick N+1 until all players' flags for tick N are available
- If flags are late, the simulation freezes (appears as lag/stutter to all players)
- There is no prediction, interpolation, or rollback in the original engine
- The game runs at 30 ticks/second; each tick must complete within 33ms wall clock + network wait

### Desync Detection

Because the simulation is deterministic, desyncs indicate a bug. Alephone includes:
- Periodic **world checksum** comparison (hash of key game state)
- If checksums diverge, the game reports a sync error
- Version compatibility checks prevent mismatched builds from connecting (`kGameworldVersion`)

## Source Files Reference

Key alephone source files for networking:
- `Source_Files/Network/network.cpp` -- core networking logic
- `Source_Files/Network/network_star.cpp` -- star protocol implementation
- `Source_Files/Network/network_ring.cpp` -- ring protocol implementation
- `Source_Files/Network/network_udp.cpp` -- UDP transport layer
- `Source_Files/Network/network_capabilities.cpp` -- version/feature negotiation
- `Source_Files/Misc/vbl.cpp` -- vertical blank handler, action flag gathering, recording queues
- `Source_Files/Misc/sdl_network.h` -- SDL networking headers

## Current State in Rust Rebuild

**No networking code exists yet.** The foundation is in place:

- `ActionFlags` bitflags defined in `marathon-integration/src/types.rs` (15 bits, matching Marathon)
- `ActionFlags` also defined in `marathon-sim/src/tick.rs` for the sim layer
- `TickInput` resource injected per tick with action flags + mouse deltas
- `SimWorld::tick(input)` advances the sim deterministically
- `SimRng` seeded PRNG ensures deterministic random numbers
- Film system records/replays action flags per tick

The Rust rebuild's action flag layout matches the original Marathon format bit-for-bit (bits 0-14).

### Key Rust Files

- `marathon-integration/src/types.rs` -- `ActionFlags` bitflags, `GameModeType`
- `marathon-sim/src/tick.rs` -- `TickInput`, `ActionFlags` (sim layer), `SimWorld::tick()`
- `marathon-sim/src/world.rs` -- `SimWorld`, `SimRng`, `TickCounter`, `SimConfig`
- `marathon-integration/src/shell/film.rs` -- film recording/playback

## Implications for Rust Networking

The original architecture maps well to the Rust rebuild:

1. **Action flags are already the sim's input interface** -- no refactoring needed
2. **Deterministic sim with seeded RNG** -- same guarantees as original
3. **30 tick/s fixed timestep** -- same as original
4. **Star topology** is the natural choice for a modern implementation
5. **Film recording already works** -- just needs multiplayer flag serialization

The main difference: the Rust rebuild should use **rollback** instead of pure lockstep to hide latency. See [[network-sync-for-rust]].

## See Also

- [[network-sync-for-rust]] -- Modern networking approaches for the Rust rebuild
- [[film-replay-system]] -- How the film system records and replays action flags
- [[game-mode-implementations]] -- Multiplayer game modes that depend on networking
- [Alephone Networking Users Guide](https://github.com/Aleph-One-Marathon/alephone/blob/master/docs/Networking%20Users%20Guide.txt)
- [Alephone Developer Notes](https://github.com/Aleph-One-Marathon/alephone/wiki/Developer-Notes)
