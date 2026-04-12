---
tags: [tier-1, game-loop, testing, scenarios, compatibility, e2e]
status: research-complete
created: 2026-04-12
---

# Scenario Feature Matrix

## Purpose

This matrix documents every major Marathon scenario/total conversion and the engine features each exercises. It serves as the compatibility specification for the Rust engine rebuild: if the engine can load, parse, and run these scenarios correctly, it is compatible with the Marathon ecosystem.

## Scenario Inventory

### Tier 1: Original Trilogy (Baseline -- Must Pass)

These are the minimum compatibility targets. Data files are freely redistributable under Bungie's limited license and are available from official GitHub repositories.

| Property | Marathon 1 | Marathon 2: Durandal | Marathon Infinity |
|----------|-----------|---------------------|-------------------|
| **Solo levels** | 27 (levels 0-26) | 28 (levels 0-27) | 25 solo + 3 vidmaster |
| **Net levels** | 10 | 13 (levels 28-40) | 23 |
| **Total WAD entries** | 37 | 41 | ~57 |
| **WAD version** | 2 (M1-compat) | 2 | 4 |
| **Data format** | Map.scen, Shapes.shps, Sounds.sndz, Physics.phys | Map, Shapes, Sounds, Physics Model | Map.sceA, Shapes.shpA, Sounds.sndA |
| **Download source** | [GitHub data-marathon](https://github.com/Aleph-One-Marathon/data-marathon) | [GitHub data-marathon-2](https://github.com/Aleph-One-Marathon/data-marathon-2) | [GitHub data-marathon-infinity](https://github.com/Aleph-One-Marathon/data-marathon-infinity) |
| **Release download** | [20250829 ZIP](https://github.com/Aleph-One-Marathon/alephone/releases/download/release-20250829/Marathon-20250829-Data.zip) | [20250829 ZIP](https://github.com/Aleph-One-Marathon/alephone/releases/download/release-20250829/Marathon2-20250829-Data.zip) | [20250829 ZIP](https://github.com/Aleph-One-Marathon/alephone/releases/download/release-20250829/MarathonInfinity-20250829-Data.zip) |
| **Licensing** | Bungie limited license (free non-commercial redistribution) | Same | Same |

### Tier 2: Major Total Conversions (High Priority)

| Property | Rubicon X | Eternal X | Phoenix | Apotheosis X | Tempus Irae Redux |
|----------|-----------|-----------|---------|-------------|-------------------|
| **Author** | Chris Lund et al. | Pfhorrest / Xeventh Project | RyokoTK | hypersleep | Nardo / Chris |
| **Solo levels** | 83 | 52 | 35 | 24 | 49 (incl. Lost Levels) |
| **Net levels** | Unknown | Unknown | Unknown | Unknown | 79 |
| **Dev time** | Years | Since 2004 | Years | 15 years | Since 1997 |
| **Download** | [marathonrubicon.com](https://www.marathonrubicon.com/) | [eternal.bungie.org](http://eternal.bungie.org) | [Simplici7y](https://simplici7y.com/items/marathon-phoenix-2) | [Simplici7y](https://simplici7y.com/items/apotheosis-x-5/) | [tempusirae.org](https://tempusirae.org/) |
| **AO version req** | 2004 July build+ | Recent | 1.5+ | Recent | 1.10.1+ |
| **Licensing** | Community freeware | Community freeware | Community freeware | Community freeware | Community freeware |

### Tier 3: Specialized Test Scenarios

| Property | Istoria | Yuge | Trojan | EVIL | RED |
|----------|---------|------|--------|------|-----|
| **Author** | windbreaker | Various | Community (first TC ever) | Various | Various |
| **Solo levels** | Unknown (RPG) | 30 + 225 secret | 25 | Multi-level | Multi-level |
| **Primary test value** | Lua stress test | Scale/stability test | Historical baseline | Horror atmosphere | Difficulty tuning |
| **Download** | [ModDB](https://www.moddb.com/mods/marathon-istoria) | [Simplici7y](https://simplici7y.com/items/mararthon-yuge/) | [Simplici7y](https://simplici7y.com/items/marathon-trojan/) | [Citadel](https://citadel.lhowon.org/scenarios/marathon-evil/) | [Citadel](https://citadel.lhowon.org/scenarios/marathon-red/) |

## Engine Feature Matrix

This matrix shows which engine subsystems each scenario exercises. Use it to prioritize implementation and to design targeted tests.

### Data Loading Features

| Feature | M1 | M2 | MInf | Rubicon X | Eternal X | Phoenix | Apotheosis X | Tempus Irae | Istoria | Yuge |
|---------|----|----|------|-----------|-----------|---------|-------------|-------------|---------|------|
| WAD v2 parsing | YES | YES | -- | -- | -- | -- | -- | -- | -- | -- |
| WAD v4 parsing | -- | -- | YES | YES | YES | YES | YES | YES | YES | YES |
| Map geometry (endpoints/lines/polygons) | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Map sides (wall textures) | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Platform data | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Light data (static) | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Media/liquid data | -- | YES | YES | YES | YES | YES | YES | YES | YES | ? |
| Terminal data | YES | YES | YES | YES | YES | YES | YES | YES | YES | min |
| Annotation data | ? | YES | YES | YES | YES | YES | YES | YES | YES | ? |
| Object placement | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Ambient/random sounds | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Shapes file (8-bit) | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Shapes file (16-bit) | -- | ? | ? | YES | YES | YES | YES | YES | YES | ? |
| Sounds file | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Physics model (monsters) | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Physics model (weapons) | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Physics model (projectiles) | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Physics model (player constants) | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |

### Content Customization Features

| Feature | M1 | M2 | MInf | Rubicon X | Eternal X | Phoenix | Apotheosis X | Tempus Irae | Istoria | Yuge |
|---------|----|----|------|-----------|-----------|---------|-------------|-------------|---------|------|
| MML overrides (minimal) | YES | YES | YES | -- | -- | -- | -- | -- | -- | -- |
| MML overrides (extensive) | -- | -- | -- | YES | YES | YES | YES | YES | YES | YES |
| Custom shapes (full replacement) | -- | -- | -- | YES | YES | YES | YES | YES | YES | -- |
| Custom sounds (full replacement) | -- | -- | -- | YES | YES | YES | YES | YES | YES | -- |
| Custom physics | -- | -- | -- | YES | YES | YES | YES | ? | YES | YES |
| Shapes patches (.ShPa) | -- | -- | -- | ? | ? | ? | ? | ? | ? | -- |
| OpenGL texture overrides | -- | -- | -- | ? | ? | ? | YES | YES | ? | -- |
| Animated textures (extended) | -- | basic | basic | YES | YES | YES | YES | YES | YES | -- |
| High-frame-rate animations | -- | -- | -- | -- | -- | -- | YES | -- | -- | -- |

### Scripting Features

| Feature | M1 | M2 | MInf | Rubicon X | Eternal X | Phoenix | Apotheosis X | Tempus Irae | Istoria | Yuge |
|---------|----|----|------|-----------|-----------|---------|-------------|-------------|---------|------|
| Solo Lua scripting | -- | -- | -- | -- | YES | -- | ? | -- | **YES** | -- |
| HUD Lua scripting | -- | -- | -- | -- | ? | -- | YES | -- | ? | -- |
| Lua dynamic music | -- | -- | -- | -- | YES | -- | -- | -- | -- | -- |
| Lua RPG systems | -- | -- | -- | -- | -- | -- | -- | -- | **YES** | -- |
| Lua NPC communication | -- | -- | -- | -- | -- | -- | -- | -- | **YES** | -- |
| Lua inventory/spells | -- | -- | -- | -- | -- | -- | -- | -- | **YES** | -- |
| Level-embedded MML (MMLS tags) | ? | ? | ? | YES | YES | YES | YES | YES | YES | ? |

### Rendering Features

| Feature | M1 | M2 | MInf | Rubicon X | Eternal X | Phoenix | Apotheosis X | Tempus Irae | Istoria | Yuge |
|---------|----|----|------|-----------|-----------|---------|-------------|-------------|---------|------|
| Basic polygon rendering | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Floor/ceiling textures | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Wall textures (full/high/low/split) | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Transparent walls | -- | YES | YES | YES | YES | YES | YES | YES | YES | ? |
| Liquid surface rendering | -- | YES | YES | YES | YES | YES | YES | YES | YES | ? |
| Landscape textures | YES | YES | YES | YES | YES | YES | YES | **13 landscapes** | YES | min |
| Sprite rendering | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Transfer modes (normal) | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Transfer modes (glow/pulsate) | ? | YES | YES | YES | YES | YES | YES | YES | YES | ? |
| Dynamic lighting | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Weather effects (rain/snow/fog) | -- | -- | -- | -- | -- | -- | -- | YES | -- | -- |
| Camera scripting | -- | -- | -- | -- | ? | -- | ? | -- | ? | -- |

### Simulation Features

| Feature | M1 | M2 | MInf | Rubicon X | Eternal X | Phoenix | Apotheosis X | Tempus Irae | Istoria | Yuge |
|---------|----|----|------|-----------|-----------|---------|-------------|-------------|---------|------|
| Player movement physics | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Monster AI (all types) | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Weapon mechanics | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Projectile physics | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Platform mechanics | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Switch/terminal activation | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Item pickup | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Level teleportation | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Liquid/media physics | -- | YES | YES | YES | YES | YES | YES | YES | YES | ? |
| Oxygen depletion | YES | YES | YES | YES | YES | YES | YES | YES | YES | ? |
| Vacuum environments | YES | -- | ? | ? | ? | ? | ? | ? | ? | -- |
| Pattern buffers (save) | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Shield rechargers | YES | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| Dynamic limits (raised) | default | default | default | likely | likely | default | likely | likely | likely | default |

## Scenario Acquisition for CI

### Freely Downloadable (Can automate in CI)

| Scenario | Method | URL |
|----------|--------|-----|
| Marathon 1 | `git clone` or ZIP download | `https://github.com/Aleph-One-Marathon/data-marathon` |
| Marathon 2 | `git clone` or ZIP download | GitHub release ZIP (see Tier 1 table) |
| Marathon Infinity | `git clone` or ZIP download | `https://github.com/Aleph-One-Marathon/data-marathon-infinity` |

### Community Downloads (Manual or scripted)

| Scenario | Method | Notes |
|----------|--------|-------|
| Rubicon X | HTTP download from marathonrubicon.com | Check ToS before automated download |
| Eternal X | HTTP download from eternal.bungie.org | Check ToS |
| Phoenix | Simplici7y download page | Requires navigating download link |
| Apotheosis X | Simplici7y download page | Same |
| Tempus Irae Redux | tempusirae.org | Same |
| Istoria | ModDB download | Requires ModDB flow |
| Yuge | Simplici7y download page | Same |

### Licensing Summary for Test Use

- **Marathon Trilogy**: Bungie granted limited license for free non-commercial redistribution (2005/2021). Data files can be used in CI testing. Cannot be committed to repository but can be downloaded as CI artifacts.
- **Community scenarios**: Generally distributed as freeware. Most do not have explicit open-source licenses. Using them as test fixtures requires downloading at test time, not bundling in the repository.
- **Recommendation**: CI pipeline should download trilogy data from GitHub releases at test time. Community scenarios should be an optional "extended compatibility" test tier that runs on-demand.

## Priority Implementation Order

Based on this matrix, the implementation priority for the Rust engine is:

1. **WAD parsing (v2 + v4)** -- unlocks all scenarios
2. **Map geometry** -- endpoints, lines, polygons, sides
3. **Shapes file loading** -- 8-bit and 16-bit collections
4. **Sounds file loading** -- all sound definitions
5. **Physics model loading** -- monsters, weapons, projectiles, player constants
6. **Platform/light/media data** -- dynamic level elements
7. **Terminal data** -- story delivery mechanism
8. **MML parsing and application** -- unlocks all total conversions
9. **Basic rendering pipeline** -- floors, ceilings, walls, sprites
10. **Solo Lua VM** -- unlocks Eternal X, Istoria
11. **HUD Lua VM** -- unlocks Apotheosis X
12. **OpenGL texture overrides** -- unlocks Tempus Irae Redux

## Related Notes

- [[community-content-ecosystem]] -- Detailed overview of the content ecosystem
- [[e2e-tdd-framework]] -- How to use these scenarios as the test standard
- [[golden-test-levels]] -- Specific levels selected as the compatibility test suite
- [[mml-override-system]] -- MML parsing and application details
- [[lua-vm-integration]] -- Lua scripting integration details
