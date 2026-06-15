## 1. MML Interpretation Layer

- [x] 1.1 Create `marathon-formats/src/mml_interpret.rs` module with attribute parsing helpers: `parse_mml_i16`, `parse_mml_i32`, `parse_mml_u32`, `parse_mml_f32`, `parse_mml_bool` with AlephOne-compatible rules (decimal, hex 0x prefix, boolean 1/t/true/0/f/false), returning `Option<T>` and logging warnings for malformed values
- [ ] 1.2 Define `MonsterOverride` struct with `index: usize` and `Option<T>` fields for all `MonsterDefinition` fields (vitality, immunities, weaknesses, flags, class, friends, enemies, sound_pitch, speed, radius, height, visual_range, etc.) plus `must_be_exterminated: Option<bool>`
- [ ] 1.3 Implement `interpret_monsters(section: &MmlSection) -> Vec<MonsterOverride>` that iterates `<monster>` elements, parses `index` attribute, and maps each recognized attribute to the corresponding `MonsterOverride` field
- [ ] 1.4 Define `ShellCasingOverride`, `WeaponOrderEntry`, and implement `interpret_weapons(section: &MmlSection)` producing shell casing overrides and weapon order data
- [ ] 1.5 Define `ProjectileOverride` struct and implement `interpret_projectiles(section: &MmlSection) -> Vec<ProjectileOverride>` for all `ProjectileDefinition` fields
- [ ] 1.6 Define `EffectOverride` struct and implement `interpret_effects(section: &MmlSection) -> Vec<EffectOverride>` for all `EffectDefinition` fields
- [ ] 1.7 Define `PlayerOverride` struct and implement `interpret_player(section: &MmlSection) -> PlayerOverride` for player attributes (energy, oxygen, light, visual arcs, swim, powerup durations, starting items)
- [ ] 1.8 Define `DynamicLimitsOverride` struct and implement `interpret_dynamic_limits(section: &MmlSection) -> DynamicLimitsOverride` parsing child element text content as integers
- [ ] 1.9 Define `ItemOverride` struct and implement `interpret_items(section: &MmlSection) -> Vec<ItemOverride>` for item type, names, maximum, invalid flag
- [ ] 1.10 Define `LandscapeOverride` struct and implement `interpret_landscapes(section: &MmlSection)` producing landscape overrides and clear directives
- [ ] 1.11 Define `TextureLoadingOverride` struct and implement `interpret_texture_loading(section: &MmlSection)` for the landscapes flag and texture_env entries
- [ ] 1.12 Define `StringSetOverride` as a collection of `(resource_id, string_index) -> String` entries and implement `interpret_stringset(section: &MmlSection)` parsing resource ID from stringset `index` and string entries from child `<string>` elements
- [ ] 1.13 Define `ScenarioIdOverride` struct and implement `interpret_scenario(section: &MmlSection)` for scenario name, version, id attributes
- [ ] 1.14 Add stub interpreters for remaining sections (interface, motion_sensor, overhead_map, infravision, animated_textures, control_panels, platforms, liquids, sounds, faders, view, scenery, opengl, software, console, logging) that log "not yet implemented" and return empty/default overrides
- [x] 1.15 Register `mml_interpret` module in `marathon-formats/src/lib.rs`

## 2. Element-Level Merge Logic

- [x] 2.1 Add `MmlSection::find_element(&self, name: &str, index: &str) -> Option<&MmlElement>` method for index-based element lookup
- [x] 2.2 Add `MmlElement::merge_attributes(&mut self, overlay: &MmlElement)` method that copies overlay attributes into self (overlay wins on conflict, base-only attributes preserved)
- [x] 2.3 Add `MmlElement::merge_children(&mut self, overlay: &MmlElement)` method that recursively merges child elements by name+index
- [x] 2.4 Implement `MmlSection::merge(base: Self, overlay: Self) -> Self` that matches elements by name+index, merges matched pairs, preserves unmatched elements from both sides, and appends non-indexed overlay elements
- [x] 2.5 Update `MmlDocument::layer()` to call `MmlSection::merge()` when both base and overlay have the same section, instead of `Option::or`
- [x] 2.6 Update existing `layer()` tests to verify element-level merge behavior (monster index preservation, attribute-level merge)
- [x] 2.7 Add new tests: overlay modifies one element among many, overlay adds new indexed element, attribute-level merge preserves unmentioned attributes, recursive child merge, three-layer cascade

## 3. Override Cascade Assembly

- [ ] 3.1 Define `MmlOverrideSet` struct in `marathon-formats/src/mml_interpret.rs` with typed fields for all section overrides (monster, weapon, projectile, effect, player, dynamic_limits, item, landscape, texture_loading, stringset, scenario, plus stubs for remaining sections)
- [ ] 3.2 Implement `MmlOverrideSet::from_document(doc: &MmlDocument) -> Self` that calls the appropriate `interpret_*` function for each populated section
- [ ] 3.3 Implement `assemble_mml_cascade()` function (in marathon-formats or a new integration module) that takes global MML paths, local MML paths, scenario WAD entry, plugin metadata list, and current level WAD entry, layers all documents in cascade order, and returns the final merged `MmlDocument`
- [ ] 3.4 Add `MmlCascadeCache` struct that stores the scenario+plugin MML base and provides `with_level_mml(level_entry: &WadEntry) -> MmlDocument` to layer level-embedded MML on top of the cached base
- [ ] 3.5 Add tests for cascade ordering: global < local < scenario < plugin < level, plugin alphabetical order, level MML override reset on transition

## 4. Physics Override Application

- [ ] 4.1 Add `MonsterDefinition::apply_override(&mut self, ovr: &MonsterOverride)` method that sets each field from the override's `Some` values
- [ ] 4.2 Add `WeaponDefinition::apply_override(&mut self, ovr: &WeaponOverride)` if weapon physics overrides are defined (or defer to when weapon override structs are implemented)
- [ ] 4.3 Add `ProjectileDefinition::apply_override(&mut self, ovr: &ProjectileOverride)` method
- [ ] 4.4 Add `EffectDefinition::apply_override(&mut self, ovr: &EffectOverride)` method
- [ ] 4.5 Add `PhysicsData::apply_overrides(&mut self, overrides: &MmlOverrideSet)` method that iterates each override type and applies to the corresponding definition by index, silently skipping out-of-bounds indices
- [ ] 4.6 Add tests: apply single monster override, apply multiple overlapping overrides, out-of-bounds index ignored, `None` fields preserve original values

## 5. Integration into Level Loading Pipeline

- [ ] 5.1 In `marathon-game` (or `marathon-integration`) level loading path, add MML cascade assembly after WAD/physics parsing and before `SimWorld` construction
- [ ] 5.2 Call `PhysicsData::apply_overrides()` with the resolved `MmlOverrideSet` before passing physics data to `SimWorld::new()`
- [ ] 5.3 Store the `MmlOverrideSet` alongside the level state so renderers and HUD can query it
- [ ] 5.4 Initialize `MmlCascadeCache` during scenario loading (after plugin discovery), reuse across level transitions
- [ ] 5.5 On level transition, rebuild `MmlOverrideSet` from cached base + new level's embedded MML
- [ ] 5.6 Wire dynamic limits from `MmlOverrideSet` into `SimWorld` initialization (entity pool sizes)
- [ ] 5.7 Wire landscape and texture_loading overrides to the rendering pipeline during level init

## 6. Testing

- [ ] 6.1 Unit tests for each `parse_mml_*` helper: decimal, hex, boolean variants, malformed values return None
- [ ] 6.2 Unit tests for each `interpret_*` function: valid elements, missing index, malformed attributes, empty sections
- [ ] 6.3 Unit tests for `MmlSection::merge()`: index matching, attribute preservation, recursive child merge, non-indexed element handling
- [ ] 6.4 Unit tests for `MmlDocument::layer()` with element-level merge: backward compatibility with existing tests plus new merge scenarios
- [ ] 6.5 Unit tests for `MmlOverrideSet::from_document()`: empty document, document with subset of sections, full document
- [ ] 6.6 Unit tests for `PhysicsData::apply_overrides()`: single override, multiple overrides, out-of-bounds, None fields, empty override set
- [ ] 6.7 Integration tests with real MML snippets from Marathon 2 and Infinity scenarios: monster stats, weapon modifications, dynamic limits changes
- [ ] 6.8 Integration test for full cascade: global + scenario + plugin + level MML layered and applied to physics data, verify final values match expected cascade order
- [ ] 6.9 Verify all tests pass in Docker CI (`cargo test` in `marathon-formats` crate)
