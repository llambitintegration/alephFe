# Spec: plugin-metadata

ADDED capability for parsing Marathon plugin metadata from Plugin.xml files, discovering plugins from directory structures, and determining plugin load order.

## Context

Marathon plugins extend scenarios with additional content: MML configuration overrides, shapes/sounds patches, Lua scripts (HUD, solo, stats), themes, and map patches. Each plugin is a directory containing a `Plugin.xml` file that declares the plugin's identity, its resource references, and its scenario compatibility constraints.

The C++ reference implementation lives in `Plugins.h` and `Plugins.cpp` in Aleph One. Key data structures:

- **Plugin**: name, description, version, directory, mmls (list), hud_lua, solo_lua, solo_lua_write_access, stats_lua, theme, required_version, shapes_patches (list), sounds_patches (list), required_scenarios (list), map_patches (list), auto_enable, enabled
- **ScenarioInfo**: name (max 31 chars), scenario_id (max 23 chars), version (max 7 chars)
- **ShapesPatch**: path, requires_opengl flag
- **MapPatch**: parent_checksums (set of u32), resource_map (map from (tag: u32, id: i32) to file path)
- **SoloLuaWriteAccess**: flags -- world (0x01), fog (0x02), music (0x04), overlays (0x08), ephemera (0x10), sound (0x20)

---

### Requirement: Parse Plugin.xml metadata files

The system MUST parse a Plugin.xml file and extract the plugin's core identity fields from the root `<plugin>` element attributes.

#### Scenario: WHEN a valid Plugin.xml is parsed THEN the plugin name, description, and version are extracted

- WHEN a Plugin.xml contains `<plugin name="My Plugin" version="1.2" description="A cool plugin">`
- THEN the parsed plugin has name "My Plugin", version "1.2", and description "A cool plugin"

#### Scenario: WHEN a Plugin.xml has a minimum_version attribute THEN the required engine version is captured

- WHEN a Plugin.xml contains `<plugin name="P" minimum_version="20230101">`
- THEN the parsed plugin has required_version set to "20230101"

#### Scenario: WHEN a Plugin.xml has an auto_enable attribute THEN the auto-enable flag is captured

- WHEN a Plugin.xml contains `<plugin name="P" auto_enable="false">`
- THEN the parsed plugin has auto_enable set to false

#### Scenario: WHEN auto_enable is not specified THEN it defaults to true

- WHEN a Plugin.xml contains `<plugin name="P">` with no auto_enable attribute
- THEN the parsed plugin has auto_enable set to true

#### Scenario: WHEN the plugin element has no name attribute THEN the plugin is rejected

- WHEN a Plugin.xml contains `<plugin description="no name">`
- THEN the plugin is not added to the plugin list (name is required)

#### Scenario: WHEN a plugin specifies a theme_dir THEN Lua scripts and patches are cleared

- WHEN a Plugin.xml contains `<plugin name="Theme" theme_dir="resources">`
- THEN the parsed plugin has theme set to "resources"
- AND hud_lua, solo_lua, shapes_patches, sounds_patches, and map_patches are all cleared/empty

---

### Requirement: Parse scenario requirements (compatibility checking)

The system MUST parse `<scenario>` child elements that declare which scenarios the plugin is compatible with.

#### Scenario: WHEN a plugin has scenario elements THEN each scenario's name, id, and version are captured

- WHEN a Plugin.xml contains:
  ```xml
  <plugin name="P">
    <scenario name="Marathon Infinity" id="minf" version="1.0"/>
  </plugin>
  ```
- THEN the parsed plugin has one required_scenario with name "Marathon Infinity", scenario_id "minf", and version "1.0"

#### Scenario: WHEN a scenario name exceeds 31 characters THEN it is truncated

- WHEN a Plugin.xml contains a scenario element with a name longer than 31 characters
- THEN the parsed name is truncated to 31 characters

#### Scenario: WHEN a scenario id exceeds 23 characters THEN it is truncated

- WHEN a Plugin.xml contains a scenario element with an id longer than 23 characters
- THEN the parsed scenario_id is truncated to 23 characters

#### Scenario: WHEN a scenario version exceeds 7 characters THEN it is truncated

- WHEN a Plugin.xml contains a scenario element with a version longer than 7 characters
- THEN the parsed version is truncated to 7 characters

#### Scenario: WHEN a scenario element has neither name nor id THEN it is skipped

- WHEN a Plugin.xml contains `<scenario version="1.0"/>` with no name or id
- THEN no required_scenario entry is added for that element

#### Scenario: WHEN a plugin has no scenario elements THEN it is compatible with all scenarios

- WHEN a Plugin.xml contains `<plugin name="P">` with no `<scenario>` children
- THEN the parsed plugin has an empty required_scenarios list
- AND the plugin is considered compatible with any scenario

#### Scenario: WHEN checking compatibility against a scenario THEN partial matches succeed

- WHEN a required_scenario has name "Marathon Infinity" and an empty scenario_id
- AND the current scenario name is "Marathon Infinity" (regardless of its id)
- THEN the plugin is considered compatible

---

### Requirement: Parse resource references (MML, Lua, shapes, sounds, maps)

The system MUST parse all resource reference elements that declare which files the plugin provides.

#### Scenario: WHEN a plugin has MML elements THEN MML file paths are collected and sorted

- WHEN a Plugin.xml contains:
  ```xml
  <plugin name="P">
    <mml file="b.mml"/>
    <mml file="a.mml"/>
  </plugin>
  ```
- THEN the parsed plugin has mmls list ["a.mml", "b.mml"] (sorted alphabetically)

#### Scenario: WHEN a plugin has a hud_lua attribute THEN the HUD Lua path is captured

- WHEN a Plugin.xml contains `<plugin name="P" hud_lua="Scripts/hud.lua">`
- THEN the parsed plugin has hud_lua set to "Scripts/hud.lua"

#### Scenario: WHEN a plugin has a solo_lua element THEN the solo Lua path and write access flags are captured

- WHEN a Plugin.xml contains:
  ```xml
  <plugin name="P">
    <solo_lua file="Scripts/solo.lua">
      <write_access>fog</write_access>
      <write_access>music</write_access>
    </solo_lua>
  </plugin>
  ```
- THEN the parsed plugin has solo_lua set to "Scripts/solo.lua"
- AND solo_lua_write_access flags include fog (0x02) and music (0x04)

#### Scenario: WHEN a plugin uses the legacy solo_lua attribute THEN it is parsed as a fallback

- WHEN a Plugin.xml contains `<plugin name="P" solo_lua="Scripts/solo.lua">` with no `<solo_lua>` child element
- THEN the parsed plugin has solo_lua set to "Scripts/solo.lua"
- AND solo_lua_write_access defaults to world (0x01)

#### Scenario: WHEN a plugin has more than one solo_lua element THEN it is an error

- WHEN a Plugin.xml contains two or more `<solo_lua>` child elements
- THEN the parser logs an error and does not set solo_lua

#### Scenario: WHEN solo_lua has no write_access children THEN write access defaults to world

- WHEN a Plugin.xml contains `<solo_lua file="solo.lua"/>` with no `<write_access>` children
- THEN solo_lua_write_access defaults to world (0x01)

#### Scenario: WHEN write_access values are parsed THEN all six flag types are recognized

- WHEN write_access elements contain values "world", "fog", "music", "overlays", "ephemera", or "sound"
- THEN the corresponding flag bits are set: world=0x01, fog=0x02, music=0x04, overlays=0x08, ephemera=0x10, sound=0x20

#### Scenario: WHEN a plugin has a stats_lua attribute THEN the stats Lua path is captured

- WHEN a Plugin.xml contains `<plugin name="P" stats_lua="Scripts/stats.lua">`
- THEN the parsed plugin has stats_lua set to "Scripts/stats.lua"

#### Scenario: WHEN a plugin has shapes_patch elements THEN shapes patch paths and OpenGL flags are collected

- WHEN a Plugin.xml contains:
  ```xml
  <plugin name="P">
    <shapes_patch file="patch.shpA" requires_opengl="true"/>
    <shapes_patch file="patch.shpB"/>
  </plugin>
  ```
- THEN the parsed plugin has two shapes_patches entries
- AND the first has path "patch.shpA" and requires_opengl true
- AND the second has path "patch.shpB" and requires_opengl false (default)

#### Scenario: WHEN a plugin has sounds_patch elements THEN sounds patch paths are collected

- WHEN a Plugin.xml contains:
  ```xml
  <plugin name="P">
    <sounds_patch file="patch.sndA"/>
  </plugin>
  ```
- THEN the parsed plugin has one sounds_patches entry with path "patch.sndA"

#### Scenario: WHEN a plugin has map_patch elements THEN checksums and resource mappings are captured

- WHEN a Plugin.xml contains:
  ```xml
  <plugin name="P">
    <map_patch>
      <checksum>12345</checksum>
      <checksum>67890</checksum>
      <resource type="snd " id="100" data="sounds/custom.rsrc"/>
    </map_patch>
  </plugin>
  ```
- THEN the parsed plugin has one map_patch with parent_checksums {12345, 67890}
- AND the resource_map maps (tag for "snd ", id 100) to "sounds/custom.rsrc"

#### Scenario: WHEN a map_patch resource type is a 4-character tag THEN it is converted to a u32 via Mac Roman encoding

- WHEN a resource element has `type="snd "`
- THEN the type is converted to a u32 using the four bytes of the Mac Roman encoding of the string

#### Scenario: WHEN a map_patch has no checksums or no resources THEN it is skipped

- WHEN a map_patch element has no `<checksum>` children or no `<resource>` children
- THEN that map_patch is not added to the plugin's map_patches list

#### Scenario: WHEN a resource type does not encode to exactly 4 Mac Roman bytes THEN the resource entry is skipped

- WHEN a resource element has a type attribute that does not produce exactly 4 bytes in Mac Roman encoding
- THEN that resource entry is not added to the resource_map

---

### Requirement: Discover plugins from directory structure

The system MUST recursively scan directories to find Plugin.xml files and construct Plugin entries from them.

#### Scenario: WHEN a directory contains a Plugin.xml file THEN a plugin is discovered at that location

- WHEN scanning a directory that contains a file named "Plugin.xml"
- THEN the parser reads and parses that Plugin.xml
- AND the resulting plugin's directory is set to the directory containing Plugin.xml

#### Scenario: WHEN a directory contains subdirectories THEN they are scanned recursively

- WHEN scanning a directory that contains subdirectories
- THEN each subdirectory is recursively scanned for Plugin.xml files
- AND subdirectories whose names start with '.' are skipped

#### Scenario: WHEN a directory contains ZIP files THEN they are searched for Plugin.xml entries

- WHEN scanning a directory that contains .zip or .ZIP files
- THEN the ZIP archive is inspected for entries named "Plugin.xml" or entries ending with "/Plugin.xml"
- AND any found Plugin.xml within the ZIP is parsed as a plugin

#### Scenario: WHEN the standard Plugins directory is scanned THEN plugins from all data search paths are enumerated

- WHEN plugin enumeration is triggered
- THEN the system scans a "Plugins" subdirectory under each data search path
- AND all discovered plugins are collected into a single list

---

### Requirement: Determine plugin load order

The system MUST sort plugins and resolve conflicts when multiple plugins provide exclusive resources.

#### Scenario: WHEN plugins are enumerated THEN they are sorted alphabetically by name

- WHEN plugin enumeration completes
- THEN the plugin list is sorted by plugin name in ascending alphabetical order

#### Scenario: WHEN multiple plugins provide a HUD Lua script THEN only the last one in order is active

- WHEN two or more valid plugins declare a hud_lua path
- THEN iterating from the end of the sorted list, only the first encountered plugin with hud_lua is considered active for HUD Lua
- AND earlier plugins with hud_lua are marked as overridden

#### Scenario: WHEN multiple plugins provide a stats Lua script THEN only the last one in order is active

- WHEN two or more valid plugins declare a stats_lua path
- THEN only the last plugin (in sorted order) with stats_lua is active for stats Lua

#### Scenario: WHEN multiple plugins provide a theme THEN only the last one in order is active

- WHEN two or more valid plugins declare a theme
- THEN only the last plugin (in sorted order) with a theme is active for the theme

#### Scenario: WHEN solo Lua plugins have conflicting write access THEN exclusive flags determine which are active

- WHEN iterating plugins from the end in sorted order
- AND a plugin's solo_lua_write_access exclusive flags overlap with the accumulated exclusive flags of later plugins
- THEN that plugin is marked as overridden for solo mode
- AND the exclusive_mask includes world, fog, music, and overlays (0x0F)

#### Scenario: WHEN a plugin with world write access is active THEN it excludes all other exclusive solo Lua

- WHEN a plugin has solo_lua_write_access with the world flag (0x01)
- THEN its exclusive flags are the full exclusive_mask (world | fog | music | overlays)
- AND any earlier plugin with exclusive solo Lua flags is overridden

#### Scenario: WHEN MML files from multiple valid plugins are loaded THEN they are applied in plugin order

- WHEN multiple valid plugins each declare MML files
- THEN MML files are loaded in plugin list order (alphabetically by plugin name)
- AND later MML entries override earlier ones for the same configuration keys

---

### Requirement: Handle missing or malformed Plugin.xml gracefully

The system MUST handle error cases without crashing or corrupting other plugin data.

#### Scenario: WHEN a Plugin.xml file cannot be opened THEN it is silently skipped

- WHEN the parser attempts to open a Plugin.xml file and the file cannot be read
- THEN the parser returns without adding any plugin
- AND no panic or unrecoverable error occurs

#### Scenario: WHEN a Plugin.xml contains malformed XML THEN an error is logged and the plugin is skipped

- WHEN a Plugin.xml file contains invalid XML (parse error, missing root element, etc.)
- THEN the parser logs an error message identifying the plugin directory
- AND the plugin is not added to the plugin list

#### Scenario: WHEN a Plugin.xml has unexpected elements or attributes THEN they are ignored

- WHEN a Plugin.xml contains elements or attributes not recognized by the parser
- THEN the unrecognized content is silently ignored
- AND recognized content is still parsed correctly

#### Scenario: WHEN a referenced resource file does not exist THEN the reference is cleared

- WHEN a Plugin.xml references a hud_lua, solo_lua, or stats_lua file that does not exist in the plugin directory
- THEN the corresponding field is set to empty/cleared
- AND the plugin is still added (with the cleared field)

#### Scenario: WHEN a referenced MML file does not exist THEN it is excluded from the MML list

- WHEN a Plugin.xml contains an `<mml file="missing.mml"/>` entry and the file does not exist in the plugin directory
- THEN "missing.mml" is not added to the plugin's mmls list

#### Scenario: WHEN a referenced shapes_patch file does not exist THEN it is excluded

- WHEN a Plugin.xml contains a `<shapes_patch file="missing.shpA"/>` entry and the file does not exist
- THEN that shapes patch is not added to the plugin's shapes_patches list

#### Scenario: WHEN a referenced sounds_patch file does not exist THEN it is excluded

- WHEN a Plugin.xml contains a `<sounds_patch file="missing.sndA"/>` entry and the file does not exist
- THEN that sounds patch is not added to the plugin's sounds_patches list
