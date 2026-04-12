---
tags: [tier-3, content-pipeline, community, scenarios, total-conversions]
status: research-complete
---

# Community Content Ecosystem

## Overview

The Marathon modding community has produced dozens of scenarios, total conversions, and plugins over more than two decades. Understanding what this content requires from the engine is essential for ensuring the Rust rebuild achieves compatibility with the existing content library. The primary distribution hub is [Simplici7y](https://simplici7y.com/), with official scenario downloads available at [alephone.lhowon.org/scenarios.html](https://alephone.lhowon.org/scenarios.html).

## The Marathon Trilogy (Baseline Requirements)

These three games are the minimum compatibility target. All use the standard Aleph One data formats.

### Marathon (Marathon 1)
- **Levels:** ~27
- **Engine features used:** Basic rendering, AI, weapons, terminals, pattern buffers
- **Special notes:** Originally a separate engine; ported to Aleph One with some compatibility shims. Uses Marathon 1 physics and HUD layout. Requires M1-specific collection handling.

### Marathon 2: Durandal
- **Levels:** ~28
- **Engine features used:** Full Aleph One feature set (this is the base game the engine was built for). Liquids/media, platforms, switches, all monster types, all weapons.

### Marathon Infinity
- **Levels:** ~20 (plus Forge/Anvil tools for creation)
- **Engine features used:** Same as Marathon 2, plus additional visual effects, more complex level designs, and the scenario framework that all mods build upon.

## Major Total Conversions

### Marathon: Rubicon X
- **Author:** Chris Lund and team
- **Scale:** 83 levels (one of the largest ever created)
- **Content:** All new high-resolution artwork, new and updated maps, new story set 50 years after Marathon Infinity
- **Engine features required:**
  - MML for extensive configuration overrides
  - Custom shapes (complete texture/sprite replacement)
  - Custom sounds
  - Custom physics models
  - Advanced terminal scripting
- **Compatibility priority:** HIGH -- considered the "unofficial fourth chapter"

### Marathon: Eternal X
- **Author:** Forrest Cameranesi (Pfhorrest) and the Xeventh Project team
- **Scale:** 52 levels
- **Content:** All new levels, textures, weapons, music, and several new creatures
- **Engine features required:**
  - Lua scripting for dynamic music (music changes mid-level based on game events)
  - MML for all game data overrides
  - Custom shapes and sounds (complete replacement)
  - Custom physics
  - Advanced level scripting
- **Special notes:** One of the oldest scenarios still being actively developed (since 2004). Continuously refined and expanded.
- **Compatibility priority:** HIGH

### Marathon: Phoenix
- **Author:** RyokoTK
- **Scale:** 35 levels
- **Content:** Full arsenal of new, powerful weapons and more threatening enemies
- **Engine features required:**
  - MML for weapon/monster customization
  - Custom shapes and sounds
  - Custom physics (difficulty tuning)
  - Requires Aleph One 1.5 or newer
- **Compatibility priority:** HIGH

### Apotheosis X
- **Author:** hypersleep
- **Scale:** 24 levels with original soundtrack
- **Content:** New sprites, textures, high frame-rate animations, new enemies and weapons
- **Engine features required:**
  - Lua scripting (custom HUD)
  - MML for extensive customization
  - Custom shapes with high frame-rate animation sequences
  - Custom sounds and music
  - Pushes aesthetic boundaries of the engine
- **Special notes:** 15 years in development. Demonstrates the upper bounds of what the Marathon engine can achieve visually.
- **Compatibility priority:** HIGH -- showcases advanced engine features

### Trojan
- **Author:** Bungie community (first total conversion ever)
- **Scale:** 25 levels
- **Content:** New artwork, weapons, enemies, and music
- **Engine features required:**
  - Basic MML overrides
  - Custom shapes and sounds
  - Standard physics modifications
- **Compatibility priority:** MEDIUM -- historically significant, simpler feature requirements

### Marathon: EVIL
- **Scale:** Multi-level conversion for Marathon Infinity
- **Content:** New weapons and monsters, horror atmosphere
- **Engine features required:**
  - Custom shapes and sounds
  - Physics modifications
  - MML overrides
- **Compatibility priority:** MEDIUM

### Marathon RED
- **Content:** Survival horror style, considered the most difficult conversion
- **Engine features required:**
  - Custom shapes, sounds, textures
  - MML overrides for difficulty tuning
  - Custom physics
- **Compatibility priority:** MEDIUM

### Tempus Irae Redux
- **Content:** Time travel to Renaissance Italy with completely rebuilt HD textures
- **Engine features required:**
  - Extensive OpenGL texture overrides (every texture rebuilt from scratch)
  - Animated textures (expanded from 4 to 13 landscapes)
  - MML for texture configuration
  - Advanced architecture requiring solid geometry rendering
- **Special notes:** One of the oldest scenarios still actively developed. Heavy reliance on OpenGL texture replacement system.
- **Compatibility priority:** HIGH -- stress-tests the texture pipeline

### Marathon: Yuge
- **Scale:** 30 levels + 225 secret levels (!)
- **Content:** Minimalist textures, Marathon 2-style combat, procedurally generated levels, inside jokes
- **Engine features required:**
  - Standard MML and physics
  - Large level count support
  - Demonstrates engine stability with massive content volumes
- **Compatibility priority:** LOW -- valuable for stress testing

### Marathon: Istoria
- **Content:** RPG scenario with character progression, seven player classes, spell system
- **Engine features required:**
  - **Advanced Lua scripting** (RPG systems, class abilities, spell casting, inventory management, NPC communication)
  - MML for extensive game data overrides
  - Custom shapes and sounds
  - Original soundtrack
  - Requires latest Aleph One version
- **Special notes:** Pushes Lua scripting to its limits. Implements RPG mechanics (classes, spells, XP progression) entirely through Lua. Co-op support requires additional Lua scripts.
- **Compatibility priority:** HIGH -- exercises the most complex Lua features

## Plugin Ecosystem

Beyond full scenarios, the community produces many smaller plugins:

### HUD Plugins
- **Enhanced HUD** -- Custom Lua-drawn HUD with additional information
- **Basic HUD** -- Simplified HUD alternative
- These are distributed as plugins with `hud_lua` scripts

### Visual Enhancement Plugins
- HD texture packs (shapes patches with `requires_opengl="true"`)
- Transparent sprite plugins (MML `<opengl>` texture transparency settings)
- Custom landscape packs

### Gameplay Modification Plugins
- Custom physics models (difficulty adjustments, weapon rebalancing)
- Solo Lua scripts that add gameplay features (regeneration, new mechanics)

### Theme Plugins
- UI theme replacements (`theme_dir` in Plugin.xml)
- Custom fonts, colors, menu layouts

## Engine Feature Dependencies Summary

This matrix shows which engine features are required by major scenarios:

| Feature | Trilogy | Rubicon X | Eternal X | Phoenix | Apotheosis X | Istoria |
|---------|---------|-----------|-----------|---------|-------------|---------|
| Basic rendering | YES | YES | YES | YES | YES | YES |
| MML overrides | minimal | extensive | extensive | extensive | extensive | extensive |
| Custom shapes | no | full replacement | full replacement | full replacement | full replacement | full replacement |
| Custom sounds | no | full replacement | full replacement | full replacement | full replacement | full replacement |
| Custom physics | no | YES | YES | YES | YES | YES |
| Solo Lua | no | no | YES (music) | no | unknown | YES (RPG systems) |
| HUD Lua | no | no | unknown | no | YES | unknown |
| OpenGL textures | no | possible | possible | possible | YES | possible |
| Animated textures | basic | YES | YES | YES | YES | YES |
| Level scripts | basic | YES | YES | YES | YES | YES |
| Camera scripting | no | no | possible | no | possible | possible |
| Dynamic limits | default | likely raised | likely raised | default | likely raised | likely raised |

## Compatibility Requirements for the Rust Engine

### Must-Have (Phase 1)
1. **Shapes file loading** -- All 32 collections, 8-bit and 16-bit depth
2. **Sounds file loading** -- All sound definitions
3. **Physics model loading** -- Monster, weapon, projectile, effect definitions
4. **Map file loading** -- All polygon types, platforms, lights, media, terminals
5. **Basic MML parsing** -- All 24 sections (already done)
6. **Plugin discovery and metadata** -- Plugin.xml parsing (already done)

### Must-Have (Phase 2)
7. **MML runtime application** -- Convert parsed MML to game state modifications
8. **Shapes patching** -- Tag-based `.ShPa` format parsing and overlay
9. **Sounds patching** -- Sounds patch application
10. **Plugin MML loading** -- Load and layer MML from plugin directories
11. **Level-embedded MML** -- Extract and apply MMLS tags from WAD entries

### Must-Have (Phase 3)
12. **Solo Lua VM** -- Execute gameplay scripts with full API
13. **HUD Lua VM** -- Custom HUD rendering via Lua
14. **OpenGL texture overrides** -- External image files replacing shapes textures
15. **Exclusive resource resolution** -- Write access conflict detection (already done)
16. **Scenario requirement matching** -- Plugin filtering by scenario ID

### Nice-to-Have (Phase 4)
17. **Stats Lua** -- Network game statistics
18. **Map patches** -- Checksum-based resource injection
19. **Zip plugin support** -- Read plugins from zip archives
20. **3D model support** -- MML `<model>` definitions
21. **Custom shader support** -- MML `<shader>` definitions
22. **Camera scripting** -- Lua camera path system

## Content Distribution and Discovery

### Simplici7y (simplici7y.com)
The primary community download site. Tags include: scenario, plugin, lua, shapes, textures, physics, sounds, m1a1, durandal, infinity, solo, net, script, utility, replacement.

### Official Scenarios Page (alephone.lhowon.org)
Hosts the trilogy and major community scenarios with direct download links.

### ModDB
Some larger total conversions (Apotheosis X, Istoria) have ModDB pages for broader visibility.

### Fileball (fileball.whpress.com)
Historical archive of Marathon modding content including MML patches.

## Testing Strategy

For verifying Rust engine compatibility:

1. **Smoke test with Marathon Trilogy** -- Ensure all three games load and play correctly
2. **MML stress test with Rubicon X** -- Heavily customized game data
3. **Lua stress test with Istoria** -- Complex RPG Lua scripting
4. **Visual stress test with Apotheosis X** -- Advanced sprite/texture work
5. **Texture pipeline test with Tempus Irae Redux** -- Extensive OpenGL texture overrides
6. **Scale test with Yuge** -- 255 levels, tests resource management

## Related Notes

- [[mml-override-system]] -- MML is the primary configuration mechanism for all scenarios
- [[lua-vm-integration]] -- Lua scripting enables the most advanced scenario features
- [[plugin-system-patching]] -- Plugin infrastructure that delivers scenario content
- [[shapes-file-patching]] -- Graphics customization mechanism
