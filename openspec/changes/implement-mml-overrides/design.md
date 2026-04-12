## Context

The `marathon-formats` crate has a complete MML parser (`mml.rs`) that reads all 24 recognized `<marathon>` sections into generic XML trees (`MmlElement` with `HashMap<String, String>` attributes). The parser supports section-level layering via `MmlDocument::layer()`, which uses `Option::or` semantics -- an overlay section entirely replaces the base section.

However, none of the parsed MML data is interpreted at runtime. The XML trees are discarded after parsing. This means community scenarios that customize monster stats, weapon behavior, physics constants, texture assignments, HUD layout, string tables, and other game data through MML get no effect. Since virtually all Marathon community content depends on MML overrides, this is a hard blocker for scenario compatibility.

Additionally, the section-level replacement in `layer()` is too coarse. Real-world plugins expect element-level merging by `index` attribute -- a plugin that changes monster #3's vitality should leave all other monsters untouched. Without this, stacking multiple plugins silently destroys each other's changes.

The `PhysicsData` struct holds `Vec<MonsterDefinition>`, `Vec<WeaponDefinition>`, `Vec<ProjectileDefinition>`, `Vec<EffectDefinition>`, and `Vec<PhysicsConstants>`, all parsed from binary WAD tags via `binrw`. The MML override system must produce mutations that modify individual fields within these structs. Non-physics sections (strings, interface, textures, dynamic limits, etc.) need their own typed override representations consumed by the integration layer and renderers.

Plugins are already discovered and sorted alphabetically by `plugin.rs`, with `mml_files: Vec<String>` listing each plugin's MML files. The infrastructure for loading plugin MML in order already exists.

## Goals / Non-Goals

**Goals:**
- Convert parsed MML XML trees into typed Rust override structs for all 24 sections
- Implement element-level merge by `index` attribute so plugins can modify individual monsters, weapons, items, etc. without clobbering sibling entries
- Implement the full override cascade: engine defaults -> global MML -> local MML -> scenario MML -> plugin MML (per plugin, in alphabetical order) -> level-embedded MML
- Produce a single resolved `MmlOverrideSet` that downstream systems consume
- Wire the resolved overrides into the level loading pipeline so `SimWorld`, renderers, and HUD receive overridden data
- Support the priority MML sections needed for gameplay: physics (monsters, weapons, projectiles, effects, physics_constants), dynamic_limits, player, items, landscapes, and texture_loading

**Non-Goals:**
- Full OpenGL/software section interpretation (hi-res texture replacement, 3D model replacement, custom shaders) -- these require asset loading infrastructure not yet built
- Per-level MML scripts via `<marathon_levels>` -- deferred to a separate change
- Lua console integration (the `<console lua="">` flag) -- depends on Lua VM integration
- Runtime hot-reloading of MML files -- overrides are computed once per level load
- GUI for MML editing or visualization

## Decisions

### 1. Typed override structs use `Option<T>` for every field

**Decision:** Each section's override struct wraps every field in `Option<T>`. A `None` field means "no override, keep the base value." A `Some(v)` field means "set this field to `v`."

**Rationale:** MML elements are sparse -- a `<monster index="5" vitality="300"/>` only overrides vitality, leaving all other fields at their base values. Using `Option<T>` per field naturally represents this sparsity and makes the merge operation straightforward: for each field, use the overlay's value if `Some`, otherwise keep the base. This mirrors how AlephOne's XML_ElementParser works internally -- each attribute setter only fires for attributes present in the XML.

**Alternative considered:** A `HashMap<String, String>` per element, deferring parsing to application time. Rejected because it pushes type errors to runtime and forces every consumer to re-parse attribute strings.

### 2. Element-level merge by `index` attribute within MmlSection

**Decision:** Upgrade `MmlSection` with a `merge()` method that combines two sections by matching child elements on their `index` attribute. Elements with the same `index` have their attributes merged (overlay wins per-attribute). Elements present only in one section are preserved. Elements without an `index` attribute are appended (not merged). `MmlDocument::layer()` calls `MmlSection::merge()` instead of `Option::or`.

**Rationale:** This matches AlephOne's actual behavior. A plugin defining `<monsters><monster index="5" vitality="300"/></monsters>` only modifies monster 5; monsters 0-4 and 6+ from the base layer remain untouched. Section-level replacement (the current behavior) would discard all other monsters. The merge also recurses into child elements by matching on element name + index, supporting nested structures like `<weapon index="0"><shell_casings index="1" .../>`.

**Alternative considered:** Merging only at the section level and relying on typed override structs to handle per-element sparsity. Rejected because the raw `MmlElement` tree must be correct before interpretation -- interpretation should read the merged tree, not implement merge logic itself.

### 3. Interpretation layer as a separate module from the parser

**Decision:** Create a new module `marathon-formats/src/mml_interpret.rs` (or `mml/interpret.rs`) containing the attribute-to-field mapping logic for each section. The parser (`mml.rs`) remains purely structural (XML to `MmlElement` trees). The interpretation module reads `MmlElement` trees and produces typed override structs.

**Rationale:** Separation of concerns. The parser is stable and well-tested. Interpretation involves per-section domain knowledge (attribute names, value ranges, type conversions) that will evolve as more sections are fully supported. Keeping them separate means parser changes don't risk interpretation logic and vice versa.

**Alternative considered:** Adding interpretation directly into `MmlDocument` methods. Rejected because it would bloat `mml.rs` significantly (24 sections, dozens of attributes each) and mix XML parsing with game-domain logic.

### 4. MmlOverrideSet as a flat struct with typed section accessors

**Decision:** Define `MmlOverrideSet` as a struct with one field per section, each holding that section's typed override data (e.g., `monsters: Vec<MonsterOverride>`, `dynamic_limits: DynamicLimitsOverride`, `player: PlayerOverride`). The struct is constructed once during level loading from the merged `MmlDocument` and passed to subsystems by reference.

**Rationale:** A flat struct is simple, efficient, and makes the API surface explicit. Subsystems take `&MmlOverrideSet` and access only the sections they need. No dynamic dispatch, no trait objects, no runtime section lookup.

**Alternative considered:** A `HashMap<SectionType, Box<dyn Any>>` for extensibility. Rejected because it sacrifices type safety and requires downcasting at every access site.

### 5. Override application via `apply_overrides()` methods on physics structs

**Decision:** Add `apply_mml_overrides(&mut self, overrides: &[MonsterOverride])` methods (or similar) to `PhysicsData`, `MonsterDefinition`, `WeaponDefinition`, etc. These methods iterate the override list and, for each override, find the matching definition by index and apply each `Some` field.

**Rationale:** Keeps the override application close to the data structures being modified. The physics structs already exist with all the right fields. Adding a method to each struct is the most direct approach and keeps the dependency graph clean (no reverse dependency from formats to integration).

**Alternative considered:** A standalone applicator function in the integration layer. This would work but requires the integration layer to have intimate knowledge of every physics struct field. Placing the method on the struct keeps that knowledge encapsulated.

### 6. Attribute parsing uses Marathon/AlephOne conventions

**Decision:** Numeric attributes are parsed as: decimal integers, hex (0x prefix), fixed-point (where AlephOne expects fixed-point, multiply by 65536 for storage or divide for display as the context requires). Booleans accept `1`, `t`, `true` for true and `0`, `f`, `false` for false. Missing attributes produce `None` (no override). Malformed attribute values log a warning and produce `None` rather than failing the entire parse.

**Rationale:** Matches AlephOne's lenient parsing behavior. Community MML files have been written against AlephOne's parser for decades. Being strict about value formats would break existing content. Logging warnings helps content creators find issues without blocking gameplay.

### 7. Override cascade assembled during level loading

**Decision:** The cascade is assembled in `game-shell`'s level loading sequence:
1. Start with an empty `MmlDocument`
2. Layer global MML files (from `MML/` directories, alphabetical)
3. Layer local MML files (from `Scripts/` directory, alphabetical)
4. Layer scenario MML (from the scenario WAD's global entry)
5. For each enabled plugin (alphabetical by name), layer each of its MML files (in the order listed in Plugin.xml, which are already sorted)
6. Layer level-embedded MML (from the current level's MMLS WAD tag)
7. Interpret the final merged `MmlDocument` into an `MmlOverrideSet`
8. Apply the `MmlOverrideSet` to `PhysicsData` before constructing `SimWorld`
9. Pass the `MmlOverrideSet` to renderers and HUD for non-physics overrides

**Rationale:** This matches AlephOne's documented cascade order. Steps 2-5 produce a "scenario+plugin base" that is stable across level transitions. Step 6 adds per-level overrides. On level transitions, steps 1-5 are cached and only step 6 changes. Step 7 interpretation happens after all merging is complete, so each attribute is parsed exactly once from the final merged tree.

## Risks / Trade-offs

**[Incomplete section coverage]** Not all 24 MML sections can be fully interpreted in one pass. Physics sections (monsters, weapons, projectiles, effects, physics_constants) are highest priority for gameplay. Other sections (opengl, software, faders, sounds) will have stub interpreters that log "not yet implemented" until their consuming subsystems are ready. Mitigation: prioritize by gameplay impact; stub interpreters ensure the cascade and merge logic work for all sections even if interpretation is partial.

**[AlephOne compatibility edge cases]** AlephOne's MML parsing has accumulated decades of quirks (e.g., some attributes are parsed as signed 16-bit then stored as unsigned, some fixed-point fields have non-standard scaling). Mitigation: test against real-world MML files from popular scenarios (Rubicon X, Phoenix, Eternal) and compare behavior with AlephOne where discrepancies are found.

**[Performance of element-level merge]** Element-level merge scans by `index` attribute, which is O(n*m) for n base elements and m overlay elements. Mitigation: Monster/weapon/projectile lists are small (typically <50 elements), so linear scan is fine. If profiling shows issues, switch to a HashMap-based merge.

**[Merge semantics for non-indexed elements]** Some sections contain elements without `index` attributes (e.g., `<clear>` directives in `<animated_textures>`, `<opengl>`). These need special handling -- `<clear>` typically resets the section before subsequent entries are applied. Mitigation: handle `<clear>` elements as section-reset markers during interpretation, not during merge. The merge layer preserves all elements; interpretation handles semantic meaning.

## Open Questions

- Should the `MmlOverrideSet` be cached across level transitions for the scenario+plugin base, with only level-embedded MML re-applied? This is an optimization that matters for scenarios with many plugins.
- For the `<stringset>` section, should overridden strings be stored in a flat `HashMap<(u16, u16), String>` (resource_id, string_index -> string) or in a nested `Vec<Vec<String>>` mirroring the resource structure?
- Should we validate MML attribute value ranges (e.g., monster index 0-46, weapon index 0-9) during interpretation, or silently ignore out-of-range values? AlephOne silently clamps/ignores, which suggests we should do the same.
