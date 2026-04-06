## MODIFIED Requirements

### Requirement: Frame-paced main loop
The system SHALL run a main loop that processes winit events, advances the simulation at a fixed tick rate (30 ticks per second, matching Marathon's original rate), and renders frames. The simulation tick rate SHALL be decoupled from the display refresh rate. If the display refreshes faster than 30 Hz, the system SHALL interpolate visual state between ticks for smooth rendering. The main loop SHALL be driven by the `marathon-game` binary's winit event loop, with the shell state machine receiving frame events and dispatching to the appropriate subsystems based on the current state.

#### Scenario: 60 Hz display with 30 tick simulation
- **WHEN** the display runs at 60 Hz
- **THEN** the system SHALL render 2 frames per simulation tick, interpolating entity positions between the previous and current tick states

#### Scenario: Simulation tick timing
- **WHEN** 33.33ms have elapsed since the last simulation tick
- **THEN** the system SHALL advance marathon-sim by one tick with the current action flags

#### Scenario: Slow frame does not skip simulation
- **WHEN** a frame takes 100ms to render (3 ticks worth of time)
- **THEN** the system SHALL run 3 simulation ticks in sequence to catch up, then render one frame

#### Scenario: Shell receives frame events from binary
- **WHEN** the marathon-game binary's event loop processes a frame
- **THEN** the shell state machine SHALL receive the frame event with elapsed time and dispatch to the current state's update and render logic

### Requirement: Level loading and initialization
The system SHALL load a level by: (1) parsing the map entry from the WadFile via marathon-formats, (2) initializing marathon-sim with the map data, physics data, and game mode, (3) initializing the rendering pipeline with the map geometry, textures, and entity sprite data, (4) optionally initializing marathon-audio with the map data and sound definitions, (5) transitioning to the `Playing` state. All parsing and initialization SHALL complete before gameplay begins. Audio initialization failure SHALL be non-fatal.

#### Scenario: Load first level of campaign
- **WHEN** the player starts a new campaign game
- **THEN** the system SHALL load level 0 from the scenario's WadFile, initialize all subsystems, and transition to `Playing`

#### Scenario: Level load failure
- **WHEN** a level's map data fails to parse
- **THEN** the system SHALL display an error message and transition to `MainMenu`

#### Scenario: Audio unavailable during level load
- **WHEN** the audio subsystem is not available during level loading
- **THEN** the system SHALL skip audio initialization and proceed with visual-only gameplay
