---
tags: [tier-3, content-pipeline, mml, configuration, xml]
status: research-complete
---

# MML (Marathon Markup Language) Override System

## Overview

MML is a subset of XML created by Loren Petrich for configuring the Marathon engine in a non-hardcoded way. It allows scenarios, plugins, and users to override virtually every aspect of game data: monster stats, weapon behavior, texture assignments, string tables, HUD layout, rendering settings, and more. MML is the primary mechanism through which Marathon content creators customize the engine without modifying source code.

## Root Structure

Every MML document has a single root element `<marathon>`. Child elements represent sections, each governing a different aspect of the engine. Boolean values accept `1`, `t`, `true` for true and `0`, `f`, `false` for false.

## Complete Section Reference

### 1. `<stringset>` -- String Table Overrides

Mimics MacOS 'STR#' resources. Each stringset is identified by `index` (resource ID 128-149).

```xml
<stringset index="128">
  <string index="0">Custom error message</string>
</stringset>
```

**Resource IDs:**
| Index | Content |
|-------|---------|
| 128 | Error messages |
| 129 | Filenames |
| 130 | Top-level interface items |
| 131 | Prompts |
| 132 | Network errors |
| 133 | Key codes |
| 134 | Preferences advice |
| 135 | Computer interface strings |
| 136 | Join dialog |
| 137 | Weapon names |
| 138 | File search paths |
| 139 | Preference groupings |
| 140 | Network statistics |
| 141 | Network game setup |
| 142 | New join dialog |
| 143 | Network progress |
| 144 | Difficulty names |
| 145 | Item names |
| 146 | Item types |
| 147 | Color names |
| 148 | Network statistics (alt) |
| 149 | OpenGL color prompts |

### 2. `<interface>` -- HUD Layout

Controls HUD element positioning (coordinates: x=rightward, y=downward, origin top-left 0,320).

**Attributes:** `motion_sensor` (boolean, default true)

**Child elements:**
- `<rect index="0..30" top="" left="" bottom="" right="">` -- 31 HUD rectangles (player name, oxygen bar, shield bar, weapon display, ammo counters, etc.)
- `<color index="0..25" red="0-1" green="0-1" blue="0-1">` -- HUD colors
- `<font index="0..6" name="" size="" style="" file="">` -- Fonts (0=Interface, 1=Weapon Name, 2=Player Name, 3=Item Count, 4=Terminal, 5=Terminal Title, 6=Network Stats)
- `<weapon index="0..9">` with shape, position, ammo display children
- `<vidmaster stringset_index="">` -- Vidmaster oath dialog

### 3. `<motion_sensor>` -- Motion Sensor Configuration

**Child elements:**
- `<assign monster="type" type="0-2">` -- Monster-to-blip mapping (0=Self/Friendly/Bob, 1=Alien, 2=Hostile Player)

### 4. `<overhead_map>` -- Overhead Map

**Attributes:** `mode` (0=Cumulative, 1=Visible only, 2=All), `title_offset` (pixels)

**Child elements:**
- `<assign_live monster="" type="-1,0,1">` -- Live monster display
- `<assign_dead coll="" type="-1,0,1">` -- Dead monster display
- `<aliens on="">`, `<items on="">`, `<projectiles on="">`, `<paths on="">` -- Toggle visibility
- `<line type="0-2" scale="0-3" width="">` -- Map line styles (0=Wall, 1=Elevation, 2=Control panel)
- `<color index="0..21" red="" green="" blue="">` -- 22 map colors
- `<font index="0-4" name="" size="" style="">` -- Map fonts

### 5. `<infravision>` -- Infravision Colors

**Child elements:**
- `<assign coll="" color="0-3">` -- Collection-to-color mapping
- `<color index="0-3" red="" green="" blue="">` -- (0=Aliens/Red, 1=Friends/Green, 2=Walls/Blue, 3=Player/Yellow)

### 6. `<animated_textures>` -- Animated Texture Sequences

**Child elements:**
- `<clear coll="">` -- Reset sequences for collection
- `<sequence coll="" numticks="" framephase="" tickphase="" select="">` with `<frame index="">` children

### 7. `<control_panels>` -- Control Panel Configuration

**Attributes:** `reach` (float, default 1.5 WU), `horiz` (float, default 2), `single_energy`/`rate`, `double_energy`/`rate`, `triple_energy`/`rate`

**Child elements:**
- `<panel index="0..53" type="0-8" coll="" active_frame="" inactive_frame="" pitch="" item="">` with `<sound type="0-2" which="">` children
- Panel types: 0=Oxygen, 1=Energy 1x, 2=Energy 2x, 3=Energy 3x, 4=Light Switch, 5=Platform Switch, 6=Tag Switch, 7=Pattern Buffer, 8=Computer Terminal

### 8. `<platforms>` -- Platform Configuration

**Child elements:**
- `<platform index="0-8" start_extend="" start_contract="" stop_extend="" stop_contract="" obstructed="" uncontrollable="" moving="" item="">` with optional `<damage>` child
- Types: S'pht Door, S'pht Split Door, Locked S'pht Door, S'pht Platform, Noisy S'pht Platform, Heavy S'pht Door, Pfhor Door, Heavy S'pht Platform, Pfhor Platform

### 9. `<liquids>` -- Liquid/Media Configuration

**Child elements:**
- `<liquid index="0-4" coll="" frame="" transfer="0-21" damage_freq="" submerged="0-4">` with `<damage>`, `<effect type="0-3" which="">`, `<sound type="0-8" which="">` children
- Types: 0=Water, 1=Lava, 2=Pfhor (Goo), 3=Sewage, 4=Jjaro

### 10. `<sounds>` -- Sound Configuration

**Attributes:** 17 named sound indices (terminal_logon, terminal_logoff, terminal_page, teleport_in, teleport_out, got_powerup, got_item, crunched, exploding, breathing, oxygen_warning, adjust_volume, button_success, button_failure, button_inoperative, ogl_reset, center_button)

**Child elements:**
- `<ambient index="0-27" sound="">` -- 28 ambient sound types
- `<random index="0-4" sound="">` -- 5 random sound types
- `<dialog index="0-7" sound="">` -- 8 dialog sounds
- `<sound index="" slot="0-4" file="">` -- External sound file mapping
- `<sound_clear>` -- Clear external sound definitions

### 11. `<faders>` -- Screen Fade Effects

**Child elements:**
- `<fader index="" type="0-5" initial_opacity="0-1" final_opacity="0-1" period="" flags="0-3" priority="">` with `<color>` child
- Types: 0=Tint, 1=Randomize, 2=Negate, 3=Dodge, 4=Burn, 5=Soft Tint
- `<liquid index="0-4" fader="" opacity="0-1">` -- Per-liquid fader

### 12. `<player>` -- Player Configuration

**Attributes:** `energy` (150), `oxygen` (10800), `stripped`, `light` (0.5), `oxygen_deplete` (1), `oxygen_replenish` (0), `vulnerability` (9), `guided` (false), `half_visual_arc` (42), `half_vertical_visual_arc` (42), `visual_range` (31), `dark_visual_range` (31), `single_energy` (150), `double_energy` (300), `triple_energy` (450), `can_swim` (true)

**Child elements:**
- `<item index="" type="">` -- Starting inventory
- `<damage index="" threshold="" fade="" sound="" death_sound="" death_action="6-8">`
- `<powerup invisibility="" invincibility="" extravision="" infravision="">` -- Duration in ticks
- `<powerup_assign>` -- Item ID to powerup mapping
- `<shape type="0-4" subtype="" value="">` -- Player shapes (Collection/Death, Leg, Idle/Charging/Firing Torso)

### 13. `<view>` -- View/Camera Configuration

**Attributes:** `map` (true), `fold_effect` (true), `static_effect` (true), `interlevel_in_effects` (true), `interlevel_out_effects` (true)

**Child elements:**
- `<font name="" size="" style="">` -- OSD font
- `<fov normal="" extra="" tunnel="" rate="" fix_h_not_v="">` -- Field of view (degrees 0-180)

### 14. `<weapons>` -- Weapon Configuration

**Child elements:**
- `<shell_casings index="0-4" coll="" seq="" x0="" y0="" vx0="" vy0="" dvx="" dvy="">` -- Shell casing effects
- `<order index="0..9" weapon="0-9">` -- Weapon cycling order

### 15. `<items>` -- Item Configuration

**Child elements:**
- `<item index="" type="0-5" singular="" plural="" maximum="" invalid="">` with `<shape>` child
- Types: 0=Weapon, 1=Ammunition, 2=Powerup, 3=Generic, 4=Weapon Powerup, 5=Ball

### 16. `<monsters>` -- Monster Configuration

**Child elements:**
- `<monster index="0-46" must_be_exterminated="">` -- Monster attributes

### 17. `<scenery>` -- Scenery Objects

**Child elements:**
- `<object index="0-60" flags="0-7" radius="" height="" destruction="">` with `<normal>` and `<destroyed>` shape children
- Flags: 1=Solid, 4=Destroyable

### 18. `<landscapes>` -- Landscape Rendering

**Child elements:**
- `<clear coll="">` -- Reset to defaults
- `<landscape coll="" frame="" horiz_exp="" vert_exp="" ogl_asprat_exp="" vert_repeat="" azimuth="">`

### 19. `<texture_loading>` -- Texture Environment

**Attributes:** `landscapes` (boolean)

**Child elements:**
- `<texture_env index="0-4" which="0-6" coll="-1 or 0-31">`

### 20. `<opengl>` -- OpenGL Rendering

**Child elements:**
- `<txtr_clear coll="">` -- Reset texture options
- `<texture coll="" clut="" bitmap="" opac_type="0-3" ...>` -- Hi-res texture replacement (with normal_image, glow_image, offset_image file paths)
- `<model_clear>` / `<model coll="" seq="" file="" scale="" ...>` -- 3D model replacement with `<seq_map>` and `<skin>` children
- `<shader name="" vert="" frag="" passes="">` -- Custom GLSL shaders
- `<fog type="0-1" on="" depth="" landscapes="">` with `<color>` child

### 21. `<software>` -- Software Rendering

**Child elements:**
- `<texture coll="" bitmap="" opac_type="" opac_scale="" opac_shift="">`

### 22. `<dynamic_limits>` -- Entity Limits

**Child elements:** `<objects>` (1024), `<monsters>` (512), `<paths>` (128), `<projectiles>` (128), `<effects>` (128), `<rendered>` (1024), `<local_collision>` (64), `<global_collision>` (256)

### 23. `<scenario>` -- Scenario Identification

**Attributes:** `name` (max 32 chars), `version` (max 8 chars), `id` (max 24 chars)

**Child elements:** `<can_join>scenario_id</can_join>` -- Compatible scenario IDs

### 24. `<console>` -- Console Configuration

**Attributes:** `lua` (boolean) -- Enable Lua interpreter in solo play

**Child elements:**
- `<macro input="" output="">` -- Console macros
- `<carnage_message projectile_type="" on_kill="" on_suicide="">` -- Kill messages (supports %aggressor%, %player% placeholders)

### 25. `<logging>` -- Logging Configuration

**Child elements:**
- `<logging_domain domain="" threshhold="" show_locations="" flush="">` -- Log levels: 0=Fatal through 6=Dump

## Override Cascade Order

MML overrides are applied in this order (later overrides win):

1. **Engine defaults** -- Hardcoded values
2. **Global MML scripts** -- From `MML/` subdirectory of global data directories, alphabetical order
3. **Local MML scripts** -- From `Scripts/` subdirectory of local data directory, alphabetical order
4. **Scenario MML** -- Embedded in the scenario's map file or scenario-specific scripts
5. **Plugin MML** -- From enabled plugins, in alphabetical order by plugin name
6. **Level-embedded MML** -- MMLS tag in WAD entry for current level
7. **Per-level scripts** -- Via `<marathon_levels>` in TEXT resource 128

Any value set by a later source completely replaces the value from an earlier source. Within a section, individual elements are typically replaced by index (e.g., monster 5's settings from a plugin replace monster 5's settings from the scenario).

## Level Scripting (`<marathon_levels>`)

A separate root element for per-level configuration, stored in TEXT resource 128 of the map file.

```xml
<marathon_levels>
  <level index="0">
    <mml resource="1000"/>
    <music file="level1.ogg"/>
    <movie file="intro.mov"/>
  </level>
  <default>
    <music file="ambient.ogg"/>
  </default>
  <end>
    <end_screens index="1100" count="3"/>
  </end>
  <restore/>
</marathon_levels>
```

## Common Data Types

### Shape Descriptor
```xml
<shape coll="0-31" clut="0-7" seq="0-255" frame="0-255"/>
```

### Damage
```xml
<damage type="" flags="0-1" base="" random="" scale=""/>
```
Flags: 0=Normal, 1=Alien damage

### Color
```xml
<color index="" red="0-1" green="0-1" blue="0-1"/>
```

### Font
```xml
<font index="" name="" size="" style="" file=""/>
```
Style is a bitmask: 1=Bold, 2=Italic, 4=Underline, 8=Outline, 16=Shadow, 32=Condense, 64=Extend

## Current State in Rust Rebuild

**Parser location:** `marathon-formats/src/mml.rs`

The Rust code provides:
- `MmlDocument` struct with fields for all 24 recognized sections
- `MmlSection` containing a flat `Vec<MmlElement>` preserving XML tree structure
- `MmlElement` with name, attributes (HashMap), children, and text content
- `MmlDocument::from_bytes()` -- Parse from raw bytes
- `MmlDocument::from_file()` -- Parse from filesystem
- `MmlDocument::from_wad_entry()` -- Extract embedded MMLS from WAD
- `MmlDocument::layer()` -- Merge two documents (overlay sections replace base sections)
- Null-byte stripping for WAD-embedded data

**What works:** Parsing all 24 sections into generic XML trees, section-level layering.

**What is missing:**
- No runtime application of overrides -- the parsed MML tree is not converted into actual game state modifications
- Section-level layering only (entire section replaced) -- no element-level merging within sections (e.g., replacing just monster index 5 while keeping other monsters)
- No interpretation of element attributes (types, ranges, defaults)
- No typed accessors for specific section content (e.g., `get_monster_override(index)`)
- No level script (`<marathon_levels>`) parsing
- No validation of attribute values or ranges

## Gaps and Implementation Plan

### Phase 1: Typed Section Interpreters
Create typed structs for each section's content. For each section, define a Rust struct mirroring the expected elements and attributes, with typed fields (not raw strings). Example:

```rust
pub struct MonsterOverride {
    pub index: usize,
    pub must_be_exterminated: Option<bool>,
}

pub struct WeaponShellCasing {
    pub index: usize,
    pub collection: Option<i16>,
    pub sequence: Option<i16>,
    // ... position/velocity fields
}
```

### Phase 2: Element-Level Merging
Upgrade `MmlDocument::layer()` to merge individual elements within sections by their `index` attribute rather than replacing entire sections. This is critical for plugin compatibility.

### Phase 3: Runtime Application
Create an `MmlApplicator` that takes a typed MML document and modifies game state structures (physics definitions, rendering parameters, HUD layout, etc.).

### Phase 4: Level Script Support
Parse `<marathon_levels>` root element and integrate with level loading pipeline.

## Recommended Rust Crates

- `quick-xml` (already used) -- XML parsing
- `serde` -- For typed deserialization of MML sections
- `thiserror` (already used) -- Error types

## Related Notes

- [[plugin-system-patching]] -- Plugins deliver MML overrides
- [[lua-vm-integration]] -- Lua scripts and MML interact (console lua flag, level scripts)
- [[shapes-file-patching]] -- MML `<opengl>` section references texture replacements
- [[community-content-ecosystem]] -- Scenarios depend heavily on MML customization
