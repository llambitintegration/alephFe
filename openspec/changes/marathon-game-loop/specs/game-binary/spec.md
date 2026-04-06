## ADDED Requirements

### Requirement: Binary entry point with scenario file arguments
The system SHALL provide a `marathon-game` binary that accepts command-line arguments for the map WAD file path, shapes WAD file path, and optionally a sounds WAD file path. The binary SHALL validate that the specified files exist and are parseable by marathon-formats before proceeding. The binary SHALL accept an optional `--level` argument to specify the starting level index (default: 0).

#### Scenario: Launch with required arguments
- **WHEN** the user runs `marathon-game --map Map.sceA --shapes Shapes.shpA`
- **THEN** the binary SHALL parse both files and begin loading level 0

#### Scenario: Launch with level override
- **WHEN** the user runs `marathon-game --map Map.sceA --shapes Shapes.shpA --level 5`
- **THEN** the binary SHALL begin loading level 5

#### Scenario: Missing scenario file
- **WHEN** the user runs `marathon-game --map nonexistent.sceA --shapes Shapes.shpA`
- **THEN** the binary SHALL exit with an error message indicating the file was not found

#### Scenario: Unparseable scenario file
- **WHEN** the map file exists but fails WAD parsing
- **THEN** the binary SHALL exit with an error describing the parse failure

### Requirement: Subsystem initialization sequence
The system SHALL initialize subsystems in this order: (1) parse scenario files via marathon-formats, (2) create the wgpu window and GPU device, (3) initialize the rendering pipeline with level geometry and textures, (4) initialize marathon-sim with map data, physics data, and game mode, (5) optionally initialize marathon-audio with sound definitions, (6) initialize the input system with default bindings, (7) instantiate the shell state machine and transition to Playing.

#### Scenario: Successful initialization
- **WHEN** all subsystems initialize without error
- **THEN** the binary SHALL display a window rendering the starting level from the player's spawn position

#### Scenario: Audio initialization failure is non-fatal
- **WHEN** the audio subsystem fails to initialize (no audio device available)
- **THEN** the binary SHALL log a warning and continue without audio, with all other subsystems functional

#### Scenario: GPU initialization failure is fatal
- **WHEN** wgpu fails to acquire a GPU device
- **THEN** the binary SHALL exit with an error message describing the GPU failure

### Requirement: Main event loop orchestration
The system SHALL run a winit event loop that each frame: (1) collects raw input events, (2) translates input to the active context's actions (action flags for gameplay, navigation events for menus), (3) advances the simulation by zero or more ticks based on elapsed time, (4) updates audio state (spatial listener position, one-shot sounds), (5) renders the frame with interpolated state, (6) presents the frame. The loop SHALL use `ControlFlow::Poll` for continuous rendering.

#### Scenario: Steady-state gameplay frame
- **WHEN** the game is in Playing state and a frame is due
- **THEN** the system SHALL process input, tick the simulation if enough time has elapsed, and render an interpolated frame

#### Scenario: Paused state skips simulation
- **WHEN** the game is in Paused state
- **THEN** the system SHALL still render frames but SHALL NOT advance the simulation

### Requirement: Workspace integration
The `marathon-game` crate SHALL be added as a workspace member in the root `Cargo.toml`. It SHALL depend on `marathon-formats`, `marathon-sim`, `marathon-audio`, and `marathon-integration`. It SHALL use `wgpu`, `winit`, `clap`, `env_logger`, and `glam` as direct dependencies.

#### Scenario: Cargo build succeeds
- **WHEN** `cargo build --bin marathon-game` is run
- **THEN** the binary SHALL compile successfully with all dependencies resolved
