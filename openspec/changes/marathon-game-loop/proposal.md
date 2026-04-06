## Why

Every subsystem of the Rust Marathon engine exists as a library — formats parsing, simulation, audio, HUD, menus, input, and level rendering — but there is no main binary that wires them together into a playable game. The only runnable binary (`marathon-viewer`) is a free-fly camera level viewer. To test playing the game, we need a unified entry point that drives the simulation from player input and renders the world from a first-person perspective.

## What Changes

- Create a new `marathon-game` binary crate that serves as the game's entry point
- Implement the main loop: load scenario data, initialize subsystems (sim, audio, rendering, input), run the shell state machine
- Add first-person camera rendering driven by the player's simulated position/facing (replacing the viewer's free-fly camera)
- Add entity/sprite rendering so monsters, items, projectiles, and scenery objects appear in the 3D world
- Bridge simulation tick output to audio (spatial sound triggers) and rendering (entity positions, lighting changes)
- Wire input capture through the input system to produce Marathon action flags that drive the simulation

## Capabilities

### New Capabilities
- `game-binary`: Main binary crate — CLI argument parsing (scenario file paths), subsystem initialization, window creation, and orchestration of the frame loop driving shell state transitions
- `entity-rendering`: Rendering Marathon sprites (monsters, items, projectiles, scenery) as billboarded quads in the 3D scene, sourced from Shapes data, with animation frame selection based on entity state and viewing angle
- `sim-render-bridge`: Connecting simulation state to rendering and audio each frame — extracting player camera from sim, interpolating entity positions between ticks, dispatching sound events from sim to audio engine

### Modified Capabilities
- `level-rendering`: Camera system changes from free-fly to first-person driven by player position/facing from the simulation; render pipeline accepts entity draw calls alongside level geometry
- `game-shell`: State machine must be instantiated and driven by the binary's event loop rather than existing only as library structures

## Impact

- **New crate**: `marathon-game` added to workspace `Cargo.toml` members
- **Dependencies**: `marathon-game` depends on all other crates (formats, sim, audio, integration)
- **marathon-viewer**: Unmodified — remains a standalone level viewer
- **marathon-integration**: Shell, input, and HUD modules get invoked for the first time from a real binary
- **marathon-viewer render pipeline**: Level rendering code may be refactored into a shared rendering library or duplicated/evolved in the game binary
- **Test data**: Requires Marathon 2 scenario files (WAD + Shapes) at runtime, same as viewer
