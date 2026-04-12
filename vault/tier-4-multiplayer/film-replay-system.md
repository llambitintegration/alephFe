---
tags: [multiplayer, film, replay, deterministic]
status: implemented-single-player
---

# Film/Replay System

Marathon's film system records player action flags per tick, then replays by feeding those flags into the deterministic simulation. The Rust rebuild already has a working single-player implementation.

## How the Original Alephone Handles It

### Recording

In alephone, film recording is tightly integrated with the action flag pipeline:

1. **Recording trigger**: The player enables "Record Film" before starting a game (or it is always-on in some configurations)
2. **Per-tick capture**: During `vbl.cpp`'s vertical blank handler, after action flags are gathered from input (and from network in multiplayer), the flags are written to the `recording_queues`
3. **Chunk-based storage**: Flags are accumulated in memory and periodically flushed to disk in chunks via `save_recording_queue_chunk()`
4. **Multiplayer recording**: In network games, ALL players' action flags are recorded, not just the local player's. The film contains N tracks of flags (one per player)
5. **End trigger**: Recording stops when the game ends (level complete, death, or explicit stop)

### Film File Format

Alephone film files (`.filA` extension) use the WAD container format:

- **WAD header**: Standard Marathon WAD header with magic number, version, entry count
- **Film header entry**: Contains metadata:
  - Map checksum (to verify correct scenario)
  - Difficulty level
  - Game mode (solo, cooperative, net carnage, etc.)
  - Number of players
  - Player starting positions
  - Random seed
  - Game tick count
- **Action flag entries**: Stored as sequential chunks of `action_flags_t` (uint32) per player per tick
  - In multiplayer: interleaved or sequential per-player blocks
  - `RECORD_CHUNK_SIZE` determines how many ticks per chunk

### Playback

1. **Film loading**: Parse WAD, extract header and action flag data
2. **World initialization**: Load the same map, apply same difficulty, seed RNG with recorded seed
3. **Tick replay**: Each tick, read the next set of action flags from the film data (instead of from input/network)
4. **Camera**: During playback, the camera follows player 0 by default; in multiplayer films, the viewer can switch between players
5. **Fast-forward**: Skip rendering for N ticks (just run `tick()` without drawing), then resume rendering
6. **End**: When all recorded ticks are consumed, playback ends

### Determinism Requirements

For film playback to work, the simulation must be **bit-identical** given the same inputs and seed:
- Same map data
- Same physics model
- Same RNG sequence
- Same action flags
- Same tick order

This is why Marathon's sim runs at a fixed 30 ticks/second with no floating-point non-determinism.

## Current State in Rust Rebuild

The film system is **already implemented** for single-player in `marathon-integration/src/shell/film.rs`.

### `FilmHeader`

```rust
pub struct FilmHeader {
    pub version: u32,           // Format version (currently 1)
    pub level_index: usize,     // Level within scenario WAD
    pub difficulty: Difficulty,  // Difficulty setting
    pub game_mode: GameModeType, // Campaign, Cooperative, etc.
    pub random_seed: u64,       // RNG seed for deterministic replay
}
```

### `FilmData`

```rust
pub struct FilmData {
    pub header: FilmHeader,
    pub ticks: Vec<ActionFlags>,  // One ActionFlags per tick, in order
}
```

### `FilmRecorder`

Records action flags during gameplay:
- `FilmRecorder::new(level_index, difficulty, game_mode, random_seed)` -- start recording
- `recorder.record_tick(flags)` -- record one tick's flags
- `recorder.finish() -> FilmData` -- stop and return completed film

### `FilmPlayer`

Plays back recorded films:
- `FilmPlayer::new(film) -> Self` -- load film for playback
- `player.header()` -- get metadata for level initialization
- `player.next_tick() -> Option<ActionFlags>` -- get next tick's flags
- `player.is_finished()` -- check if playback complete
- `player.current_tick()` / `player.total_ticks()` -- position tracking

### Serialization

Binary serialization via `bincode`:
- `serialize_film(&FilmData) -> Result<Vec<u8>, FilmError>`
- `deserialize_film(&[u8]) -> Result<FilmData, FilmError>`
- Version checking on deserialization (rejects mismatched versions)

### Key File

`marathon-integration/src/shell/film.rs` -- Complete implementation with tests.

## What Needs Extension for Multiplayer

### Multi-Player Action Flags

Currently `FilmData.ticks` stores `Vec<ActionFlags>` -- a single player's flags per tick. For multiplayer, this needs to become `Vec<Vec<ActionFlags>>` (or `Vec<[ActionFlags; N]>`) -- one flag set per player per tick.

Proposed change:

```rust
pub struct FilmData {
    pub header: FilmHeader,
    pub player_count: usize,
    /// ticks[tick_index][player_index] = ActionFlags for that player at that tick
    pub ticks: Vec<Vec<ActionFlags>>,
}
```

### Header Extensions

The `FilmHeader` needs:
- `player_count: usize` -- number of players
- `player_names: Vec<String>` -- for display during playback
- `map_checksum: u32` -- to verify correct scenario on load (like alephone)

### Playback Controls

Additional playback features to implement:
- **Camera switching**: In multiplayer films, switch which player the camera follows
- **Fast-forward**: Run N ticks without rendering (sim-only), then resume
- **Rewind**: Re-initialize world from header, re-play up to target tick
- **Pause**: Stop tick advancement, allow camera movement
- **Playback speed**: 0.5x, 1x, 2x, 4x speed multipliers

### Integration with GGRS

When using rollback networking (see [[network-sync-for-rust]]), film recording should tap into **confirmed inputs** (not predicted inputs):

1. GGRS confirms remote inputs for tick N
2. Confirmed inputs for all players at tick N are recorded to film
3. This ensures the film contains only verified, correct inputs
4. Playback doesn't need rollback -- just feed confirmed inputs sequentially

## Architecture

```
Input Source (gameplay or network)
       |
       v
+------------------+
| ActionFlags      | (per-player, per-tick)
+------------------+
       |
       +----> FilmRecorder.record_tick(flags)  --> FilmData --> serialize --> .film file
       |
       v
SimWorld.tick(input)
       |
       v
    Render

--- During Playback ---

.film file --> deserialize --> FilmData
                                |
                                v
                    FilmPlayer.next_tick() --> ActionFlags
                                |
                                v
                        SimWorld.tick(input)
                                |
                                v
                              Render
```

## Testing

Existing tests in `film.rs` cover:
- Record and playback round-trip (3 ticks with various flags)
- Serialize/deserialize round-trip via bincode
- Empty film edge case
- Version mismatch detection

Additional tests needed:
- Multi-player film recording/playback
- Fast-forward (skip N ticks, verify final state matches sequential playback)
- Large film (thousands of ticks) performance
- Determinism verification: record + play back, compare final world state

## See Also

- [[alephone-network-architecture]] -- How action flags flow through the network
- [[network-sync-for-rust]] -- GGRS confirmed inputs feed into film recording
- [[game-mode-implementations]] -- Game modes affect film metadata
- [Demo recording/playback for FPS games (GameDev.net)](https://gamedev.net/forums/topic/323042-demo-recordingplayback-for-fps-games/)
- [Developing Your Own Replay System (Gamedeveloper.com)](https://www.gamedeveloper.com/programming/developing-your-own-replay-system)
