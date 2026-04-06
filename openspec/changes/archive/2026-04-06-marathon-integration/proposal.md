## Why

The marathon-formats, marathon-viewer, marathon-sim, and marathon-audio crates provide the foundational pieces -- data parsing, rendering, game logic, and sound -- but nothing wires them together into a playable game. Without an integration layer, there is no input handling, no HUD, no menus, no level transitions, no save/load, and no film recording. This change builds the `marathon-integration` crate that orchestrates all prior crates into a functioning Marathon engine capable of running original Aleph One content from start to finish.

The original C++ codebase scatters integration concerns across shell.cpp (main loop), the Misc/ directory (~87K LOC for UI, preferences, dialogs), and RenderOther/ (~22K LOC for HUD and UI rendering). Consolidating these into a single Rust crate with clear capability boundaries gives us a maintainable architecture while preserving full compatibility with Marathon's gameplay and content.

## What Changes

This change introduces the `marathon-integration` crate, which sits on top of all other crates and provides:

- **Input system**: Translate raw keyboard, mouse, and gamepad events (via winit) into Marathon action flags consumed by marathon-sim. Support rebindable key mappings and sensitivity settings. Handle input differently per game state (menu navigation vs. gameplay vs. terminal reading).

- **HUD rendering**: Render the in-game heads-up display using wgpu, drawing health/shield bars, oxygen meter, weapon/ammo display, motion sensor (radar), and inventory panel. The HUD reads live state from marathon-sim and renders as a 2D overlay on top of the 3D scene produced by marathon-viewer.

- **Menu and UI system**: Main menu, preferences screens, level selection, and in-game pause menu. Menus handle navigation via keyboard/mouse/gamepad and manage transitions between game states (menu, playing, paused, terminal).

- **Terminal interface**: Marathon's signature story delivery mechanism -- in-game computer terminals that display scrollable text, images, and inter-level teleport commands. Terminals pause gameplay, capture input for scrolling/paging, and can trigger level teleports on exit.

- **Game shell and level transitions**: The top-level game loop orchestrating frame timing, state machine transitions (loading, playing, intermission, finished), level completion triggers, inter-level teleporters, and map switching. Uses bevy_ecs to manage the lifecycle of all subsystems.

- **Save/load**: Serialize and deserialize full game state (player state, monster state, map state, item pickups, terminal read status) to allow resuming campaigns. Maintain save slot management.

- **Film recording and playback**: Record per-tick action flags during gameplay for deterministic replay. Support playback by feeding recorded flags into marathon-sim instead of live input. This replicates Marathon's original film system.

- **Sprite rendering integration**: Bridge marathon-sim entity state (positions, facing angles, animation frames) to marathon-viewer's sprite rendering, producing billboarded sprites for monsters, items, projectiles, and other entities.

- **Game modes**: Single-player campaign progression through a scenario's level sequence. Cooperative multiplayer. Deathmatch variants (every man for himself, king of the hill, kill the man with the ball, tag). Mode-specific rules, scoring, and spawn logic.

## Capabilities

### New Capabilities

- **`input-system`**: Input device abstraction layer. Captures raw events from winit (keyboard, mouse, gamepad), applies configurable key/button bindings, and produces Marathon action flags (move forward/back, strafe, turn, fire primary/secondary, action, cycle weapons, look up/down, map, microphone). Handles dead zones, mouse sensitivity, and input state per game context (gameplay, menu, terminal). Provides a clean boundary between platform-specific input and the simulation's fixed-tick action flag interface.

- **`hud-rendering`**: Draws the in-game HUD overlay via wgpu as a 2D render pass composited on top of the 3D scene. Renders health and oxygen bars, shield meter, weapon display with ammunition counts, motion sensor (radar showing nearby entities as dots with distance/direction), and inventory items. Reads per-frame state from marathon-sim. Supports Marathon's multiple HUD styles and adapts layout to display resolution.

- **`game-shell`**: The outermost orchestration layer. Manages the main event loop, frame pacing, and a state machine governing transitions between menus, loading screens, gameplay, terminals, intermission screens, and end-of-game. Coordinates level loading (parsing map data via marathon-formats, initializing marathon-sim, setting up marathon-viewer and marathon-audio). Handles level completion detection, inter-level teleporters, and sequential level progression through a scenario. Manages save/load of game state and save slot UI. Implements film recording (capturing action flags per tick) and film playback (replaying recorded flags through marathon-sim for deterministic replay). Supports all Marathon game modes -- single-player campaign, cooperative, and multiplayer variants -- with mode-specific spawn rules, scoring, and win conditions. Uses bevy_ecs as the scheduling backbone to coordinate all subsystems within the frame loop.

- **`terminal-interface`**: Implements Marathon's in-game computer terminals. When a player activates a terminal, gameplay pauses and the terminal UI takes over, displaying styled text pages (plain text, logon/logoff sequences, images, chapter headers) parsed from marathon-formats terminal data. Handles paging, scrolling, and input capture within the terminal view. Terminals can trigger level teleports on exit, serving as both narrative delivery and level-transition mechanism. Supports conditional text groups that display different content based on game state (mission success/failure).

### Modified Capabilities

None. This is a new crate with no modifications to existing capabilities.

## Impact

- **marathon-formats**: Consumed as a dependency. The integration crate reads map data, physics definitions, terminal text, HUD shape data, and film file structures through marathon-formats' public API. No changes required to marathon-formats.

- **marathon-viewer**: Consumed as a dependency. The game shell initializes the renderer per level, feeds it camera state from marathon-sim each frame, and composites HUD rendering on top of the 3D scene. Sprite rendering integration passes entity state from marathon-sim to marathon-viewer for billboarded sprite drawing. No changes required to marathon-viewer.

- **marathon-sim**: Consumed as a dependency. The input system feeds action flags into marathon-sim's tick function. The game shell reads simulation state for HUD display, terminal activation, level completion detection, and entity positions for sprite rendering. Film recording captures the action flags passed to marathon-sim; film playback replays them. Save/load serializes marathon-sim's full game state. No changes required to marathon-sim.

- **marathon-audio**: Consumed as a dependency. The game shell initializes audio per level and routes sound-trigger events from marathon-sim to marathon-audio for playback. No changes required to marathon-audio.

- **External dependencies**: Adds winit (window/input events), wgpu (already used by marathon-viewer, now also used for HUD/UI rendering), and bevy_ecs (entity-component-system scheduling for the game loop). These are the only new external crate dependencies introduced.
