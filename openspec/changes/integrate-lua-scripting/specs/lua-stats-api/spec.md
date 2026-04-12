## ADDED Requirements

### Requirement: Stats script callback dispatch
The system SHALL call stats script callbacks at appropriate game events when a stats script is loaded. The stats VM SHALL receive the following callbacks: `got_kill(aggressor, victim, damage_type)` when a player or monster is killed, `player_damaged(victim, aggressor, damage_type, amount)` when a player takes damage, and `game_ended()` when the game session ends (level complete, player death, or quit).

#### Scenario: Kill recorded
- **WHEN** player 0 kills a monster of type 3 with damage type 5
- **THEN** the system SHALL call `got_kill(aggressor, victim, damage_type)` in the stats VM where `aggressor` is a Player UserData and `victim` is a Monster UserData

#### Scenario: Player damage recorded
- **WHEN** a player takes 30 points of damage type 2
- **THEN** the system SHALL call `player_damaged(victim, aggressor, damage_type, 30)` in the stats VM

#### Scenario: Game ended
- **WHEN** the level ends (player completes objective or dies)
- **THEN** the system SHALL call `game_ended()` in the stats VM

### Requirement: Stats script draw callback
The system SHALL call `draw()` in the stats VM during the post-game stats screen (Intermission or GameOver state). The stats `draw()` function SHALL have access to the same Screen drawing API as the HUD script (fill_rect, draw_text, etc.) to render custom statistics displays.

#### Scenario: Stats draw on intermission screen
- **WHEN** the game transitions to `Intermission` state with a stats script loaded
- **THEN** the system SHALL call `draw()` in the stats VM each frame to render the stats display

### Requirement: Stats scripts are read-only
The stats VM SHALL only have read-only access to game objects. Stats scripts SHALL NOT be able to modify any game state (health, positions, etc.). Collection accessors (Players, Monsters, etc.) SHALL be available for reading only.

#### Scenario: Stats script reads player data
- **WHEN** a stats script accesses `Players[0].health`
- **THEN** the system SHALL return the player's current health value

#### Scenario: Stats script cannot write
- **WHEN** a stats script attempts `Players[0].health = 999`
- **THEN** the system SHALL raise a Lua error indicating stats scripts cannot modify game state

### Requirement: Stats data accumulation
Stats scripts SHALL be able to use Lua tables and variables to accumulate statistics data across the game session. The stats VM's global state SHALL persist from the first callback until `game_ended()` is called. After `game_ended()`, the accumulated data SHALL be available for the `draw()` function to render.

#### Scenario: Accumulate kill count
- **WHEN** a stats script initializes `kills = 0` and increments it in each `got_kill` callback
- **THEN** the `kills` variable SHALL reflect the total count when `draw()` is called at game end
