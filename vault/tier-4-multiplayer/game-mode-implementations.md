---
tags: [multiplayer, game-modes, gameplay]
status: implemented
---

# Game Mode Implementations

Marathon supports six game modes: one single-player, one cooperative, and four competitive multiplayer modes. The Rust rebuild has implementations of all six.

## Original Marathon Game Modes

Marathon (and its sequels) define these multiplayer modes, selectable when hosting a network game:

### Every Man for Himself (EMFH)

Classic free-for-all deathmatch.

- **Objective**: Kill other players to score points
- **Scoring**: +1 kill per frag. Deaths tracked but don't subtract score.
- **Win condition**: First player to reach the **kill limit**, or highest kills when **time limit** expires
- **Player count**: 2-8
- **Respawn**: After a short delay (~3 seconds in the Rust rebuild, 90 ticks at 30fps)
- **Map flags**: Levels supporting EMFH have `EntryPointFlags::MULTIPLAYER_CARNAGE` set

### King of the Hill (KOTH)

Area control mode.

- **Objective**: Stand on the designated "hill" polygon to accumulate time
- **Hill location**: A specific polygon on the map, indicated by a compass arrow on the motion sensor
- **Scoring**: Time spent on the hill accumulates per player. Kills/deaths also tracked.
- **Win condition**: First player to reach the **time limit** of hill occupation
- **Contested hill**: If multiple players are on the hill, typically only one scores (first arrival or none)
- **Map flags**: `EntryPointFlags::KING_OF_THE_HILL`

### Kill the Man with the Ball (KTMWTB)

Possession-based scoring. A unique Marathon mode.

- **Objective**: Hold "the ball" (a skull item, `ITEM_THE_BALL`, item type 32) for the longest time
- **Ball mechanics**:
  - Ball spawns on the map as a pickup item
  - Player picks up ball by walking over it (action key)
  - Ball holder **cannot attack** and **cannot run** in original Marathon
  - Ball holder can **drop the ball** voluntarily (fire key)
  - Ball is dropped automatically on death
- **Scoring**: Time holding the ball accumulates. Kills/deaths also tracked.
- **Win condition**: First player to hold the ball for the **time limit**
- **Map flags**: `EntryPointFlags::KILL_THE_MAN_WITH_THE_BALL`

### Tag

"It" mode -- the tagged player accumulates score (trying to avoid being "it").

- **Objective**: Avoid being "it". The tagged player accumulates time (bad).
- **Tag transfer**: When the tagged player is killed, the tag transfers to the killer
- **Initial tag**: First player to die becomes "it"
- **Scoring**: Time spent as "it" accumulates (lower is better)
- **Win condition**: Time limit expires; player with **least** time as "it" wins
- **Note**: In the original Marathon, the scoring is inverted -- being "it" is BAD. The Rust rebuild currently tracks time-as-tagged without the inversion; this needs to be addressed in win condition logic.

### Cooperative

Multi-player campaign.

- **Objective**: Complete single-player levels together
- **Scoring**: Each player's kill count tracked (percentage of total aliens killed)
- **Respawn**: After death, respawn after a longer delay (5 seconds / 150 ticks in Rust rebuild)
- **Level progression**: Standard campaign level sequence, all players advance together
- **Spawn points**: Uses team spawn points if available
- **Map flags**: `EntryPointFlags::MULTIPLAYER_COOPERATIVE`

### Campaign (Single-Player)

Standard single-player mode.

- **Objective**: Complete each level
- **Scoring**: Kill count tracked
- **Respawn**: Instant (delay = 0)
- **Level progression**: Sequential through scenario WAD

### Additional Modes (in Map Format)

The map format defines flags for additional modes not yet fully documented:
- `EntryPointFlags::DEFENSE` (0x20)
- `EntryPointFlags::RUGBY` (0x40)
- `EntryPointFlags::CAPTURE_THE_FLAG` (0x80)

These appear in Marathon Infinity and community scenarios.

## Current State in Rust Rebuild

All six core modes are implemented in `marathon-integration/src/modes/`.

### Module Structure

```
marathon-integration/src/modes/
  mod.rs          -- GameMode trait, PlayerScore, SpawnPoint, WinCheckResult
  campaign.rs     -- CampaignMode
  cooperative.rs  -- CooperativeMode
  deathmatch.rs   -- EveryManForHimself, KingOfTheHill, KillTheManWithTheBall, TagMode
```

### GameMode Trait

Defined in `marathon-integration/src/modes/mod.rs`:

```rust
pub trait GameMode {
    fn on_kill(&mut self, killer: usize, victim: usize);
    fn check_win_condition(&self) -> WinCheckResult;
    fn get_spawn_point(&self, player_id: usize, spawn_points: &[SpawnPoint]) -> Option<usize>;
    fn respawn_delay(&self) -> u32;
    fn scores(&self) -> &[PlayerScore];
}
```

### PlayerScore

```rust
pub struct PlayerScore {
    pub player_id: usize,
    pub kills: u32,
    pub deaths: u32,
    pub time_score: f64,  // Used by KOTH, KTMWTB, Tag
}
```

### WinCheckResult

```rust
pub enum WinCheckResult {
    InProgress,
    Winner(usize),
    TimeLimitReached,
    LevelComplete,  // Campaign/Coop
}
```

### SpawnPoint

```rust
pub struct SpawnPoint {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub polygon_index: usize,
    pub facing: u16,
    pub team: Option<usize>,
}
```

### Implementation Details

| Mode | Constructor | Special Methods | Respawn Delay |
|------|-------------|-----------------|---------------|
| `EveryManForHimself` | `new(num_players, kill_limit)` | -- | 90 ticks (3s) |
| `KingOfTheHill` | `new(num_players, hill_polygon, time_limit)` | `award_hill_time(player, secs)` | 90 ticks (3s) |
| `KillTheManWithTheBall` | `new(num_players, time_limit)` | `pickup_ball(player)`, `drop_ball()`, `award_possession_time(secs)` | 90 ticks (3s) |
| `TagMode` | `new(num_players, time_limit)` | `set_tagged(player)`, `award_tag_time(secs)` | 90 ticks (3s) |
| `CooperativeMode` | `new(starting_level, total_levels, num_players)` | `mark_level_complete()` | 150 ticks (5s) |
| `CampaignMode` | `new(starting_level, total_levels)` | `mark_level_complete()`, `advance_level()` | 0 (instant) |

### Key Behaviors

**Tag transfer**: When victim is the tagged player, tag transfers to killer:
```rust
fn on_kill(&mut self, killer: usize, victim: usize) {
    // ... score tracking ...
    if self.tagged_player == Some(victim) {
        self.tagged_player = Some(killer);
    }
}
```

**Ball drop on death**: KTMWTB drops the ball when the holder is killed:
```rust
fn on_kill(&mut self, killer: usize, victim: usize) {
    // ... score tracking ...
    if self.ball_holder == Some(victim) {
        self.ball_holder = None;
    }
}
```

**Cooperative spawn**: Uses team spawn points if available:
```rust
fn get_spawn_point(&self, _player_id: usize, spawn_points: &[SpawnPoint]) -> Option<usize> {
    spawn_points.iter().position(|sp| sp.team.is_some())
        .or(if spawn_points.is_empty() { None } else { Some(0) })
}
```

## What Needs Work

### Spawn Point Selection

Currently all modes return `Some(0)` (first spawn point) as a placeholder. Proper spawn selection needs:
- **Random selection** from available spawn points (using `SimRng` for determinism)
- **Distance-based selection**: Prefer spawn points far from enemies (telefrag avoidance)
- **Team-based selection**: Cooperative/team modes use team-specific spawn points
- **Map object flags**: Respect `MapObjectFlags::NETWORK_ONLY` for multiplayer-only spawns

### Time Limit Support

`WinCheckResult::TimeLimitReached` exists but no mode currently checks elapsed time against a game-wide time limit. Need:
- `elapsed_time` field on modes (incremented each tick)
- `time_limit` configuration parameter
- Time limit check in `check_win_condition()`

### Tag Scoring Inversion

In original Marathon, Tag is a "lowest score wins" mode. The current implementation tracks time-as-tagged as a positive score. The win condition check should compare for the **minimum** time score, or the UI should display it as "time to avoid."

### Team Modes

Team variants of the competitive modes are not yet implemented:
- Team EMFH (team deathmatch)
- Team KOTH
- Team scoring aggregation

### Item Respawn

Multiplayer modes need item respawn timers. The `ItemRespawnState` struct exists in `marathon-sim/src/world_mechanics/items.rs` but isn't wired into the game modes yet. Items should respawn after a configurable delay in competitive modes.

### Score Display / Intermission

The intermission screen (between levels or at game end) needs to display scores. The `GameMode::scores()` method provides the data; the UI rendering needs implementation.

### Map Entry Point Filtering

When setting up a multiplayer game, the map selection should filter levels based on `EntryPointFlags`. A KOTH map must have `EntryPointFlags::KING_OF_THE_HILL`, etc. The flags are already parsed in `marathon-formats/src/map.rs`.

## Key Files

- `marathon-integration/src/modes/mod.rs` -- Trait and shared types
- `marathon-integration/src/modes/campaign.rs` -- Single-player campaign
- `marathon-integration/src/modes/cooperative.rs` -- Cooperative mode
- `marathon-integration/src/modes/deathmatch.rs` -- All four competitive modes
- `marathon-integration/src/types.rs` -- `GameModeType` enum
- `marathon-formats/src/map.rs` -- `EntryPointFlags` for map filtering
- `marathon-sim/src/world_mechanics/items.rs` -- Item respawn, ball item types

## See Also

- [[alephone-network-architecture]] -- Networking needed for multiplayer modes
- [[film-replay-system]] -- Film records game mode in header
- [[control-remapping]] -- Input handling for all modes
