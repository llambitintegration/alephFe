---
tags: [tier-2, index, rendering, visual, audio]
status: research-complete
---

# Tier 2: Visual and Audio Polish

This tier covers the visual rendering effects and audio systems that transform the basic 3D geometry into the Marathon experience. These features sit between the core game loop (Tier 1) and the content pipeline (Tier 3).

## Research Notes

### Rendering Effects

| Topic | Note | Priority | Complexity |
|-------|------|----------|------------|
| Liquid Surfaces | [[liquid-surface-rendering]] | High | Medium |
| Visual Effects (VFX) | [[visual-effects-vfx]] | High | High |
| Dynamic Lighting | [[dynamic-lighting]] | High | Medium |
| Transfer Modes & Glow | [[glow-transfer-modes]] | Medium | Medium |
| Overhead Map / Automap | [[overhead-map-automap]] | Medium | Low-Medium |
| Infravision Mode | [[infravision-mode]] | Low | Low |

### Dependency Graph

```
dynamic-lighting
    |
    v
liquid-surface-rendering  <-->  visual-effects-vfx
    |                               |
    v                               v
glow-transfer-modes  <-------->  infravision-mode
    |
    v
overhead-map-automap
```

**Dynamic lighting** is foundational -- lights drive media height, polygon brightness, and interact with every other visual system.

**Liquid surfaces** and **VFX** are tightly coupled through media detonation effects (splashes when projectiles hit water/lava).

**Transfer modes** affect all surface rendering including media, walls, and floors.

**Infravision** overrides the lighting and transfer mode systems.

**Automap** is largely independent but uses media type info for polygon coloring.

## Current Codebase Summary

### Rendering Pipeline Architecture

The Rust rebuild has three rendering targets:

| Crate | Renderer | GPU API | Status |
|-------|----------|---------|--------|
| marathon-viewer | Desktop viewer | wgpu (native) | Storage buffers, polygon data |
| marathon-game | Desktop game | wgpu (native) | Storage buffers, sprite rendering |
| marathon-web | Browser game | wgpu (WebGL2) | Uniform buffers, sprites, basic automap |

All three share a similar shader structure:
- Camera uniform (view_proj, yaw, pitch, elapsed_time)
- Per-polygon data (floor/ceiling height, light, transfer mode, media)
- Texture arrays per collection
- Billboarded sprite rendering for entities

### Key Source Files

**Formats & Parsing:**
- `/home/llambit/0_repos/alephone-rust/marathon-formats/src/map.rs` - Map data including polygons, media, sides, lights
- `/home/llambit/0_repos/alephone-rust/marathon-formats/src/physics.rs` - Effect, projectile, weapon definitions
- `/home/llambit/0_repos/alephone-rust/marathon-formats/src/shapes.rs` - Shape collections, transfer modes per shape

**Simulation:**
- `/home/llambit/0_repos/alephone-rust/marathon-sim/src/world_mechanics/lights.rs` - Light intensity computation (4 of 6 functions)
- `/home/llambit/0_repos/alephone-rust/marathon-sim/src/world_mechanics/media.rs` - Media height, damage, drag
- `/home/llambit/0_repos/alephone-rust/marathon-sim/src/combat/projectiles.rs` - Projectile physics (no contrails yet)

**Rendering:**
- `/home/llambit/0_repos/alephone-rust/marathon-viewer/src/render.rs` - Desktop viewer renderer
- `/home/llambit/0_repos/alephone-rust/marathon-viewer/src/shader.wgsl` - Main WGSL shader (5 transfer modes)
- `/home/llambit/0_repos/alephone-rust/marathon-viewer/src/transfer.rs` - Transfer mode constants (6 defined)
- `/home/llambit/0_repos/alephone-rust/marathon-game/src/shader.wgsl` - Game shader (same as viewer)
- `/home/llambit/0_repos/alephone-rust/marathon-game/src/sprite_shader.wgsl` - Sprite billboard shader
- `/home/llambit/0_repos/alephone-rust/marathon-web/src/render.rs` - Web renderer with automap

**Integration:**
- `/home/llambit/0_repos/alephone-rust/marathon-integration/src/sprites/mod.rs` - Sprite bridge (entity to render command)

## Recommended Implementation Order

1. **Fix transfer mode enum values** ([[glow-transfer-modes]]) -- currently wrong, affects all rendering
2. **Per-tick light animation** ([[dynamic-lighting]]) -- foundational for media and atmosphere
3. **Media surface geometry + transparency** ([[liquid-surface-rendering]]) -- high visual impact
4. **Effect entity system** ([[visual-effects-vfx]]) -- needed for combat feedback
5. **Enhanced automap** ([[overhead-map-automap]]) -- gameplay utility, relatively simple
6. **Glow/self-luminous two-pass** ([[glow-transfer-modes]]) -- visual fidelity
7. **Infravision** ([[infravision-mode]]) -- niche feature, implement last

## Alephone Source References

Primary C++ source files consulted:

- [map.h](https://github.com/Aleph-One-Marathon/alephone/blob/master/Source_Files/GameWorld/map.h) - Transfer mode enums, polygon structures
- [media.h](https://github.com/Aleph-One-Marathon/alephone/blob/master/Source_Files/GameWorld/media.h) / [media.cpp](https://github.com/Aleph-One-Marathon/alephone/blob/master/Source_Files/GameWorld/media.cpp) - Media system
- [media_definitions.h](https://github.com/Aleph-One-Marathon/alephone/blob/master/Source_Files/GameWorld/media_definitions.h) - Per-media-type properties
- [effects.h](https://github.com/Aleph-One-Marathon/alephone/blob/master/Source_Files/GameWorld/effects.h) / [effects.cpp](https://github.com/Aleph-One-Marathon/alephone/blob/master/Source_Files/GameWorld/effects.cpp) - Effect lifecycle
- [effect_definitions.h](https://github.com/Aleph-One-Marathon/alephone/blob/master/Source_Files/GameWorld/effect_definitions.h) - 79 effect type definitions
- [lightsource.h](https://github.com/Aleph-One-Marathon/alephone/blob/master/Source_Files/GameWorld/lightsource.h) / [lightsource.cpp](https://github.com/Aleph-One-Marathon/alephone/blob/master/Source_Files/GameWorld/lightsource.cpp) - Light animation system
- [overhead_map.h](https://github.com/Aleph-One-Marathon/alephone/blob/master/Source_Files/RenderOther/overhead_map.h) / [overhead_map.cpp](https://github.com/Aleph-One-Marathon/alephone/blob/master/Source_Files/RenderOther/overhead_map.cpp) - Automap config
- [OverheadMapRenderer.cpp](https://github.com/Aleph-One-Marathon/alephone/blob/master/Source_Files/RenderOther/OverheadMapRenderer.cpp) - Automap rendering
- [RenderRasterize.cpp](https://github.com/Aleph-One-Marathon/alephone/blob/master/Source_Files/RenderMain/RenderRasterize.cpp) - Transfer mode application, render order
- [OGL_Render.cpp](https://github.com/Aleph-One-Marathon/alephone/blob/master/Source_Files/RenderMain/OGL_Render.cpp) - OpenGL: glow, infravision, fog, media
- [OGL_Textures.h](https://github.com/Aleph-One-Marathon/alephone/blob/master/Source_Files/RenderMain/OGL_Textures.h) - TextureManager, glow mapping
- [scottish_textures.h](https://github.com/Aleph-One-Marathon/alephone/blob/master/Source_Files/RenderMain/scottish_textures.h) - Low-level transfer modes
- [MML docs](http://tst2005.github.io/alephone-doc/docs/MML.html) - MML configuration for overrides
