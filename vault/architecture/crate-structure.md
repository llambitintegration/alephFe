---
tags: [architecture, crates, dependencies]
---

# Crate Structure & Dependencies

The project is a Cargo workspace with 7 crates. The workspace root `Cargo.toml` lists them all with `resolver = "2"`.

## Crate Overview

| Crate | Type | Purpose |
|-------|------|---------|
| `marathon-formats` | Library | Parser for all Marathon/Aleph One binary content formats |
| `marathon-sim` | Library | Deterministic game simulation (ECS-based) |
| `marathon-audio` | Library | Spatial audio engine using kira |
| `marathon-integration` | Library | Game shell: input, HUD, menus, terminals, game modes |
| `marathon-game` | Binary | Desktop game client (wgpu + winit) |
| `marathon-web` | Library (cdylib) | WebAssembly game client (wgpu + web-sys) |
| `marathon-viewer` | Binary | Standalone 3D level viewer |

## Dependency Graph

```
                          marathon-formats
                         /       |        \
                        /        |         \
               marathon-sim  marathon-audio  marathon-viewer
                  |    \         |
                  |     \        |
                  |  marathon-integration
                  |       /
                  |      /
             marathon-game
                  
               marathon-sim
                  |
                  |
             marathon-web
```

### ASCII Dependency Flow (detailed)

```
marathon-formats  (standalone: binrw, thiserror, bitflags, quick-xml)
       |
       +---> marathon-sim     (+ bevy_ecs, glam, serde, bincode, rand, thiserror)
       |          |
       +---> marathon-audio   (+ kira, thiserror, rand)
       |          |
       +---> marathon-integration  (+ bevy_ecs, winit, wgpu, glyphon, serde, bincode,
       |          |                    bitflags, rand, thiserror)
       |          |
       +---> marathon-game    (+ marathon-sim, marathon-audio, marathon-integration,
       |                         wgpu, winit, clap, glam, bytemuck, log, env_logger,
       |                         pollster)
       |
       +---> marathon-web     (+ marathon-sim, wgpu[webgpu,webgl], glam, bytemuck,
       |                         wasm-bindgen, web-sys, js-sys, console_log,
       |                         console_error_panic_hook, getrandom[wasm_js])
       |
       +---> marathon-viewer  (+ wgpu v22, winit, pollster, glam, clap, bytemuck,
                                 log, env_logger)
```

## Detailed Crate Descriptions

### marathon-formats

**Path:** `/marathon-formats/`
**Version:** 0.1.0
**License:** MIT OR Apache-2.0

Parses all Marathon/Aleph One binary content formats. Zero dependencies on game logic or rendering. Designed as a standalone library suitable for tooling.

**Key dependencies:**
- `binrw 0.14` -- Declarative binary format parsing (big-endian)
- `thiserror 2` -- Error types
- `bitflags 2` -- Flag parsing (sound flags, line flags, etc.)
- `quick-xml 0.37` -- MML (Marathon Markup Language) XML parsing

**Modules:**
- `wad` -- WAD container format (header, directory, entries, tag chunks)
- `map` -- Map geometry: endpoints, lines, sides, polygons, objects, lights, platforms, media, annotations, terminals, ambient/random sounds
- `shapes` -- Shapes file: collection headers, bitmaps, color tables, high/low level shapes
- `sounds` -- Sound file: sound definitions, permutations, audio data extraction
- `physics` -- Physics model: PhysicsConstants, MonsterDefinition, ProjectileDefinition, WeaponDefinition, EffectDefinition
- `mml` -- Marathon Markup Language XML parser
- `plugin` -- Plugin metadata and scenario requirement parsing
- `tags` -- WAD tag four-character-code enum (60+ known tags)
- `types` -- Shared types: WorldPoint2d/3d, ShapeDescriptor, SideTexture, DamageDefinition
- `error` -- Error types for all parsing operations
- `test_helpers` -- (feature-gated) Synthetic test data construction

**Feature flags:**
- `test-helpers` -- Enables the `test_helpers` module for constructing synthetic binary data in tests

### marathon-sim

**Path:** `/marathon-sim/`
**Version:** 0.1.0

Deterministic game simulation. Uses `bevy_ecs` for the entity-component system but does NOT use the Bevy scheduler -- instead it drives the ECS world manually with direct queries. See [[ecs-architecture]] for details.

**Key dependencies:**
- `marathon-formats` -- For MapData, PhysicsData types
- `bevy_ecs 0.15` -- Entity-component-system (World, Component, Resource, Query)
- `glam 0.29` (with serde) -- Math library (Vec2, Vec3, Mat4)
- `serde 1` + `bincode 1` -- Snapshot serialization for save/load
- `rand 0.8` -- Deterministic PRNG (StdRng)
- `thiserror 2` -- Error types

**Modules:**
- `components` -- All ECS components (Position, Velocity, Facing, Health, Shield, Monster, Projectile, Platform, Light, Media, etc.)
- `tick` -- Per-tick input (ActionFlags, TickInput), the `SimWorld::tick()` entry point
- `world` -- SimWorld construction, MapGeometry resource, entity spawning, snapshot serialization
- `player/movement` -- Player physics: acceleration, deceleration, gravity, collision, media effects
- `player/inventory` -- Weapon inventory, weapon slot state machine
- `combat/damage` -- Damage calculation with immunities, weaknesses, AOE, shield-then-health
- `combat/weapons` -- Weapon firing state machine, burst fire, dual-wield
- `combat/projectiles` -- Projectile advancement, gravity, homing, wall/entity collision
- `collision` -- Point-in-polygon, segment intersection, wall sliding, line-of-sight
- `monster/ai` -- AI state machine, vision checks, cascade alerting, attack resolution
- `monster/pathfinding` -- BFS polygon-graph pathfinding
- `world_mechanics/platforms` -- Platform state machine (rest/extend/delay/return), activation, crushing
- `world_mechanics/lights` -- Light intensity computation (constant, linear, smooth, flicker)
- `world_mechanics/media` -- Media height, drag factors, damage types
- `world_mechanics/items` -- Item pickup logic
- `world_mechanics/panels` -- Panel interaction (switches, etc.)

### marathon-audio

**Path:** `/marathon-audio/`
**Version:** 0.1.0

Spatial audio engine. Owns all sound playback, ambient loops, random environmental sounds, and music.

**Key dependencies:**
- `marathon-formats` -- Sound definitions, map data for obstruction
- `kira 0.12` -- Audio playback engine (static sound data, panning, volume, playback rate)
- `rand 0.8` -- Random permutation selection, pitch/volume randomization

**Modules:**
- `engine` -- Main AudioEngine: sound cache, event processing, spatial parameter updates
- `spatial` -- Distance attenuation, directional panning, obstruction, pitch/volume randomization
- `channel` -- Channel pool management (allocate, release, cleanup finished)
- `ambient` -- Ambient loop manager, random sound trigger manager
- `music` -- Background music player
- `types` -- AudioConfig, AudioEvent, PlaySoundRequest, ListenerState, etc.

### marathon-integration

**Path:** `/marathon-integration/`
**Version:** 0.1.0

Game integration layer. Bridges simulation, audio, and rendering with platform-level concerns: input mapping, HUD rendering, menu systems, terminal display, game state machine, film recording, and save/load.

**Key dependencies:**
- `marathon-formats`, `marathon-audio`, `bevy_ecs 0.15`, `winit 0.30`, `wgpu 24`, `glyphon 0.7` (text rendering)
- `serde`, `bincode`, `bitflags`, `rand`, `thiserror`

**Modules:**
- `shell/states` -- Game state machine (Loading, MainMenu, Playing, Paused, Terminal, Intermission, GameOver), tick accumulator, valid transitions
- `shell/level` -- Level loading orchestration
- `shell/save` -- Save/load game
- `shell/film` -- Film recording/playback
- `shell/intermission` -- Level-complete intermission screen
- `input/` -- Input mapping, key bindings, action flag generation, context-sensitive input modes
- `hud/` -- HUD rendering: health, shield, oxygen bars, weapon display, motion sensor, inventory
- `menu/` -- Main menu, load game, preferences screens
- `terminal/` -- Terminal text rendering: page layout, image display
- `modes/` -- Game mode implementations: campaign, cooperative, deathmatch
- `sprites/` -- Sprite resolution helpers
- `types` -- GameState, ActionFlags, Difficulty, GameModeType, GameConfig

### marathon-game

**Path:** `/marathon-game/`
**Version:** 0.1.0

Desktop game binary. The primary way to play. Uses `winit` for windowing/input and `wgpu` for rendering.

**Key dependencies:**
- All four library crates (formats, sim, audio, integration)
- `wgpu 24`, `winit 0.30`, `clap >=4.0 <4.6`, `glam`, `bytemuck`, `pollster`

**Modules:**
- `main` -- CLI argument parsing (--map, --shapes, --sounds, --level), entry point
- `render` -- wgpu setup, ApplicationHandler, game loop, tick accumulation, camera interpolation, render passes
- `mesh` -- CPU-side mesh generation from MapData (floors, ceilings, walls, media surfaces)
- `texture` -- Texture loading from Shapes files, GPU texture array creation
- `level` -- Level enumeration, loading, texture descriptor collection, light evaluation
- `sprites` -- Entity sprite billboard rendering
- `shader.wgsl` -- Level geometry shader (vertex + fragment with transfer modes)
- `sprite_shader.wgsl` -- Billboard sprite shader

### marathon-web

**Path:** `/marathon-web/`
**Version:** 0.1.0
**Crate type:** cdylib + rlib (WASM target)

WebAssembly game client. Compiled with `wasm-pack` for browser deployment. Receives pre-fetched binary data from JavaScript.

**Key dependencies:**
- `marathon-formats`, `marathon-sim`
- `wgpu 24` (features: webgpu, webgl)
- `wasm-bindgen`, `web-sys`, `js-sys`
- `console_log`, `console_error_panic_hook`

**Differences from marathon-game:**
- No marathon-audio or marathon-integration dependencies (HUD is done via DOM manipulation)
- No winit -- input handled via web-sys keyboard/mouse events
- Timing via `js_sys::Date::now()` instead of `std::time::Instant`
- Mesh vertices bake light and transfer_mode per-vertex (no storage buffer)
- Draw calls batched by collection index (no storage buffer for polygon data)
- HUD rendered with HTML/CSS DOM elements, not wgpu
- Automap rendered on a separate HTML canvas using 2D context

### marathon-viewer

**Path:** `/marathon-viewer/`
**Version:** 0.1.0

Standalone level viewer for debugging and development. Displays the 3D geometry of a level with textures but without simulation.

**Key dependencies:**
- `marathon-formats`, `wgpu 22` (older version), `winit`, `pollster`, `glam`, `clap`, `bytemuck`

**Modules:**
- `main` -- CLI parsing (--map, --shapes)
- `render` -- Free-fly camera, wgpu rendering
- `mesh` -- Mesh generation (shared approach with marathon-game)
- `texture` -- Texture loading
- `level` -- Level loading helpers
- `transfer` -- Transfer mode handling

## Version Compatibility Notes

- **wgpu:** marathon-game and marathon-web use v24, marathon-viewer uses v22
- **bevy_ecs:** v0.15 used in marathon-sim and marathon-integration (standalone, not full Bevy)
- **glam:** v0.29 across all crates that use it
- **Rust edition:** 2021 for all crates
