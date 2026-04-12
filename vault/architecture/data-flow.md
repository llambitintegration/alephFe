---
tags: [architecture, data-flow, parsing, integration]
---

# Data Flow

This document traces how data flows from Marathon's binary files through parsing, simulation, and rendering.

## High-Level Data Flow

```
  Scenario Files (on disk or fetched via HTTP)
  ============================================
  Map.sceA   Shapes   Sounds   Physics (optional, embedded in map)
     |          |        |          |
     v          v        v          v
  marathon-formats (parsing layer)
  ================================
  WadFile -> MapData    ShapesFile   SoundsFile   PhysicsData
                |            |           |             |
                +-----+------+-----------+-------------+
                      |
            +---------+---------+
            |                   |
            v                   v
     marathon-sim          marathon-audio
     (simulation)          (audio engine)
            |                   |
            v                   v
     SimWorld state        AudioEngine state
     (ECS entities,        (channel pool,
      resources)            ambient, music)
            |                   |
            +--------+----------+
                     |
                     v
            marathon-integration
            (game shell, glue)
                     |
         +-----------+-----------+
         |                       |
         v                       v
   marathon-game           marathon-web
   (desktop renderer)      (WASM renderer)
```

## Phase 1: File Loading

### Desktop (marathon-game)

```rust
fn main() {
    let args = Args::parse();  // --map, --shapes, --sounds, --level
    render::run(args.map, args.shapes, args.sounds, args.level);
}
```

Files are loaded from local paths:
1. Map WAD: `WadFile::from_file(path)` -> parses 128-byte header, directory, entries
2. Shapes: `ShapesFile::from_file(path)` -> parses 32 collection headers, lazy collection loading
3. Sounds: `SoundsFile::from_file(path)` (optional) -> parses sound definitions
4. Physics: extracted from the map WAD entry's physics tags

### Web (marathon-web)

```rust
#[wasm_bindgen]
pub async fn start_game(map_data: &[u8], shapes_data: &[u8], physics_data: &[u8]) {
    // JavaScript fetches the files and passes raw bytes
}
```

Files are fetched via JavaScript `fetch()`, then passed as `&[u8]` slices to the WASM entry point. No file system access.

## Phase 2: Parsing (marathon-formats)

### WAD File Structure

```
+------------------+
| WAD Header       |  128 bytes: version, wad_count, directory_offset
+------------------+
| Entry 0 data     |  Variable: tag chunks (EPNT, LINS, POLY, etc.)
| Entry 1 data     |
| ...              |
+------------------+
| Directory        |  N entries: offset + size for each entry
+------------------+
```

Each WAD entry contains multiple tagged chunks. Tags are 4-character codes (FourCC):

```
Entry (level):
  EPNT -> Endpoints (16 bytes each)
  LINS -> Lines (32 bytes each)
  SIDS -> Sides (64 bytes each)
  POLY -> Polygons (128 bytes each)
  OBJS -> Map Objects (16 bytes each)
  LITE -> Lights (100 bytes each, or 32 for old format)
  plat -> Platforms (32 bytes each)
  medi -> Media (32 bytes each)
  Minf -> Map Info (88 bytes)
  ambi -> Ambient Sounds
  bonk -> Random Sounds
  term -> Terminals
  MNpx -> Monster Physics (156 bytes each)
  PXpx -> Player Physics (104 bytes each)
  PRpx -> Projectile Physics (48 bytes each)
  WPpx -> Weapon Physics (134 bytes each)
  FXpx -> Effects Physics (14 bytes each)
```

Parsing uses `binrw` for declarative big-endian binary reading.

### MapData Construction

```rust
MapData::from_entry(wad_entry) -> Result<MapData, MapError>
```

Extracts all tags from the entry, parses each into typed vectors:
- `endpoints: Vec<Endpoint>` -- Vertex positions
- `lines: Vec<Line>` -- Edges connecting endpoints, with flags and side references
- `sides: Vec<Side>` -- Wall textures and types
- `polygons: Vec<Polygon>` -- Convex map regions with floor/ceiling heights, textures
- `objects: Vec<MapObject>` -- Entity spawn points (player, monsters, items)
- `lights: LightData` -- Static or old format light definitions
- `platforms: Vec<StaticPlatformData>` -- Moving floor/ceiling definitions
- `media: Vec<MediaData>` -- Liquid definitions
- `map_info: Option<MapInfo>` -- Level name, environment flags, song index
- Plus: ambient sounds, random sounds, annotations, terminals

### Physics Data

```rust
PhysicsData::from_entry(wad_entry) -> Result<PhysicsData, PhysicsError>
```

Extracts:
- `physics: Option<Vec<PhysicsConstants>>` -- Player movement model (typically 2: walking + running)
- `monsters: Option<Vec<MonsterDefinition>>` -- Monster stats and behavior
- `projectiles: Option<Vec<ProjectileDefinition>>` -- Projectile characteristics
- `weapons: Option<Vec<WeaponDefinition>>` -- Weapon parameters
- `effects: Option<Vec<EffectDefinition>>` -- Visual effect definitions

### Fixed-Point Conversion

Marathon stores many values as 16.16 fixed-point (i32). The parser converts these to f32:

```rust
fn fixed_to_f32(v: i32) -> f32 {
    v as f32 / 65536.0
}
```

World coordinates use i16 where 1024 = 1 world unit:

```rust
fn world_coord(v: i16) -> f32 {
    v as f32 / 1024.0
}
```

### Shapes File

```rust
ShapesFile::from_data(data) -> Result<ShapesFile, ShapeError>
```

Structure: 32 collection headers (1024 bytes), then collection data blocks.

Each Collection contains:
- Color tables (CLUTs): 256-entry palettes (16-bit RGB per entry)
- Bitmaps: raw pixel data (indexed into CLUT)
  - Can be column-order or row-order
  - Can be transparent (index 0 = transparent)
  - Wall/Interface types: uncompressed
  - Object/Scenery types: RLE compressed
- High-level shapes: named animation sequences
- Low-level shapes: individual frames with bitmap references

## Phase 3: Simulation Construction

```rust
SimWorld::new(map_data, physics_data, config) -> Result<SimWorld, SimWorldError>
```

Data flow into the ECS world:

```
MapData.endpoints + .lines + .polygons
    |
    v
build_map_geometry() -> MapGeometry resource
    - polygon_vertices: Vec<Vec<Vec2>>
    - floor_heights, ceiling_heights: Vec<f32>
    - polygon_adjacency: Vec<Vec<(line_idx, Option<adj_poly>)>>
    - line_endpoints: Vec<(Vec2, Vec2)>
    - line_solid, line_transparent: Vec<bool>

PhysicsData.physics[1] (or [0])
    |
    v
PlayerPhysicsParams::from_physics_constants()
    - Converts angular values from Marathon angle units (512 = full circle)
      to radians (factor: TAU / 512)
    - Linear velocity/acceleration values pass through unchanged

MapData.objects
    |
    v
spawn_map_objects()
    - Player (type 3): Position, Velocity, Facing, Health(150), Shield(150),
      Oxygen(600), CollisionRadius, EntityHeight, PolygonIndex, Grounded
    - Monster (type 0): Monster, MonsterState, Target, AttackCooldown,
      Position, Velocity, Facing, CollisionRadius, EntityHeight, Health,
      Immunities, Weaknesses, PolygonIndex, Grounded, SpriteShape, AnimationFrame,
      [Flying if flags & 0x0002]
    - Item (type 2): Item, Position, CollisionRadius(0.25), PolygonIndex,
      SpriteShape, AnimationFrame

MapData.platforms -> spawn_platforms() -> Platform entities
MapData.lights -> spawn_lights() -> Light entities
MapData.media -> spawn_media() -> Media entities
```

## Phase 4: Per-Tick Data Flow

```
InputState (keyboard + mouse)
    |
    v
TickInput { action_flags, mouse_yaw, mouse_pitch }
    |
    v
SimWorld::tick(input)
    |
    +-> Player entity updated (position, velocity, facing, polygon, grounded)
    +-> SimEvents pushed (sounds, teleports, damage, deaths)
    +-> Tick counter incremented
    |
    v
SimWorld query API
    +-> player_position() -> camera position
    +-> player_facing() -> camera yaw
    +-> player_vertical_look() -> camera pitch
    +-> entities() -> entity render states
    +-> drain_events() -> events for audio/integration
```

## Phase 5: Render Data Flow

### Mesh Data (one-time per level)

```
MapData
    |
    v
build_level_mesh()
    |
    v
LevelMesh { vertices, indices }
    |
    v
GPU vertex buffer + index buffer
```

### Texture Data (one-time per level)

```
collect_texture_descriptors(MapData)
    |
    v
TextureManager::load_collections(ShapesFile, descriptors)
    |
    v
LoadedCollection { bitmaps: Vec<Vec<u8>>, max_width, max_height }
    |
    v
create_gpu_textures()
    |
    v
GpuCollectionTexture { texture, view, bind_group }
```

### Per-Frame Render Data

```
CameraState (interpolated)
    |
    v
CameraUniform { view_proj, yaw, pitch, elapsed_time }
    |
    v
GPU uniform buffer write

Entity snapshots (interpolated)
    |
    v
Sprite draw calls (billboarded quads)
    |
    v
GPU vertex buffer (rebuilt each frame)
```

## Phase 6: Audio Data Flow (Desktop Only)

```
SoundsFile
    |
    v
AudioEngine::load_level(map_data, sounds_file)
    |
    +-> Decode all sound permutations (8-bit unsigned PCM -> kira Frame)
    +-> Initialize ambient manager (polygon ambient sound images)
    +-> Initialize random manager (polygon random sound triggers)
    +-> Start level music (song_index from MapInfo)
    |
    v
AudioEngine::update(dt, listener_state, events)
    |
    +-> Process AudioEvents (PlaySound, StopSound, UpdatePosition, etc.)
    +-> Update spatial parameters (distance attenuation, panning, obstruction)
    +-> Tick ambient sounds
    +-> Tick random sound triggers
    +-> Clean up finished channels
```

### Spatial Audio Pipeline

```
PlaySoundRequest { sound_index, x, y, z, source_polygon, source_entity }
    |
    v
1. Look up SoundDefinition
2. Check chance gate (random)
3. Check CANNOT_BE_RESTARTED / DOES_NOT_SELF_ABORT flags
4. Select permutation (round-robin with randomization)
5. Compute volume: base * distance_atten * rear_atten * (1 - obstruction) * sfx_volume
   - distance_atten: based on SoundBehavior (Quiet/Normal/Loud distance curves)
   - rear_atten: directional pan from listener facing
   - obstruction: polygon-graph wall count between listener and source
6. Compute pitch: random between low_pitch and high_pitch
7. Play via kira AudioManager
```

## Integration Layer Role

`marathon-integration` sits between the simulation and the frontends. It provides:

1. **Input translation**: Platform events (winit KeyEvent, web-sys KeyboardEvent) -> ActionFlags -> TickInput
2. **Game state machine**: Loading/MainMenu/Playing/Paused/Terminal/Intermission/GameOver
3. **Tick timing**: TickAccumulator with fixed 30 Hz step
4. **HUD rendering**: Health/shield/oxygen bars, weapon display, motion sensor
5. **Menu system**: Main menu, load game, preferences
6. **Terminal display**: Page layout, text rendering, image display
7. **Game modes**: Campaign, cooperative, deathmatch rule sets
8. **Film recording**: Capture/replay ActionFlags sequences
9. **Save/load**: Serialize SimWorld snapshot + metadata

Currently, marathon-game and marathon-web implement some integration concerns directly (input, camera, basic HUD on web) rather than going through the integration crate. The integration crate represents the target architecture for full game shell functionality.
