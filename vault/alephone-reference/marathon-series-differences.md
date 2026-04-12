---
tags: [alephone, reference, marathon-series, history]
---

# Marathon Series Differences

The Marathon trilogy consists of three games built on progressively enhanced engines. Aleph One is the open-source continuation based on Marathon Infinity's engine.

## Timeline

| Game | Year | Platform | Engine |
|------|------|----------|--------|
| Marathon | 1994 | Macintosh | Marathon 1 engine |
| Marathon 2: Durandal | 1995 | Mac, Windows | Marathon 2 engine |
| Marathon Infinity | 1996 | Macintosh | Modified Marathon 2 engine |
| Aleph One | 2000-present | Cross-platform | Open-sourced Marathon 2/Infinity |

## Marathon 1 (1994)

### Engine Characteristics
- Software renderer only
- 256-color indexed graphics
- Limited polygon types
- Simpler monster AI
- No liquids/media
- Simpler light model
- Limited sound system

### File Format Differences
- Uses "old" map format with `PNTS` tags instead of `EPNT`
- Physics tags use lowercase FourCCs: `phys`, `mons`, `proj`, `weap`, `effe`
- Smaller record sizes for some structures
- Old light format (32 bytes vs 100 bytes for static lights)
- No media data
- Simpler terminal format
- WAD version 0

### Unique Features
- The only game to feature the original vacuum mechanic where oxygen depletes in vacuum-flagged polygons
- Introductory storyline establishing the Marathon universe

## Marathon 2: Durandal (1995)

### Major Engine Additions
- **Liquids (Media)**: Water, lava, goo, sewage with swim mechanics, drag, oxygen depletion, and damage
- **Ambient Sounds**: Per-polygon ambient sound images for environmental audio
- **Random Sounds**: Per-polygon random environmental sound triggers
- **Scripted Teleportation**: NPCs and items can be teleported via script
- **External Map Loading**: Maps can be loaded from separate files (enabling user scenarios)
- **Higher Resolutions**: Support for resolutions beyond 640x480
- **Higher Color Depths**: 16-bit and true-color rendering options
- **Improved Sound**: Better quality audio with spatial positioning
- **Improved Performance**: Optimized rendering engine

### File Format Changes
- New "static" light format (100 bytes, multiple function states)
- `EPNT` tag replaces `PNTS` (16-byte endpoints vs 4-byte points)
- Media tag (`medi`) added
- Ambient sound tag (`ambi`) added
- Random sound tag (`bonk`) added
- Physics tags use uppercase FourCCs: `PXpx`, `MNpx`, `PRpx`, `WPpx`, `FXpx`
- WAD version 1-2
- 128-byte polygon records (vs smaller in M1)
- Platform records expanded (32 bytes)

### Gameplay
- Larger, more complex levels
- Swimming mechanics
- More diverse environments
- Richer terminal narrative
- Multiple monster infighting
- Windows port (first cross-platform release)

## Marathon Infinity (1996)

### Engine Modifications (from Marathon 2)
- **Branching Campaigns**: Non-linear level progression based on player actions
- **Per-Level Physics**: Each level can have its own complete physics model
- **Separate Physics Files**: Physics definitions can be externalized
- Very little core engine code changed from Marathon 2

### Content Tools
The most significant addition was the inclusion of Bungie's development tools:
- **Forge**: Level editor (the same tool Bungie used internally)
- **Anvil**: Physics and shapes editor

These tools spawned the Marathon modding community and made scenario creation accessible.

### File Format
- Essentially the same as Marathon 2
- WAD version 2
- Added support for physics model references per-level in MapInfo
- Campaign branch data in terminal definitions

### Gameplay
- Larger and more complex levels than Marathon 2
- More intricate plot with timeline-jumping narrative
- One new weapon: KKV-7 10mm SMG Flechette
- Harder difficulty overall
- Non-linear progression through dream/alternate-reality levels

## Aleph One (2000-present)

### Engine Enhancements Over Infinity
- **Cross-Platform**: Linux, macOS, Windows (via SDL)
- **OpenGL Rendering**: Hardware-accelerated rendering alongside software renderer
- **Lua Scripting**: Extensible game logic via Lua scripts
- **MML Support**: Marathon Markup Language for configuration
- **Plugin System**: WAD-based plugins with shapes/sounds/MML/Lua patches
- **Higher Resolutions**: Arbitrary resolution support
- **Widescreen**: Native widescreen aspect ratios
- **3D Models**: OBJ model import for replacing sprites
- **Modern Audio**: OpenAL audio with OGG support
- **Networking**: Improved multiplayer with star topology
- **Film Format**: Enhanced demo/replay recording
- **Save Game**: Full game state serialization

### File Format Extensions
- WAD version 4
- Lua script tags (`LUAS`)
- MML script tags (`MMLS`)
- Shape patch tags (`ShPa`)
- Sound patch tags (`SnPa`)
- Save meta/image tags
- Plugin metadata format

## Format Compatibility Matrix

| Feature | M1 | M2 | MInf | AO |
|---------|----|----|------|----|
| PNTS (old points) | Yes | -- | -- | Read |
| EPNT (endpoints) | -- | Yes | Yes | Yes |
| Static lights (100B) | -- | Yes | Yes | Yes |
| Old lights (32B) | Yes | -- | -- | Read |
| Media | -- | Yes | Yes | Yes |
| Ambient sounds | -- | Yes | Yes | Yes |
| Random sounds | -- | Yes | Yes | Yes |
| M1 physics tags | Yes | -- | -- | Read |
| M2 physics tags | -- | Yes | Yes | Yes |
| Per-level physics | -- | -- | Yes | Yes |
| Lua scripting | -- | -- | -- | Yes |
| MML | -- | -- | -- | Yes |
| Plugins | -- | -- | -- | Yes |

## Implications for the Rust Rebuild

The `marathon-formats` crate handles all format versions:
- Both `PNTS` and `EPNT` endpoint tags
- Both old (32B) and static (100B) light formats
- Both M1 (`mons`, `phys`, etc.) and M2 (`MNpx`, `PXpx`, etc.) physics tags
- Media/ambient/random sound tags (absent in M1)
- Lua/MML/plugin tags (Aleph One extensions)

The `marathon-sim` crate targets Marathon 2/Infinity behavior as the baseline, since Aleph One is based on the Infinity engine. Marathon 1 compatibility would primarily be about handling the older file formats, as the gameplay mechanics are largely a subset of Marathon 2's.

The `WadTag` enum in `marathon-formats/src/tags.rs` exhaustively enumerates all known tag types across all versions (60+ tags).
