---
tags: [tier-3, content-pipeline, plugins, patching, resource-management]
status: research-complete
---

# Plugin System and Patching

## Overview

Aleph One plugins are self-contained directories containing a `Plugin.xml` manifest and associated resource files (MML scripts, Lua scripts, shapes patches, sounds patches, map patches, and theme assets). The plugin system provides a structured way for the community to extend and modify the game without replacing core data files.

## Plugin Directory Structure

```
My Sample Plugin/
  Plugin.xml              -- Manifest (required)
  config.mml              -- MML override file
  Scripts/
    hud.lua               -- HUD Lua script
    solo.lua              -- Solo Lua script
    stats.lua             -- Statistics script
  Patches/
    hd_textures.ShPa      -- Shapes patch file
    new_sounds.sndA        -- Sounds patch file
  Resources/
    image.png             -- Referenced by MML or Lua
```

Plugins are installed into the `Plugins/` directory alongside game data files. They can also be installed globally in the user data directory's `Plugins/` folder to work across all Aleph One games.

**Zip support:** Plugins can remain as zip files for distribution; Aleph One can read them directly, though unzipped plugins load faster.

## Plugin.xml Format -- Complete Reference

### Root Element: `<plugin>`

| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | string | Yes | Plugin identifier (used for sorting and display) |
| `description` | string | No | Short description |
| `version` | string | No | Display-only version |
| `minimum_version` | string (YYYYMMDD) | No | Minimum Aleph One engine date; plugin disabled if not met |
| `auto_enable` | boolean | No | Default: true. If false, plugin starts disabled |
| `theme_dir` | string | No | Theme directory path (activates theme mode) |
| `hud_lua` | string | No | Path to HUD Lua script (legacy attribute form) |
| `solo_lua` | string | No | Path to solo Lua script (legacy attribute form) |
| `stats_lua` | string | No | Path to stats Lua script |

### Child Elements

#### `<mml>` -- MML Override File
```xml
<mml file="config.mml"/>
```
Multiple `<mml>` elements allowed. Files loaded in sorted (alphabetical) order.

#### `<solo_lua>` -- Solo Lua Script (Element Form)
```xml
<solo_lua file="Scripts/solo.lua">
  <write_access>fog</write_access>
  <write_access>music</write_access>
</solo_lua>
```
- Only one `<solo_lua>` element allowed per plugin (if multiple present, solo_lua is cleared/disabled)
- Element form overrides the legacy `solo_lua` attribute
- If no `<write_access>` children, defaults to `world` access
- Valid write_access values: `world`, `fog`, `music`, `overlays`, `ephemera`, `sound`

#### `<shapes_patch>` -- Shapes Patch File
```xml
<shapes_patch file="Patches/hd_textures.ShPa" requires_opengl="true"/>
```
- Multiple allowed
- `requires_opengl`: if true, patch not loaded when software renderer is active

#### `<sounds_patch>` -- Sounds Patch File
```xml
<sounds_patch file="Patches/new_sounds.sndA"/>
```
Multiple allowed.

#### `<scenario>` -- Scenario Requirement
```xml
<scenario name="Marathon Infinity" id="minf" version="1.0"/>
```
- Multiple allowed (any match satisfies)
- Plugin only loads when playing a matching scenario
- `name` (max 31 chars), `id` (max 23 chars), `version` (max 7 chars) must match the scenario's MML `<scenario>` section
- If neither `name` nor `id` is present, the requirement is skipped

#### `<map_patch>` -- Map-Specific Resource Patch
```xml
<map_patch>
  <checksum>12345</checksum>
  <checksum>67890</checksum>
  <resource type="snd " id="100" data="sounds/custom.rsrc"/>
</map_patch>
```
- Requires at least one `<checksum>` and one `<resource>` to be valid
- `type`: 4-character resource type code (e.g., `snd `, `PICT`)
- `id`: Resource ID (integer)
- `data`: Path to replacement resource file within plugin directory
- Only applied when playing a map whose checksum matches

## Plugin Loading Order

### Discovery
1. Scan the `Plugins/` directory recursively for directories containing `Plugin.xml`
2. Skip dot-prefixed directories (`.hidden/`)
3. Parse each `Plugin.xml`
4. Validate file references (remove entries pointing to nonexistent files)
5. Sort plugins **alphabetically by name**

### MML Application (Additive)
All enabled plugins' MML files are processed in alphabetical order by plugin name. Within a plugin, MML files are processed in alphabetical order by filename. MML overrides are additive -- all plugins' MML is applied, with later entries overriding earlier ones for the same setting.

### Exclusive Resource Resolution
Some resources are **exclusive** -- only one plugin can provide them. Resolution follows a "last wins" rule, iterating plugins in reverse alphabetical order:

| Resource Type | Exclusivity | Resolution |
|---------------|-------------|------------|
| HUD Lua | Exclusive | Last enabled plugin with `hud_lua` wins |
| Stats Lua | Exclusive | Last enabled plugin with `stats_lua` wins |
| Theme | Exclusive | Last enabled plugin with `theme_dir` wins |
| Solo Lua | Conditional | Based on write access flags (see below) |
| MML files | Additive | All enabled plugins' MML applied |
| Shapes patches | Additive | All enabled plugins' patches applied in order |
| Sounds patches | Additive | All enabled plugins' patches applied in order |
| Map patches | Conditional | Applied only when map checksum matches |

### Solo Lua Exclusivity Rules
Solo Lua scripts are exclusive based on their write access flags:

1. **`world` access** implies exclusive access to `world`, `fog`, `music`, and `overlays`
2. Scripts conflict if their exclusive access flags overlap
3. The last plugin (alphabetically) wins when conflicts occur
4. Scripts with only `ephemera` and/or `sound` access never conflict

Example: A `fog` plugin and a `music` plugin can coexist. But a `world` plugin prevents all others from running (since `world` claims `fog`, `music`, and `overlays`).

### Theme Directory Behavior
When `theme_dir` is set on a plugin:
- `hud_lua`, `solo_lua`, `shapes_patches`, `sounds_patches`, and `map_patches` are **all cleared**
- The plugin acts purely as a theme provider

## Shapes Patch Application

Shapes patches use a **tag-based binary format** (not raw shapes file format). The patch file contains sequential records, each identified by four-character tags:

| Tag | Content |
|-----|---------|
| `CLDF` | Collection definition replacement |
| `HLSH` | High-level shape (animation sequence) replacement |
| `LLSH` | Low-level shape (frame metadata) replacement |
| `BMAP` | Bitmap image data replacement |
| `CTAB` | Color table (palette) replacement |
| `ENDC` | End-of-collection marker |

Each record specifies a collection index and bit-depth, then the replacement data. Patches are applied after all standard collections load. Modified bitmaps receive a `_PATCHED_BIT` flag for renderer identification.

See [[shapes-file-patching]] for full details.

## Sounds Patch Application

Sounds patches follow the same principle as shapes patches -- they provide replacement or additional sound data for specific sound indices. The patch file uses the same binary format as the main Sounds file (`.sndA`).

## Map Patch Application

Map patches provide resource replacements that are only applied when the current map's checksum matches one of the listed checksums. This allows plugins to fix or modify specific maps without affecting others. Resources are identified by their four-character type code and numeric ID.

## Current State in Rust Rebuild

**Parser location:** `marathon-formats/src/plugin.rs`

**What the Rust code handles:**
- `PluginMetadata` struct with all Plugin.xml fields
- `PluginMetadata::from_bytes()` / `from_file()` -- Parse Plugin.xml
- `PluginMetadata::validate_references()` -- Remove entries for nonexistent files
- `discover_plugins()` -- Recursive directory scanning with dot-prefix filtering
- `sort_plugins()` -- Alphabetical sorting by name
- `resolve_exclusive_resources()` -- Last-wins resolution for HUD Lua, Stats Lua, Theme, and Solo Lua (with write access conflict detection)
- `SoloLuaWriteAccess` bitflags -- WORLD, FOG, MUSIC, OVERLAYS, EPHEMERA, SOUND
- `ShapesPatch` struct with file path and requires_opengl flag
- `MapPatch` struct with checksums and resources
- `ScenarioRequirement` with name, id, version (truncation enforced)
- Theme directory behavior (clears all other resources)
- Handles both attribute and element forms of solo_lua
- Multiple solo_lua elements detection (clears solo_lua)

**What is missing:**
- No MML file loading from plugin directories
- No Lua script loading from plugin directories
- No shapes patch application (binary format parsing and overlay)
- No sounds patch application
- No map patch application (checksum matching and resource injection)
- No scenario requirement checking (matching against active scenario)
- No plugin enable/disable state management (preferences)
- No zip file reading for packaged plugins
- No minimum_version checking against engine version

## Gaps and Implementation Plan

### Phase 1: Plugin Loading Pipeline
Create a `PluginLoader` that:
1. Discovers plugins via `discover_plugins()` (already done)
2. Checks `minimum_version` against engine version
3. Matches `required_scenarios` against active scenario
4. Filters by user enable/disable preferences
5. Resolves exclusive resources (already done)
6. Returns a finalized list of active plugins with their resolved resources

### Phase 2: MML Integration
For each active plugin, load and parse its MML files:
1. Read MML files from plugin directory in alphabetical order
2. Parse each with `MmlDocument::from_file()`
3. Layer all plugin MML documents in plugin order using `MmlDocument::layer()`
4. Apply the final layered MML to game state

### Phase 3: Shapes and Sounds Patch Application
- Parse the tag-based shapes patch binary format
- Implement collection-level overlay: replace individual collections, shapes, bitmaps, or color tables
- Parse sounds patch files and apply audio replacements

### Phase 4: Map Patch Application
- Compute map file checksums
- Match against `MapPatch.checksums`
- Inject replacement resources into the resource loading pipeline

### Phase 5: Lua Script Loading
Wire into [[lua-vm-integration]]:
- Load solo_lua from the winning plugin (after exclusivity resolution)
- Load hud_lua from the winning plugin
- Load stats_lua from the winning plugin
- Pass write_access flags to the Lua VM mutability interface

### Phase 6: Zip Support
- Add zip file reading (via `zip` crate) for packaged plugins
- Read Plugin.xml and all referenced files from within zip archives

## Recommended Rust Crates

- `quick-xml` (already used) -- Plugin.xml parsing
- `zip` -- Zip archive reading for packaged plugins
- `walkdir` -- Recursive directory scanning (alternative to manual recursion)
- `semver` or custom date parsing -- For minimum_version checking
- `crc32fast` -- For map checksum computation

## Related Notes

- [[mml-override-system]] -- Plugins deliver MML overrides
- [[lua-vm-integration]] -- Plugins declare and deliver Lua scripts
- [[shapes-file-patching]] -- Detailed shapes patch binary format
- [[community-content-ecosystem]] -- Real-world plugin usage patterns
