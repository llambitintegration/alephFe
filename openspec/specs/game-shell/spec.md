## ADDED Requirements

### Requirement: Game state machine with defined transitions
The system SHALL implement a state machine with states: `Loading`, `MainMenu`, `Playing`, `Paused`, `Terminal`, `Intermission`, `GameOver`. Transitions SHALL follow these rules:
- `Loading` -> `MainMenu` (after initial assets loaded)
- `MainMenu` -> `Loading` (when starting a game or loading a save)
- `Loading` -> `Playing` (after level assets loaded)
- `Playing` -> `Paused` (on pause input)
- `Paused` -> `Playing` (on resume input)
- `Paused` -> `MainMenu` (on quit to menu)
- `Playing` -> `Terminal` (on terminal activation)
- `Terminal` -> `Playing` (on terminal exit)
- `Playing` -> `Intermission` (on level completion)
- `Intermission` -> `Loading` (on next level start)
- `Playing` -> `GameOver` (on player death or campaign end)
- `GameOver` -> `MainMenu` (on acknowledgement)

#### Scenario: Start new game from main menu
- **WHEN** the player selects "New Game" from the main menu
- **THEN** the state SHALL transition from `MainMenu` to `Loading` with the first level of the selected difficulty

#### Scenario: Pause during gameplay
- **WHEN** the player presses the pause key during `Playing` state
- **THEN** the state SHALL transition to `Paused` and the simulation SHALL stop advancing

#### Scenario: Terminal activation
- **WHEN** the player activates a terminal polygon during `Playing` state
- **THEN** the state SHALL transition to `Terminal` and the simulation SHALL stop advancing

#### Scenario: Level completion
- **WHEN** marathon-sim signals level completion (via success condition or teleport to next level)
- **THEN** the state SHALL transition from `Playing` to `Intermission`

### Requirement: Frame-paced main loop
The system SHALL run a main loop that processes winit events, advances the simulation at a fixed tick rate (30 ticks per second, matching Marathon's original rate), and renders frames. The simulation tick rate SHALL be decoupled from the display refresh rate. If the display refreshes faster than 30 Hz, the system SHALL interpolate visual state between ticks for smooth rendering.

#### Scenario: 60 Hz display with 30 tick simulation
- **WHEN** the display runs at 60 Hz
- **THEN** the system SHALL render 2 frames per simulation tick, interpolating entity positions between the previous and current tick states

#### Scenario: Simulation tick timing
- **WHEN** 33.33ms have elapsed since the last simulation tick
- **THEN** the system SHALL advance marathon-sim by one tick with the current action flags

#### Scenario: Slow frame does not skip simulation
- **WHEN** a frame takes 100ms to render (3 ticks worth of time)
- **THEN** the system SHALL run 3 simulation ticks in sequence to catch up, then render one frame

### Requirement: Level loading and initialization
The system SHALL load a level by: (1) parsing the map entry from the WadFile via marathon-formats, (2) initializing marathon-sim with the map data, physics data, and game mode, (3) initializing marathon-viewer with the map geometry and textures, (4) initializing marathon-audio with the map data and sound definitions, (5) transitioning to the `Playing` state. All parsing and initialization SHALL complete before gameplay begins.

#### Scenario: Load first level of campaign
- **WHEN** the player starts a new campaign game
- **THEN** the system SHALL load level 0 from the scenario's WadFile, initialize all subsystems, and transition to `Playing`

#### Scenario: Load level from save file
- **WHEN** the player loads a saved game
- **THEN** the system SHALL load the saved level, initialize subsystems, restore the serialized simulation state, and transition to `Playing`

#### Scenario: Level load failure
- **WHEN** a level's map data fails to parse
- **THEN** the system SHALL display an error message and transition to `MainMenu`

### Requirement: Level transitions via inter-level teleporters
The system SHALL detect when marathon-sim signals that the player has entered an inter-level teleporter or a terminal has triggered a level teleport. The system SHALL determine the target level index, transition through `Intermission` (showing level completion stats), then load the target level.

#### Scenario: Teleporter to next level
- **WHEN** the player enters a polygon that marathon-sim identifies as an inter-level teleporter targeting level 5
- **THEN** the system SHALL transition to `Intermission`, display completion stats, then load level 5

#### Scenario: Terminal-triggered teleport
- **WHEN** the player exits a terminal that specifies a teleport to level 3
- **THEN** the system SHALL transition from `Terminal` to `Intermission`, then load level 3

### Requirement: Save game state to persistent storage
The system SHALL serialize the full game state (marathon-sim state, current level index, difficulty, game mode, terminal read status, film data if recording) to a save file. The system SHALL support multiple save slots. Save files SHALL be written to a platform-appropriate user data directory.

#### Scenario: Save to empty slot
- **WHEN** the player saves to slot 2 which is empty
- **THEN** the system SHALL serialize the current game state and write it to slot 2's save file

#### Scenario: Overwrite existing save
- **WHEN** the player saves to slot 1 which contains a previous save
- **THEN** the system SHALL overwrite slot 1's save file with the current game state

#### Scenario: Save includes level and difficulty
- **WHEN** a save is created on level 7 at Major Damage difficulty
- **THEN** the save file SHALL contain the level index (7), difficulty setting, and full simulation state

### Requirement: Load game state from persistent storage
The system SHALL deserialize a save file and restore the full game state. Loading SHALL initialize all subsystems for the saved level, then apply the serialized simulation state on top. The system SHALL validate the save file format before applying it.

#### Scenario: Load valid save
- **WHEN** the player selects a valid save file in slot 1
- **THEN** the system SHALL load the level, restore simulation state, and transition to `Playing` with the exact state at save time

#### Scenario: Load corrupted save
- **WHEN** the player selects a save file that fails deserialization
- **THEN** the system SHALL display an error message and remain on the save/load screen

### Requirement: Film recording captures action flags per tick
The system SHALL, when film recording is enabled, record the `ActionFlags` value for every simulation tick along with the starting random seed, level index, difficulty, and game mode. The recorded data SHALL be written to a film file on level completion or save.

#### Scenario: Record a complete level
- **WHEN** the player completes a level with film recording enabled
- **THEN** the film file SHALL contain the level index, random seed, difficulty, and the sequence of action flags for every tick from level start to completion

#### Scenario: Recording includes metadata
- **WHEN** a film recording begins on level 3 at Total Carnage difficulty
- **THEN** the film header SHALL contain level index 3, difficulty Total Carnage, the initial random seed, and game mode

### Requirement: Film playback replays recorded action flags
The system SHALL, during film playback, load the recorded film file, initialize the level with the recorded seed and settings, and feed the recorded `ActionFlags` sequence into marathon-sim tick by tick instead of live input. The 3D scene, HUD, and audio SHALL render normally from the simulated state.

#### Scenario: Play back a recorded film
- **WHEN** the player opens a film file for level 5
- **THEN** the system SHALL load level 5, set the random seed from the film, and advance the simulation using recorded action flags at 30 ticks per second

#### Scenario: Film playback ends
- **WHEN** all recorded action flags have been consumed
- **THEN** the system SHALL transition to `GameOver` or `MainMenu`

### Requirement: Single-player campaign mode
The system SHALL support single-player campaign mode where the player progresses sequentially through a scenario's levels. The campaign SHALL track the current level, difficulty setting, and completed levels. Level progression SHALL follow the scenario's level order unless overridden by inter-level teleporters.

#### Scenario: Sequential level progression
- **WHEN** the player completes level 2 in campaign mode
- **THEN** the system SHALL advance to level 3 (the next in sequence)

#### Scenario: Difficulty affects simulation
- **WHEN** the player starts a campaign on Major Damage difficulty
- **THEN** marathon-sim SHALL be initialized with Major Damage difficulty parameters for each level

### Requirement: Multiplayer game modes
The system SHALL support the following multiplayer game modes with mode-specific rules: Every Man for Himself (deathmatch), King of the Hill (timed zone control), Kill the Man with the Ball (ball possession scoring), Tag (tagged player scores points), and Cooperative (campaign with multiple players). Each mode SHALL define its own scoring rules, win conditions, and respawn behavior.

#### Scenario: King of the Hill scoring
- **WHEN** a player stands in the designated hill polygon for 5 consecutive seconds in KOTH mode
- **THEN** the player's score SHALL increase by 5 seconds worth of hill time

#### Scenario: Deathmatch kill scoring
- **WHEN** player A kills player B in Every Man for Himself mode
- **THEN** player A's kill count SHALL increase by 1

#### Scenario: Cooperative respawn
- **WHEN** a player dies in cooperative mode
- **THEN** the player SHALL respawn at a team spawn point after a respawn delay

#### Scenario: Kill the Man with the Ball possession
- **WHEN** a player picks up the ball in KTMWTB mode
- **THEN** that player SHALL begin accumulating score for each second of ball possession

### Requirement: Menu system with screen navigation
The system SHALL provide a menu system with screens for: main menu, new game (difficulty selection), load game (save slot selection), preferences (controls, audio, video settings), and in-game pause menu. Menu screens SHALL support keyboard, mouse, and gamepad navigation. Screen transitions SHALL be stack-based (pushing a submenu, popping returns to parent).

#### Scenario: Navigate main menu
- **WHEN** the player is on the main menu and presses Down then Select
- **THEN** the menu cursor SHALL move down one item and the selected item's action SHALL execute

#### Scenario: Open preferences from main menu
- **WHEN** the player selects "Preferences" from the main menu
- **THEN** the preferences screen SHALL be pushed onto the menu stack

#### Scenario: Back from preferences
- **WHEN** the player presses Back on the preferences screen
- **THEN** the preferences screen SHALL be popped and the main menu SHALL be displayed

### Requirement: Sprite rendering bridge
The system SHALL read entity state (positions, facing angles, animation frame indices, collection/sequence references) from marathon-sim each frame and pass this data to marathon-viewer for billboarded sprite rendering. The bridge SHALL handle all entity types: monsters, items, projectiles, effects, players.

#### Scenario: Monster sprite update
- **WHEN** marathon-sim reports a monster at position (1024, 2048, 0) facing 180 degrees in animation frame 3 of sequence 5 in collection 12
- **THEN** the system SHALL pass this state to marathon-viewer, which SHALL render the appropriate sprite frame billboarded at that world position

#### Scenario: Item pickup disappears
- **WHEN** marathon-sim removes an item entity (picked up by player)
- **THEN** the system SHALL stop passing that entity to marathon-viewer, removing it from the rendered scene

#### Scenario: Projectile in flight
- **WHEN** marathon-sim reports a projectile entity at a position with a velocity vector
- **THEN** the system SHALL pass the projectile's position and sprite data to marathon-viewer each frame for rendering
