## Why

The MML parser in `marathon-formats/src/mml.rs` successfully parses all 24 MML sections into XML trees, but none of the parsed data is applied at runtime -- it is parsed and discarded. This means every community scenario that customizes monster behavior, weapon stats, physics constants, texture assignments, UI strings, or any other game data through MML gets no effect. Since virtually all Marathon community content (total conversions, plugins, balance mods) relies on MML overrides, this is a hard blocker for scenario compatibility.

Additionally, the current `MmlDocument::layer()` method uses section-level replacement (`Option::or`), meaning an overlay that modifies a single monster replaces the entire `<monsters>` section. Real-world plugins expect element-level merging by `index` attribute -- e.g., a plugin that changes monster #3's vitality should leave all other monsters untouched. Without this, stacking multiple plugins (the normal case) silently destroys each other's changes.

## What Changes

- Add MML interpretation logic that reads parsed `MmlElement` trees and applies their attribute values to the corresponding runtime data structures (`MonsterDefinition`, `WeaponDefinition`, `ProjectileDefinition`, `EffectDefinition`, `PhysicsConstants`, and non-physics tables like string sets, interface colors, texture loading rules, and dynamic limits)
- Replace the section-level `Option::or` layering in `MmlDocument::layer()` with element-level merging that matches elements by `index` attribute within each section, preserving unmentioned entries from the base layer
- Implement the full override cascade: engine defaults -> global MML -> local MML -> scenario MML -> plugin MML -> level-embedded MML, wired into the game's level loading path so overrides take effect before simulation and rendering begin
- Add an `MmlOverrideSet` (or similar) struct that holds the flattened, merged override state and provides typed accessor methods for each section, consumed by marathon-sim (physics/combat data), marathon-game and marathon-web (texture assignments, interface config, string tables)

## Capabilities

### New Capabilities

- `mml-interpretation`: Reading parsed MML element trees and converting attribute key/value pairs into mutations on typed Rust game data structures -- mapping XML attribute names to struct fields for all 24 MML sections, with numeric parsing (integer, fixed-point), boolean flags, and enum/index resolution matching AlephOne's MML semantics
- `mml-element-merge`: Element-level merging within MML sections by `index` attribute -- when two MML layers both define a `<monsters>` section, individual `<monster index="N">` entries are merged by index rather than the overlay replacing the entire section, and within a single element, only attributes present in the overlay update the base values
- `mml-override-cascade`: Assembling the full override cascade (engine defaults, global MML, local MML, scenario MML, per-plugin MML in load order, level-embedded MML) into a single resolved configuration, integrated into the level loading path so all subsystems receive overridden data before initialization

### Modified Capabilities

- `mml-config`: The existing MML parser and `MmlDocument::layer()` method gain element-level merge semantics; the `MmlSection` type may gain helper methods for index-based element lookup
- `game-loop`: Level loading sequence applies the resolved MML override set to `PhysicsData` and other game tables before constructing `SimWorld` and initializing renderers
- `game-shell`: Scenario and plugin discovery feeds MML file lists into the override cascade; level transitions re-apply level-embedded MML on top of the scenario+plugin base

## Impact

- **marathon-formats/src/mml.rs** -- `MmlDocument::layer()` rewritten for element-level merge; new `MmlSection` merge helpers; new module or file for MML interpretation (attribute-to-field mapping for each section)
- **marathon-formats/src/physics.rs** -- `PhysicsData`, `MonsterDefinition`, `WeaponDefinition`, `ProjectileDefinition`, `EffectDefinition`, `PhysicsConstants` gain methods or trait impls to apply MML attribute overrides to individual fields
- **marathon-formats/src/plugin.rs** -- Plugin MML file list already parsed; no structural change, but the integration layer must load and parse each plugin's MML files in order
- **marathon-sim/src/world.rs** -- `SimWorld::new()` accepts overridden `PhysicsData` (and potentially overridden dynamic limits) rather than raw parsed data
- **marathon-game/src/render.rs** and **marathon-web/src/render.rs** -- Texture loading, landscape assignments, and OpenGL settings read from the resolved MML override set instead of hardcoded defaults
- **New test fixtures** -- MML override round-trip tests verifying element-level merge, cascade ordering, and attribute application against known AlephOne behavior
- **No breaking API changes** -- existing `MmlDocument` parsing API remains; `layer()` changes are backward-compatible (element-level merge is a superset of section-level replace)
