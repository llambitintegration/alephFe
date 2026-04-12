## ADDED Requirements

### Requirement: Non-participating spectator connections
The system SHALL support spectator connections that observe gameplay without participating. Spectators SHALL receive the confirmed input stream from all players via a GGRS spectator session. Spectators SHALL NOT contribute inputs and SHALL NOT occupy a player slot. Spectator connections SHALL have no impact on rollback behavior for active players.

#### Scenario: Spectator joins active game
- **WHEN** a spectator connects to an active game session
- **THEN** the spectator SHALL begin receiving confirmed inputs and SHALL start running the simulation from the beginning of the match (or from a recent checkpoint)

#### Scenario: Spectator does not affect rollback
- **WHEN** a spectator is connected and a player's input arrives late
- **THEN** only the active players SHALL experience rollback; the spectator SHALL continue receiving confirmed frames at the confirmed-frame rate

#### Scenario: Spectator disconnects
- **WHEN** a spectator disconnects from the session
- **THEN** the active game SHALL be unaffected and no player slot SHALL be freed (none was occupied)

### Requirement: Spectator camera modes
Spectators SHALL have two camera modes: free camera (WASD + mouse to fly freely through the level) and player-follow camera (locked to a selected player's first-person perspective). Spectators SHALL be able to switch between modes and cycle through players.

#### Scenario: Free camera mode
- **WHEN** the spectator selects free camera mode
- **THEN** the spectator's camera SHALL be controllable via WASD movement and mouse look, independent of any player's position

#### Scenario: Player-follow camera mode
- **WHEN** the spectator selects player-follow mode and targets player slot 2
- **THEN** the spectator's camera SHALL render from player 2's first-person perspective (position, facing, and look angle)

#### Scenario: Cycle followed player
- **WHEN** the spectator presses the cycle key while in player-follow mode
- **THEN** the camera SHALL switch to the next active player's perspective

### Requirement: Spectator simulation runs at confirmed frame rate
The spectator's local simulation SHALL run in lockstep with confirmed frames (no prediction, no rollback). The spectator SHALL always display the authoritative game state. If confirmed frames arrive faster than the display rate, the spectator SHALL catch up. If confirmed frames are delayed, the spectator SHALL pause the simulation until new confirmed frames arrive.

#### Scenario: Confirmed frames arrive on time
- **WHEN** confirmed frames arrive at the expected rate (30 fps)
- **THEN** the spectator's simulation SHALL advance at 30 ticks per second and render smoothly

#### Scenario: Confirmed frame delay
- **WHEN** confirmed frames are delayed by 200 ms
- **THEN** the spectator's simulation SHALL pause until new confirmed frames arrive, then advance to catch up

#### Scenario: Spectator catches up after delay
- **WHEN** a burst of confirmed frames arrives after a delay
- **THEN** the spectator SHALL advance through the queued frames at accelerated speed until caught up, then resume normal playback rate
