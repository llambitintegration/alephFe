## 1. Crate Scaffolding

- [x] 1.1 Create `marathon-integration` crate directory with `Cargo.toml` (deps: marathon-formats, marathon-audio, bevy_ecs, winit, wgpu, glyphon, serde, bincode, rand)
- [x] 1.2 Add `marathon-integration` to workspace members in root `Cargo.toml`
- [x] 1.3 Create module structure: `src/lib.rs` with submodule declarations for `input/`, `hud/`, `menu/`, `terminal/`, `shell/`, `modes/`, `sprites/`
- [x] 1.4 Define shared types: `GameState` enum (Loading, MainMenu, Playing, Paused, Terminal, Intermission, GameOver), `ActionFlags` bitfield, `GameConfig` struct

## 2. Input System

- [x] 2.1 Implement `RawInput` enum (KeyPress, KeyRelease, MouseDelta, MouseButton, GamepadAxis, GamepadButton) and per-frame input buffer
- [x] 2.2 Implement `InputContext` enum (Gameplay, Menu, Terminal) and context selection based on `GameState`
- [x] 2.3 Implement `KeyBindings` struct with configurable mappings per context and default Marathon-standard bindings
- [x] 2.4 Implement gameplay input translation: convert active bindings + raw input into `ActionFlags` with mouse sensitivity and gamepad dead zone support
- [x] 2.5 Implement menu input translation: convert raw input into `MenuAction` events (Up, Down, Left, Right, Select, Back)
- [x] 2.6 Implement terminal input translation: convert raw input into `TerminalAction` events (ScrollUp, ScrollDown, PageUp, PageDown, Exit)
- [x] 2.7 Write unit tests for key binding resolution, action flag generation, dead zone filtering, and context switching

## 3. Game Shell and State Machine

- [x] 3.1 Define `GameState` as a bevy_ecs `States` type with all transitions from the spec
- [x] 3.2 Implement the main loop: winit event handling, fixed-tick simulation advancement at 30 Hz, and frame rendering with interpolation
- [x] 3.3 Implement level loading: parse WadFile entry via marathon-formats, initialize marathon-sim, marathon-viewer, and marathon-audio
- [x] 3.4 Implement level transition detection: inter-level teleporter signals and terminal teleport triggers
- [ ] 3.5 Implement intermission screen: display level completion stats, advance to next level on input
- [x] 3.6 Write tests for state machine transitions and fixed-tick timing logic

## 4. Save/Load System

- [x] 4.1 Define `SaveData` struct with serde Serialize/Deserialize: simulation state, level index, difficulty, game mode, terminal read status
- [x] 4.2 Implement save slot management: enumerate slots, write save file (bincode), read save file with validation
- [x] 4.3 Implement save game: serialize current state to selected slot
- [x] 4.4 Implement load game: deserialize save file, reinitialize level, restore simulation state
- [x] 4.5 Write tests for save/load round-trip serialization

## 5. Film Recording and Playback

- [x] 5.1 Define `FilmHeader` struct (level index, difficulty, random seed, game mode) and `FilmData` struct (header + Vec of per-tick ActionFlags)
- [x] 5.2 Implement film recording: capture ActionFlags each tick, write to file on level completion or save
- [x] 5.3 Implement film playback: load film file, initialize level with recorded seed, feed recorded ActionFlags to marathon-sim per tick
- [x] 5.4 Write tests for film recording/playback round-trip determinism

## 6. HUD Rendering

- [ ] 6.1 Create HUD wgpu render pipeline: 2D sprite/quad rendering pass that composites on top of the 3D framebuffer
- [x] 6.2 Implement health and shield bar rendering with tier-based coloring
- [x] 6.3 Implement oxygen meter rendering with visibility toggle and low-oxygen warning
- [x] 6.4 Implement weapon and ammunition display with sprite frames from ShapesFile interface collection
- [x] 6.5 Implement motion sensor (radar): circular display with entity dots color-coded by type, positioned by relative angle/distance, rotating with player facing
- [x] 6.6 Implement inventory panel rendering with item icons and counts
- [x] 6.7 Implement HUD resolution scaling: proportional layout that adapts to display resolution
- [x] 6.8 Wire HUD to marathon-sim state: read player health, shield, oxygen, weapon, ammo, inventory, position each frame
- [x] 6.9 Write tests for HUD layout calculations and motion sensor positioning math

## 7. Menu System

- [x] 7.1 Implement menu screen stack: push/pop navigation with screen types (MainMenu, NewGame, LoadGame, Preferences, PauseMenu)
- [x] 7.2 Implement main menu screen: New Game, Load Game, Preferences, Quit options
- [x] 7.3 Implement new game screen: difficulty selection (Kindergarten through Total Carnage)
- [ ] 7.4 Implement load game screen: display save slots with level/date info, load selected slot
- [ ] 7.5 Implement preferences screen: controls (key rebinding), audio (volume sliders), video (resolution) settings
- [x] 7.6 Implement pause menu: Resume, Save Game, Preferences, Quit to Menu options
- [ ] 7.7 Implement menu rendering via wgpu: text rendering with glyphon, cursor highlighting, screen transitions
- [x] 7.8 Wire menu actions to game state transitions (start game, load game, resume, quit)

## 8. Terminal Interface

- [x] 8.1 Implement terminal activation detection: read terminal data from marathon-formats for activated terminal polygon
- [x] 8.2 Implement terminal text group evaluation: conditional groups based on mission state (success/failure)
- [x] 8.3 Implement terminal page layout: split text groups into pages that fit the display area
- [ ] 8.4 Implement terminal text rendering via glyphon: styled text (information, logon, logoff, chapter headers) with Marathon terminal colors
- [ ] 8.5 Implement terminal image rendering: load PICT resources, render inline within terminal view
- [x] 8.6 Implement terminal navigation: scroll within page, page forward/backward, page indicator display
- [x] 8.7 Implement terminal exit: return to Playing state, handle teleport-on-exit if specified
- [x] 8.8 Implement terminal read status tracking: mark terminals as read, include in save data
- [x] 8.9 Write tests for page layout, conditional group evaluation, and teleport-on-exit logic

## 9. Sprite Rendering Bridge

- [x] 9.1 Implement entity state reader: extract positions, facing angles, animation frames, collection/sequence refs from marathon-sim each frame
- [x] 9.2 Implement sprite data bridge: convert marathon-sim entity state into marathon-viewer sprite render commands (billboarded quads with correct sprite frames)
- [x] 9.3 Handle entity lifecycle: add sprites for new entities, update existing, remove sprites for despawned entities (pickups, killed monsters, expired projectiles)
- [x] 9.4 Write tests for entity-to-sprite state mapping

## 10. Game Modes

- [x] 10.1 Define `GameMode` trait with methods: scoring rules, win conditions, spawn behavior, respawn rules
- [x] 10.2 Implement single-player campaign mode: sequential level progression, difficulty setting, campaign completion tracking
- [x] 10.3 Implement cooperative mode: multi-player campaign with team spawn points and shared progression
- [x] 10.4 Implement Every Man for Himself (deathmatch): kill-based scoring, free-for-all respawn
- [x] 10.5 Implement King of the Hill: timed zone control scoring, hill polygon tracking
- [x] 10.6 Implement Kill the Man with the Ball: ball possession tracking, time-held scoring
- [x] 10.7 Implement Tag: tag status tracking, tagged-player scoring
- [x] 10.8 Write tests for scoring logic and win conditions for each game mode

## 11. Integration Testing

- [ ] 11.1 Create integration test that loads a real Marathon level via marathon-formats, initializes the game shell, and verifies state transitions
- [ ] 11.2 Create integration test for full save/load/film round-trip with a simulated play session
- [ ] 11.3 Verify HUD rendering pipeline produces valid wgpu output with test fixture data
