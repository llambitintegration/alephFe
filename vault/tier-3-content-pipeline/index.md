---
tags: [tier-3, content-pipeline, index]
status: research-complete
---

# Tier 3: Content Pipeline and Modding

This tier covers the systems that allow Marathon content creators (scenario designers, modders, plugin authors) to customize and extend the engine. Together, these systems form the backbone of Marathon's thriving modding community, which has produced content continuously since the late 1990s.

## Notes

### [[mml-override-system]]
The Marathon Markup Language -- an XML-based configuration system with 24+ sections covering every aspect of game data. MML is the primary mechanism for overriding monster stats, weapon properties, texture assignments, HUD layout, rendering settings, string tables, and more. The Rust parser exists but runtime application is not yet implemented.

### [[lua-vm-integration]]
Aleph One's Lua scripting system with three script types (solo gameplay, HUD rendering, statistics). The solo API exposes 20+ triggers and 15+ global tables covering players, monsters, projectiles, level geometry, cameras, and more. The HUD API provides drawing primitives for custom interface rendering. No Lua VM exists in the Rust codebase yet.

### [[lua-in-rust-options]]
Detailed comparison of all Lua-in-Rust options (mlua, Piccolo, lua-rs, Rhai, silt-lua, Wasmoon, Fengari, and more) evaluated for dual-target support (native + wasm32-unknown-unknown). Includes compatibility tables, stdlib coverage analysis, and performance notes.

### [[lua-wasm-architecture]]
Recommended architecture for supporting Lua scripting on both native and browser WASM targets. Primary recommendation: lua-rs (pure Rust Lua 5.5) as a unified VM. Fallback strategies: mlua+Piccolo split, mlua+Wasmoon JS bridge, or web-only deferral.

### [[plugin-system-patching]]
The plugin system that packages MML overrides, Lua scripts, shapes patches, sounds patches, and map patches into distributable directories with Plugin.xml manifests. Plugin discovery, metadata parsing, sorting, and exclusive resource resolution are implemented in Rust. Patch application and script loading are not.

### [[shapes-file-patching]]
How shapes (sprite/texture) data gets modified by plugins using a tag-based binary patch format. Covers collection replacement, individual bitmap replacement, color table overrides, and animation sequence modification. The Rust shapes parser handles full file loading but has no patch support.

### [[community-content-ecosystem]]
Survey of the Marathon modding community: major total conversions (Rubicon X, Eternal X, Phoenix, Apotheosis X, Istoria, Tempus Irae Redux), what engine features they depend on, and a prioritized compatibility matrix for the Rust rebuild.

## Architecture Overview

```
Content Loading Pipeline:

  Engine Defaults
       |
  Global MML Scripts (alphabetical)
       |
  Local MML Scripts (alphabetical)
       |
  Scenario MML (from map file)
       |
  Plugin Discovery & Resolution
       |
  +-- Plugin MML (alphabetical by plugin, then by file)
  +-- Plugin Shapes Patches (additive)
  +-- Plugin Sounds Patches (additive)
  +-- Plugin Solo Lua (exclusive, write access rules)
  +-- Plugin HUD Lua (exclusive, last wins)
  +-- Plugin Stats Lua (exclusive, last wins)
       |
  Level-Embedded MML (MMLS WAD tag)
       |
  Level Scripts (marathon_levels TEXT resource)
       |
  Map Patches (checksum-matched resource injection)
```

## Implementation Priority

### Phase 1 -- Parse Everything (Mostly Done)
- [x] MML parsing (all 24 sections)
- [x] Plugin.xml parsing (all attributes and elements)
- [x] Shapes file parsing (all collection types)
- [x] Plugin discovery and sorting
- [x] Exclusive resource resolution
- [ ] Shapes patch binary format parsing
- [ ] Level script (`<marathon_levels>`) parsing

### Phase 2 -- Apply Overrides
- [ ] MML typed section interpreters
- [ ] MML element-level merging (not just section-level)
- [ ] MML runtime application to game state
- [ ] Shapes patch application (collection overlay)
- [ ] Sounds patch application
- [ ] Plugin MML loading pipeline
- [ ] Scenario requirement matching

### Phase 3 -- Lua Integration
- [ ] Lua VM evaluation sprint (lua-rs vs mlua; see [[lua-wasm-architecture]])
- [ ] Mnemonic registry (string-to-ID mapping for all game constants)
- [ ] Game object UserData (Players, Monsters, Projectiles, etc.)
- [ ] Trigger dispatch system
- [ ] Solo Lua API (20+ triggers, 15+ global tables)
- [ ] Write access enforcement
- [ ] Script lifecycle management

### Phase 4 -- HUD and Advanced Features
- [ ] HUD Lua API (Screen, Fonts, Images, Shapes drawing)
- [ ] OpenGL texture overrides (external image files from MML)
- [ ] Camera scripting system
- [ ] Stats Lua
- [ ] Map patches (checksum matching)
- [ ] Zip plugin support
- [ ] 3D model and custom shader support

## Key Rust Crate Dependencies

| Crate | Purpose | Status |
|-------|---------|--------|
| `quick-xml` | MML and Plugin.xml parsing | In use |
| `binrw` | Shapes binary format parsing | In use |
| `luars` or `mlua` | Lua VM embedding (see [[lua-wasm-architecture]]) | Not yet added |
| `serde` | Typed MML deserialization, Lua state serialization | Partially in use |
| `zip` | Packaged plugin reading | Not yet added |
| `image` | External texture loading for OpenGL overrides | Not yet added |
| `walkdir` | Recursive directory scanning | Not yet added |

## Cross-References

- Tier 1 (Game Loop) -- MML `<dynamic_limits>` affects entity pools; Lua `idle()`/`postidle()` hooks into the tick cycle
- Tier 2 (Visual/Audio) -- MML `<opengl>`, `<software>`, `<faders>`, `<landscapes>` configure rendering; shapes patches modify texture data; HUD Lua replaces the built-in HUD
- Tier 4 (Multiplayer) -- Stats Lua and network scripts operate in multiplayer; solo Lua write access prevents conflicts
