## ADDED Requirements

### Requirement: Room creation and discovery via signaling server
The system SHALL support creating and joining named rooms through the Matchbox signaling server. A host player SHALL create a room with a unique room ID. Other players SHALL join by specifying the room ID. The signaling server SHALL track active rooms and their occupants.

#### Scenario: Host creates a room
- **WHEN** the host selects "Create Game" and specifies a room name
- **THEN** the system SHALL connect to the signaling server and create a room with the specified ID, placing the host as the first occupant

#### Scenario: Player joins an existing room
- **WHEN** a player enters a room ID and selects "Join Game"
- **THEN** the system SHALL connect to the signaling server, join the specified room, and establish peer connections with all existing occupants

#### Scenario: Room not found
- **WHEN** a player attempts to join a room ID that does not exist on the signaling server
- **THEN** the system SHALL display an error message and remain on the join screen

### Requirement: Lobby state with player management
The lobby SHALL display a list of connected players with their names, team assignments (for team modes), and ready status. The host SHALL be able to change game settings. Players SHALL be able to toggle their ready status. Players SHALL be able to leave the lobby, which removes them from the room.

#### Scenario: Player joins lobby
- **WHEN** a new player joins the room
- **THEN** all existing lobby participants SHALL see the new player added to the player list with "Not Ready" status

#### Scenario: Player toggles ready
- **WHEN** a player toggles their ready status to "Ready"
- **THEN** all lobby participants SHALL see that player's status update to "Ready"

#### Scenario: Player leaves lobby
- **WHEN** a player leaves the lobby (disconnects or selects "Leave")
- **THEN** all remaining participants SHALL see the player removed from the player list and the player slot SHALL be freed

#### Scenario: Host leaves lobby
- **WHEN** the host disconnects from the lobby
- **THEN** the system SHALL either migrate host status to the next player or dissolve the lobby, returning all players to the main menu

### Requirement: Game settings negotiation
The host SHALL configure game settings in the lobby: map selection (level index from the loaded scenario), game mode (Every Man For Himself, King of the Hill, Kill The Man With The Ball, Tag, Cooperative), kill limit, time limit (in minutes), and team assignments (for team modes). Settings changes SHALL be broadcast to all lobby participants. Non-host players SHALL see the current settings but SHALL NOT be able to modify them.

#### Scenario: Host changes map selection
- **WHEN** the host selects a different map in the lobby
- **THEN** all participants SHALL see the updated map selection

#### Scenario: Host changes game mode
- **WHEN** the host changes the game mode from "Every Man For Himself" to "King of the Hill"
- **THEN** all participants SHALL see the updated game mode

#### Scenario: Host sets kill limit
- **WHEN** the host sets the kill limit to 25
- **THEN** all participants SHALL see the kill limit as 25 and the game mode SHALL use this value as the win condition threshold

#### Scenario: Host sets time limit
- **WHEN** the host sets the time limit to 10 minutes
- **THEN** all participants SHALL see the time limit and the game SHALL end after 10 minutes of play regardless of scores

### Requirement: Synchronized game start
The game SHALL start only when all players are ready and the host initiates the start. The system SHALL execute a synchronized countdown (3, 2, 1, Go) to ensure all clients begin the simulation on the same frame. During the countdown, no player input SHALL affect the simulation. After the countdown, the GGRS session SHALL be initialized and gameplay SHALL begin.

#### Scenario: All players ready, host starts
- **WHEN** all players in the lobby are "Ready" and the host selects "Start Game"
- **THEN** a countdown SHALL begin and all clients SHALL transition from `Lobby` to `Loading` simultaneously

#### Scenario: Not all players ready
- **WHEN** the host attempts to start the game but one or more players are not "Ready"
- **THEN** the start SHALL be blocked and a message SHALL indicate which players are not ready

#### Scenario: Player disconnects during countdown
- **WHEN** a player disconnects during the start countdown
- **THEN** the countdown SHALL be cancelled, the disconnected player SHALL be removed, and the lobby SHALL return to the waiting state

### Requirement: Lobby data exchange over Matchbox data channel
The lobby SHALL communicate player state, settings, and control messages over the Matchbox WebRTC data channel. Lobby messages SHALL be serialized with a compact binary format (bincode or similar). The lobby protocol SHALL be separate from and precede the GGRS session protocol -- the GGRS session is not initialized until the game start countdown completes.

#### Scenario: Lobby message round-trip
- **WHEN** the host changes a setting
- **THEN** a `LobbyMessage::SettingsChanged` SHALL be serialized and sent to all peers, and each peer SHALL deserialize and apply the setting update

#### Scenario: Lobby protocol precedes GGRS
- **WHEN** the game start countdown completes
- **THEN** the lobby protocol SHALL cease, the GGRS P2PSession SHALL be initialized using the same Matchbox socket, and input synchronization SHALL begin
