## MODIFIED Requirements

### Requirement: Game state machine with Lobby and PostGame states
The system SHALL add a `Lobby` state and a `PostGame` state to the existing state machine. The updated transitions SHALL include:
- `MainMenu` -> `Lobby` (on "Multiplayer" selected)
- `Lobby` -> `Loading` (on synchronized game start)
- `Lobby` -> `MainMenu` (on leave lobby or lobby dissolved)
- `Playing` -> `PostGame` (on win condition met or all remote players disconnected)
- `PostGame` -> `MainMenu` (on acknowledgement)
- `PostGame` -> `Lobby` (on "Play Again" with same group)
All existing single-player transitions SHALL remain unchanged. Single-player campaign SHALL bypass `Lobby` entirely.

#### Scenario: Enter multiplayer lobby from main menu
- **WHEN** the player selects "Multiplayer" from the main menu
- **THEN** the state SHALL transition from `MainMenu` to `Lobby`

#### Scenario: Lobby to gameplay
- **WHEN** all players are ready and the host starts the game
- **THEN** the state SHALL transition from `Lobby` to `Loading`, then to `Playing` after level initialization

#### Scenario: Win condition reached in multiplayer
- **WHEN** the active `GameMode` reports a winner or the time limit expires during multiplayer `Playing`
- **THEN** the state SHALL transition from `Playing` to `PostGame`, displaying the final scoreboard

#### Scenario: All remote players disconnect during gameplay
- **WHEN** all remote players disconnect during multiplayer `Playing`
- **THEN** the state SHALL transition from `Playing` to `PostGame` with a "session ended" message

#### Scenario: Single-player bypasses Lobby
- **WHEN** the player selects "New Game" (single-player campaign) from the main menu
- **THEN** the state SHALL transition directly from `MainMenu` to `Loading`, bypassing `Lobby`

### Requirement: Multiplayer game loop integration
In multiplayer mode, the game loop SHALL feed local input to `marathon-net::NetSession::advance()` rather than directly to `SimWorld::tick()`. The session SHALL return the authoritative multi-player input set (local + remote, confirmed or predicted), which SHALL then be passed to `SimWorld::tick(inputs)`. The rendering path SHALL read the local player's state based on the assigned player slot for camera placement. In single-player mode, the game loop SHALL continue calling `SimWorld::tick(&[input])` directly, with no networking overhead.

#### Scenario: Multiplayer tick flow
- **WHEN** the game is in multiplayer `Playing` state and a simulation tick is due
- **THEN** the local input SHALL be submitted to `NetSession::advance()`, which SHALL return the multi-player input slice, and the sim SHALL be ticked with that slice

#### Scenario: Single-player tick flow unchanged
- **WHEN** the game is in single-player `Playing` state
- **THEN** the sim SHALL be ticked directly with `&[local_input]` and no `NetSession` SHALL be involved

#### Scenario: Camera follows local player slot
- **WHEN** rendering in multiplayer mode with local player assigned to slot 2
- **THEN** the camera SHALL use player slot 2's position, facing, and vertical look for the view matrix

### Requirement: Multiplayer film recording captures all player inputs
In multiplayer mode, film recording SHALL capture all players' `TickInput` values per tick (not just the local player). The film header SHALL include `num_players` and per-player metadata (name, slot index). Film playback of multiplayer games SHALL feed all input tracks into the sim. Single-player film recording SHALL remain unchanged.

#### Scenario: Record multiplayer film
- **WHEN** film recording is enabled during a 4-player game
- **THEN** each tick's film record SHALL contain 4 `TickInput` values (one per player slot)

#### Scenario: Playback multiplayer film
- **WHEN** a multiplayer film file is loaded for playback
- **THEN** the system SHALL initialize `SimWorld` with the recorded `num_players`, and each tick SHALL feed the recorded multi-player input slice into `tick(inputs)`

#### Scenario: Film header includes player count
- **WHEN** a multiplayer film is recorded
- **THEN** the film header SHALL contain `num_players`, the game mode, and per-player metadata

### Requirement: PostGame scoreboard display
The `PostGame` state SHALL display a scoreboard showing final scores for all players. The scoreboard SHALL show player names, kills, deaths, and mode-specific scores (hill time for KOTH, ball time for KTMWTB, tag count for Tag). The player SHALL be able to return to the main menu or (if the lobby is still active) return to the lobby for another game.

#### Scenario: Deathmatch scoreboard
- **WHEN** the game ends in Every Man For Himself mode
- **THEN** the PostGame scoreboard SHALL show each player's kills and deaths, sorted by kills descending

#### Scenario: KOTH scoreboard
- **WHEN** the game ends in King of the Hill mode
- **THEN** the PostGame scoreboard SHALL show each player's hill control time, sorted by time descending

#### Scenario: Return to lobby
- **WHEN** the player selects "Play Again" on the PostGame screen and the lobby session is still active
- **THEN** the state SHALL transition from `PostGame` to `Lobby`
