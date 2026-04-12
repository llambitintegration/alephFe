---
tags: [tier-1, game-loop, testing, golden-tests, compatibility, e2e, levels]
status: research-complete
created: 2026-04-12
---

# Golden Test Levels: The Compatibility Suite

## Purpose

This document defines ~20 specific levels from Marathon scenarios that together exercise every major engine feature. If the Rust engine can correctly load, parse, simulate, and render these levels, it has achieved broad compatibility with the Marathon ecosystem.

Each level was chosen because it stress-tests a specific subsystem. Together they form a comprehensive regression suite.

## Selection Criteria

1. **Feature coverage** -- every major engine subsystem must be exercised by at least one level
2. **Diversity** -- levels from all three trilogy games plus at least one total conversion
3. **Reproducibility** -- levels are identified by WAD entry index (stable across versions)
4. **Accessibility** -- prefer levels from freely downloadable scenarios
5. **Historical significance** -- levels the community considers canonical

## The Suite: 22 Golden Test Levels

### Marathon 2: Durandal (Primary Test Scenario)

Marathon 2 is the primary test scenario because the engine was originally built for it, it has the richest feature set of the original trilogy, and its data files are the most widely available.

#### Level 1: Waterloo Waterpark (Index 0)
| Property | Value |
|----------|-------|
| **WAD entry** | 0 |
| **Primary test** | Baseline geometry parsing, first-level smoke test |
| **Features exercised** | Basic polygon rendering, floor/ceiling textures, wall textures, liquid/water surfaces, platforms (water level switch), terminals (intro story), pattern buffers, shield rechargers, Pfhor fighters, S'pht compilers |
| **Known values** | 716 endpoints, 1106 lines, 369 polygons, 41 total WAD entries |
| **Why chosen** | First level, most thoroughly tested, establishes baseline. Water filtration plant with swimming areas tests liquid rendering. Already used as primary fixture in existing tests. |

#### Level 2: The Slings & Arrows of Outrageous Fortune (Index 1)
| Property | Value |
|----------|-------|
| **WAD entry** | 1 |
| **Primary test** | Multi-height geometry, side type variety |
| **Features exercised** | High/low/split wall types, elevation changes, stairs, varied ceiling heights, monster placement variety |
| **Why chosen** | Tests the full range of wall side types (full, high, low, split) that the mesh generator must handle correctly. |

#### Level 3: Ex Cathedra (Index 6)
| Property | Value |
|----------|-------|
| **WAD entry** | 6 |
| **Primary test** | Underwater maze geometry, oxygen mechanics |
| **Features exercised** | Extensive liquid/media areas, underwater navigation, oxygen depletion, complex maze geometry, switches controlling water levels, elevators |
| **Why chosen** | Stress-tests the liquid/media system with extensive underwater passages. Tests oxygen mechanics and water-level-changing switches. |

#### Level 4: Bob's Big Date (Index 11)
| Property | Value |
|----------|-------|
| **WAD entry** | 11 |
| **Primary test** | Dynamic water levels, platform mechanics |
| **Features exercised** | Rising/falling water levels (dynamic media), S'pht terminals, platform timing, large central room geometry |
| **Why chosen** | The constantly rising and falling water level in the central room tests dynamic media state transitions -- a critical simulation feature. |

#### Level 5: Six Thousand Feet Under (Index 12)
| Property | Value |
|----------|-------|
| **WAD entry** | 12 |
| **Primary test** | Deep underwater combat, media rendering at depth |
| **Features exercised** | Deep liquid media, underwater combat, oxygen management, light attenuation through media |
| **Why chosen** | Tests media rendering at significant depth, where light attenuation and underwater visual effects must work correctly. |

#### Level 6: If I Had a Rocket Launcher... (Index 13)
| Property | Value |
|----------|-------|
| **WAD entry** | 13 |
| **Primary test** | Large-scale combat, many simultaneous monsters |
| **Features exercised** | High monster count, multiple monster types active simultaneously, projectile physics under load, large open areas |
| **Why chosen** | Stress-tests the simulation with many active AI entities. Tests that the monster AI, projectile physics, and collision detection scale to real combat scenarios. |

#### Level 7: For Carnage, Apply Within (Index 15)
| Property | Value |
|----------|-------|
| **WAD entry** | 15 |
| **Primary test** | Arena combat, monster infighting |
| **Features exercised** | Monster-vs-monster combat, large arena geometry, multiple factions, item placement density |
| **Why chosen** | Classic arena level that tests monster AI infighting -- monsters attacking each other when hit by friendly fire. |

#### Level 8: Kill Your Television (Index 21)
| Property | Value |
|----------|-------|
| **WAD entry** | 21 |
| **Primary test** | Terminal-heavy storytelling, complex narrative flow |
| **Features exercised** | Many terminals with complex multi-page content, terminal rendering, text formatting, image embedding in terminals |
| **Why chosen** | Tests the terminal rendering system with complex narrative content. Marathon's story is delivered through terminals, so this is essential. |

#### Level 9: All Roads Lead to Sol (Index 27)
| Property | Value |
|----------|-------|
| **WAD entry** | 27 |
| **Primary test** | Final level complexity, level teleportation |
| **Features exercised** | Complex multi-area geometry, level completion triggers, final boss mechanics, endgame state handling |
| **Why chosen** | Last solo level -- tests that the full game loop can reach completion. Exercises level-end triggers and game state transitions. |

#### Level 10: Thunderdome (Index 28)
| Property | Value |
|----------|-------|
| **WAD entry** | 28 |
| **Primary test** | Net level geometry (first net map) |
| **Features exercised** | Multiplayer spawn points, arena geometry, symmetric design, item respawn positions |
| **Why chosen** | First net level in the WAD. Tests that net-specific map data (spawn points, item placement for respawn) parses correctly. Validates that the parser handles both solo and net level formats. |

### Marathon Infinity

#### Level 11: Ne Cede Malis (Index 0)
| Property | Value |
|----------|-------|
| **WAD entry** | 0 |
| **Primary test** | Marathon Infinity WAD v4 format, first-level baseline |
| **Features exercised** | WAD version 4 parsing, Infinity-specific features, basic level loading for a different scenario |
| **Why chosen** | First level of Marathon Infinity. Verifies that the v4 WAD format (with extended directory entries) parses correctly. Establishes baseline for Infinity compatibility. |

#### Level 12: Electric Sheep One (Index 4)
| Property | Value |
|----------|-------|
| **WAD entry** | 4 |
| **Primary test** | Dream level geometry, lava media, switch-activated platforms |
| **Features exercised** | Lava media type (vs water), platform bridges rising from lava, shootable switches, small constrained geometry |
| **Why chosen** | Dream levels use a distinctive style with lava, rising bridges, and shootable switches. Tests media type differentiation (lava vs water) and projectile-switch interaction. |

#### Level 13: A Converted Church in Venice, Italy (Index 19)
| Property | Value |
|----------|-------|
| **WAD entry** | 19 |
| **Primary test** | Complex architectural geometry |
| **Features exercised** | Intricate level design, many polygons, complex sight lines, landscape textures, elevated platforms |
| **Why chosen** | One of the most architecturally complex levels in the trilogy. Tests that the renderer handles intricate geometry with many overlapping sight lines. |

#### Level 14: Aye Mak Sicur (Index 24)
| Property | Value |
|----------|-------|
| **WAD entry** | 24 |
| **Primary test** | Endgame complexity, final level parsing |
| **Features exercised** | Complex level design, heavy combat, level completion mechanics, all monster types |
| **Why chosen** | Final real level of Marathon Infinity. Tests end-of-game state handling for the Infinity scenario format. |

### Marathon 1

#### Level 15: Arrival (Index 0)
| Property | Value |
|----------|-------|
| **WAD entry** | 0 |
| **Primary test** | Marathon 1 WAD v2 format, M1-specific rendering |
| **Features exercised** | WAD version 2 parsing, M1-specific collection handling, M1 HUD layout, M1 physics model |
| **Why chosen** | First level of Marathon 1. Verifies backward compatibility with the oldest WAD format. M1 uses different collection indexing and HUD layout. |

#### Level 16: Colony Ship For Sale, Cheap (Index 13)
| Property | Value |
|----------|-------|
| **WAD entry** | 13 |
| **Primary test** | Complex platform puzzles, M1 platform mechanics |
| **Features exercised** | Extensive platform height adjustment puzzles, multiple linked platforms, switch-platform dependencies, backtracking through platform states |
| **Why chosen** | Infamous for its tedious platform puzzle. Stress-tests the platform state machine with many interdependent platforms that must be raised/lowered in sequence. This is the single best test of platform mechanics in the entire trilogy. |

#### Level 17: Pfhor Your Eyes Only (Index 16)
| Property | Value |
|----------|-------|
| **WAD entry** | 16 |
| **Primary test** | Vacuum/oxygen environments (M1 specific) |
| **Features exercised** | Vacuum environment type, oxygen depletion without liquid, space exposure mechanics, M1-specific polygon types |
| **Why chosen** | Tests the vacuum polygon type where oxygen depletes without any visible liquid. This is a Marathon 1 feature that differs from M2/Infinity's underwater oxygen depletion. |

### Total Conversions (Extended Suite)

#### Level 18: Rubicon X -- Level 0
| Property | Value |
|----------|-------|
| **WAD entry** | 0 |
| **Primary test** | Custom shapes/sounds loading, extensive MML |
| **Features exercised** | Full shapes replacement, full sounds replacement, custom physics, extensive MML overrides, high-resolution artwork |
| **Why chosen** | First level of the largest total conversion. If this loads and renders, the MML override system and custom asset pipeline work correctly. Tests that every shapes collection loads when fully replaced. |
| **Data source** | [marathonrubicon.com](https://www.marathonrubicon.com/) (manual download) |

#### Level 19: Eternal X -- Level 0
| Property | Value |
|----------|-------|
| **WAD entry** | 0 |
| **Primary test** | Lua scripting (dynamic music), custom assets |
| **Features exercised** | Solo Lua script initialization, dynamic music scripting, custom shapes, custom sounds, custom physics, MML overrides |
| **Why chosen** | Tests the Lua VM integration with a real-world Lua script that controls music transitions. If the Lua VM initializes and the first level loads, basic Lua compatibility is verified. |
| **Data source** | [eternal.bungie.org](http://eternal.bungie.org) (manual download) |

#### Level 20: Apotheosis X -- Level 0
| Property | Value |
|----------|-------|
| **WAD entry** | 0 |
| **Primary test** | HUD Lua, high-frame-rate animations |
| **Features exercised** | HUD Lua script execution, high-frame-rate sprite animations, OpenGL texture overrides, custom shapes with many animation frames |
| **Why chosen** | Tests the HUD Lua system and the engine's ability to handle high-frame-rate animation sequences in shapes. Pushes the visual rendering pipeline to its aesthetic limits. |
| **Data source** | [Simplici7y](https://simplici7y.com/items/apotheosis-x-5/) (manual download) |

#### Level 21: Tempus Irae Redux -- Level 0
| Property | Value |
|----------|-------|
| **WAD entry** | 0 |
| **Primary test** | OpenGL texture override pipeline |
| **Features exercised** | Extensive external texture replacement (every texture rebuilt from scratch), 13 landscape textures (vs standard 4), animated textures, weather effects (rain, snow, fog, lightning), advanced architecture |
| **Why chosen** | The single best stress test for the texture override pipeline. If Tempus Irae Redux renders correctly, the OpenGL texture replacement system handles the extreme case. |
| **Data source** | [tempusirae.org](https://tempusirae.org/) (manual download) |

#### Level 22: Istoria -- Level 0
| Property | Value |
|----------|-------|
| **WAD entry** | 0 |
| **Primary test** | Advanced Lua scripting (RPG systems) |
| **Features exercised** | Lua RPG class system, spell casting, inventory management, NPC communication, character progression, advanced Lua API usage |
| **Why chosen** | Pushes Lua scripting to its absolute limits. Implements a full RPG system in Lua. If this scenario's scripts execute without errors, the Lua VM is feature-complete. |
| **Data source** | [ModDB](https://www.moddb.com/mods/marathon-istoria) (manual download) |

## Feature Coverage Matrix

This shows which golden levels cover which engine features:

| Engine Feature | Levels That Test It |
|---------------|-------------------|
| WAD v2 parsing | 15, 16, 17 |
| WAD v4 parsing | 11, 12, 13, 14 |
| WAD v2 (M2 compat) | 1-10 |
| Basic geometry | ALL |
| Wall side types (full) | 1, 2 |
| Wall side types (high/low/split) | 2 |
| Floor/ceiling textures | ALL |
| Liquid/media (water) | 1, 3, 4, 5 |
| Liquid/media (lava) | 12 |
| Dynamic media levels | 4 |
| Platform mechanics | 1, 4, 12, 16 |
| Complex platform puzzles | 16 |
| Oxygen depletion (underwater) | 3, 5 |
| Oxygen depletion (vacuum) | 17 |
| Terminal rendering | 1, 8 |
| Monster AI (basic) | 1, 6, 7 |
| Monster AI (infighting) | 7 |
| High monster count | 6 |
| Projectile physics | 6, 7, 12 |
| Level teleportation | 9, 14 |
| Pattern buffers | 1 |
| Shield rechargers | 1 |
| Net level parsing | 10 |
| Landscape textures | 13, 21 |
| MML overrides (extensive) | 18, 19, 20, 21 |
| Custom shapes (full replacement) | 18, 19, 20, 21, 22 |
| Custom sounds (full replacement) | 18, 19 |
| Custom physics | 18, 19 |
| Solo Lua scripting | 19, 22 |
| HUD Lua scripting | 20 |
| OpenGL texture overrides | 21 |
| High-frame-rate animations | 20 |
| Weather effects | 21 |
| Lua RPG systems | 22 |

## Test Execution Plan

### Phase 1: Trilogy Parse-Only (Automated CI)

**Data acquisition:** Download from GitHub releases (scripted).

**Tests to run on every commit:**
```
Level 1  (M2:0)   - Parse geometry, verify snapshot values (716 ep, 1106 ln, 369 poly)
Level 11 (MInf:0) - Parse v4 WAD, verify entry count
Level 15 (M1:0)   - Parse v2 WAD, verify M1-specific format
```

**Tests to run nightly:**
```
Levels 1-10  - All M2 golden levels: parse + validate
Level 11-14  - All MInf golden levels: parse + validate
Level 15-17  - All M1 golden levels: parse + validate
```

### Phase 2: Trilogy Simulation (Automated CI)

**Tests to run on every commit:**
```
Level 1  - Init sim, tick 60 frames, verify player alive
Level 4  - Init sim, verify dynamic media state changes
Level 16 - Init sim, verify platform state machine
```

**Tests to run nightly:**
```
All 17 trilogy levels - Init sim, tick 60 frames, zero panics
Determinism test     - Run level 1 twice, verify identical state
Physics golden test  - Level 1, walk forward 30 ticks, verify position
```

### Phase 3: Render Regression (Weekly, Docker + Playwright)

```
Level 1  - Golden screenshot at spawn position
Level 3  - Golden screenshot of underwater maze entrance
Level 12 - Golden screenshot of lava/bridge room
Level 13 - Golden screenshot of architectural geometry
```

### Phase 4: Total Conversion Compatibility (Monthly, Manual Trigger)

```
Level 18 (Rubicon X:0)  - Parse + sim init
Level 19 (Eternal X:0)  - Parse + Lua init
Level 20 (Apotheosis:0) - Parse + HUD Lua init
Level 21 (Tempus:0)     - Parse + texture override resolution
Level 22 (Istoria:0)    - Parse + Lua script parse
```

## Known Snapshot Values (Golden Reference)

These values are from the Marathon 2 data files and serve as regression anchors:

```json
{
  "marathon-2": {
    "wad_version": 2,
    "entry_count": 41,
    "levels": {
      "0": {
        "name": "Waterloo Waterpark",
        "endpoints": 716,
        "lines": 1106,
        "polygons": 369
      },
      "1": {
        "name": "The Slings & Arrows of Outrageous Fortune"
      },
      "6": {
        "name": "Ex Cathedra"
      },
      "11": {
        "name": "Bob's Big Date"
      },
      "12": {
        "name": "Six Thousand Feet Under"
      },
      "13": {
        "name": "If I Had a Rocket Launcher, I'd Make Somebody Pay"
      },
      "15": {
        "name": "For Carnage, Apply Within"
      },
      "21": {
        "name": "Kill Your Television"
      },
      "27": {
        "name": "All Roads Lead To Sol..."
      },
      "28": {
        "name": "Thunderdome"
      }
    }
  },
  "marathon-infinity": {
    "wad_version": 4,
    "levels": {
      "0": { "name": "Ne Cede Malis" },
      "4": { "name": "Electric Sheep One" },
      "19": { "name": "A Converted Church in Venice, Italy" },
      "24": { "name": "Aye Mak Sicur" }
    }
  },
  "marathon-1": {
    "wad_version": 2,
    "levels": {
      "0": { "name": "Arrival" },
      "13": { "name": "Colony Ship For Sale, Cheap" },
      "16": { "name": "Pfhor Your Eyes Only..." }
    }
  }
}
```

## How to Add a New Golden Level

1. Identify the feature gap in the coverage matrix above
2. Find a level that exercises that feature
3. Add an entry to this document with WAD entry index, primary test purpose, and features exercised
4. Record snapshot values (geometry counts, known positions) by running against reference Aleph One
5. Add a Rust test or Playwright test for the new level
6. Update the coverage matrix

## Related Notes

- [[scenario-feature-matrix]] -- Full scenario inventory and feature matrix
- [[e2e-tdd-framework]] -- The TDD framework that uses these levels as the standard
- [[community-content-ecosystem]] -- Where to obtain scenario data
- [[platform-mechanics]] -- Platform mechanics that Level 16 stress-tests
- [[projectile-physics]] -- Projectile physics that Levels 6/7 exercise
