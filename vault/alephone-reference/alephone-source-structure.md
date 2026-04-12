---
tags: [alephone, reference, source-code, cpp]
---

# Aleph One Source Structure

Aleph One is the open-source continuation of Bungie's Marathon 2 engine. The C++ codebase is hosted at [github.com/Aleph-One-Marathon/alephone](https://github.com/Aleph-One-Marathon/alephone) under GPL-3.0.

## Repository Layout

```
alephone/
  Source_Files/           -- All C++ source code
    CSeries/              -- Cross-platform support layer
    Expat/                -- XML parsing (expat library)
    FFmpeg/               -- Video/audio decoding
    Files/                -- File I/O, WAD parsing, resource forks
    GameWorld/            -- Core simulation (monsters, physics, map)
    Input/                -- Mouse, keyboard, joystick handling
    LibNAT/               -- NAT traversal for networking
    Lua/                  -- Lua scripting engine integration
    Misc/                 -- Miscellaneous: prefs, vbl, computer_interface
    ModelView/            -- 3D model rendering (OBJ import)
    Network/              -- Networking, multiplayer
    RenderMain/           -- Primary software/hardware renderer
    RenderOther/          -- HUD, overhead map, fader effects
    Sound/                -- Audio engine
    TCPMess/              -- TCP networking layer
    XML/                  -- MML (Marathon Markup Language) parser
    shell.cpp             -- Main entry point, event loop
    shell_misc.cpp        -- Shell utilities
  data/
    Scenarios/            -- Game data (git submodules)
      Marathon/
      Marathon 2/
      Marathon Infinity/
  PBProjects/             -- macOS Xcode project
  VisualStudio/           -- Windows Visual Studio solution
  Resources/              -- Application resources
```

## Key Source Directories

### CSeries/ -- Cross-Platform Abstraction

The "CSeries" (compatibility series) provides platform abstraction:
- `cseries.h` -- Fundamental types, byte-swapping macros, fixed-point math
- `csalerts_sdl.cpp` -- Alert dialogs via SDL
- `csstrings.h` -- String utilities
- `FilmProfile.cpp/h` -- Film playback compatibility between engine versions

This layer handles big-endian/little-endian conversion, fixed-point arithmetic, and platform-specific I/O.

### Files/ -- WAD and Resource Loading

- `wad.cpp/h` -- WAD container format reading/writing
- `FileHandler.cpp/h` -- Cross-platform file I/O
- `game_wad.cpp/h` -- Game-specific WAD loading (map, physics, etc.)
- `Packing.cpp/h` -- Network data packing/unpacking
- `import_definitions.cpp/h` -- Importing shapes, sounds, physics from files
- `resource_manager.cpp/h` -- Resource fork handling (Mac legacy)

### GameWorld/ -- Core Simulation

This is the heart of the engine:

- **`map.cpp/h`** -- Map data structures, polygon operations
- **`monsters.cpp/h`** -- Monster AI, state machine, movement, combat
- **`physics.cpp/h`** -- Player physics model (acceleration, collision, gravity)
- **`platforms.cpp/h`** -- Platform (elevator/door) state machines
- **`projectiles.cpp/h`** -- Projectile physics, collision, detonation
- **`weapons.cpp/h`** -- Weapon state machine, firing, ammo management
- **`player.cpp/h`** -- Player state, inventory, vitals (health, shield, oxygen)
- **`effects.cpp/h`** -- Visual effects (explosions, debris, blood)
- **`items.cpp/h`** -- Item pickup, placement
- **`media.cpp/h`** -- Liquid (water, lava, goo) mechanics
- **`lightsource.cpp/h`** -- Light animation functions
- **`dynamic_limits.cpp/h`** -- Runtime entity count limits
- **`world.cpp/h`** -- World update tick, game clock
- **`flood_map.cpp/h`** -- Pathfinding flood fill for monsters
- **`marathon2.cpp/h`** -- Top-level game loop, main tick dispatch

**`physics_models.h`** is particularly important -- it defines the default physics constants for walking and running physics in the format of C initializer lists. These are the canonical values that the Rust rebuild targets.

### RenderMain/ -- Rendering Engine

- **`RenderRasterize.cpp/h`** -- Software rasterizer (original Marathon rendering)
- **`RenderVisTree.cpp/h`** -- Visibility tree computation (portal-based rendering)
- **`RenderPlaceObjs.cpp/h`** -- Object (sprite) placement in the render frame
- **`OGL_Render.cpp/h`** -- OpenGL hardware rendering path
- **`OGL_Textures.cpp/h`** -- OpenGL texture management
- **`OGL_Shader.cpp/h`** -- OpenGL shader programs
- **`SW_Texture_Extras.cpp/h`** -- Software texture effects
- **`Crosshairs.cpp/h`** -- Crosshair rendering
- **`AnimatedTextures.cpp/h`** -- Texture animation (transfer modes)
- **`collection_definition.h`** -- Shape collection data structures

The original Marathon used a software renderer with a portal-based visibility determination system (RenderVisTree). Aleph One added OpenGL hardware rendering as an alternative path.

### RenderOther/ -- Secondary Rendering

- **`HUD_rendering.cpp/h`** -- Status bar (health, shield, oxygen, ammo)
- **`OverheadMap_OGL.cpp/h`** -- Automap rendering
- **`OverheadMap_QD.cpp/h`** -- QuickDraw automap (deprecated)
- **`screen.cpp/h`** -- Screen management, resolution switching
- **`screen_drawing.cpp/h`** -- 2D drawing primitives
- **`fader.cpp/h`** -- Screen fade effects (damage, teleport, death)
- **`ViewControl.cpp/h`** -- Camera control

### Sound/ -- Audio Engine

- **`SoundManager.cpp/h`** -- Main audio manager
- **`OpenALManager.cpp/h`** -- OpenAL audio backend
- **`SoundPlayer.cpp/h`** -- Sound playback
- **`Music.cpp/h`** -- Background music (MIDI/OGG)
- **`ReplacementSounds.cpp/h`** -- Support for replacement audio files

### Misc/ -- Game Shell and Integration

- **`computer_interface.cpp/h`** -- Terminal rendering and interaction
- **`interface.cpp/h`** -- Menus, game shell state machine
- **`preferences.cpp/h`** -- User preferences
- **`vbl.cpp/h`** -- Vertical blank timing, input sampling
- **`Console.cpp/h`** -- In-game console
- **`game_errors.cpp/h`** -- Error handling

### Input/ -- Input Handling

- **`mouse_sdl.cpp/h`** -- SDL mouse input
- **`keyboard_sdl.cpp/h`** -- SDL keyboard input
- **`joystick_sdl.cpp/h`** -- Joystick/gamepad

### Network/ -- Multiplayer

- **`network.cpp/h`** -- Network game setup
- **`network_games.cpp/h`** -- Game mode rules (KOTH, KTMWTB, etc.)
- **`network_star.cpp/h`** -- Star topology networking
- **`network_ring.cpp/h`** -- Ring topology networking (original Marathon)

## Key Data Structures (from the C++ source)

### Fixed-Point Math

```c
typedef int32 fixed;  // 16.16 fixed-point
#define FIXED_ONE (1 << 16)
#define FIXED_TO_FLOAT(x) ((x) / 65536.0f)
```

### World Coordinates

```c
typedef int16 world_distance;  // 1024 = 1 World Unit
#define WORLD_ONE 1024
#define WORLD_TO_FIXED(x) ((x) << 6)  // world_distance to fixed
```

### Angle System

```c
typedef int16 angle;  // 512 = full circle
#define FULL_CIRCLE 512
#define QUARTER_CIRCLE 128
#define EIGHTH_CIRCLE 64
#define ANGULAR_BITS 9  // 512 = 2^9
```

### The Main Tick Function

In `marathon2.cpp`, the main game tick:

```
update_world()
  1. update_players() -- Process input, movement, vitals
  2. move_monsters() -- AI, pathfinding, movement
  3. move_projectiles() -- Physics, collision
  4. update_effects() -- Animation timers
  5. update_lights() -- Light function evaluation
  6. update_platforms() -- Platform state machines
  7. update_control_panels() -- Switch/panel state
  8. update_media() -- Liquid height changes
```

This ordering is the canonical system execution order that the Rust rebuild's tick pipeline must match for behavioral fidelity.

## Build System

Aleph One supports multiple platforms:
- **Linux/FreeBSD**: autoconf/automake (`./configure && make`)
- **macOS**: Xcode project in `PBProjects/`
- **Windows**: Visual Studio solution in `VisualStudio/`

Dependencies: SDL2, OpenGL, OpenAL, Boost, zlib, libpng, Lua 5.x, FFmpeg (optional).

## Mapping to Rust Crates

| Aleph One Directory | Rust Crate |
|--------------------|------------|
| Files/ (WAD, shapes, sounds) | marathon-formats |
| GameWorld/ | marathon-sim |
| Sound/ | marathon-audio |
| Misc/ (interface, terminals, prefs) | marathon-integration |
| RenderMain/ + RenderOther/ | marathon-game, marathon-web |
| Input/ | marathon-integration (input module) |
| Network/ | (not yet implemented) |
| CSeries/ | Not needed (Rust's type system handles this) |
