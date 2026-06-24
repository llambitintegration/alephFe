//! MML override cascade assembly.
//!
//! Marathon resolves MML (Marathon Markup Language) overrides by layering
//! several sources in a fixed precedence order, where a later layer wins over
//! an earlier one:
//!
//! ```text
//! global  <  local  <  scenario  <  plugins (alphabetical)  <  level
//! ```
//!
//! Each layer is parsed into an [`MmlDocument`] and folded together with
//! [`MmlDocument::layer`] (which is overlay-wins at element granularity). The
//! fold starts from an empty document and applies sources from lowest to
//! highest priority, so the final document carries every source's contributions
//! with the highest-priority value winning on conflict.
//!
//! ## Resilient layering
//!
//! A source that is missing, unreadable, or malformed is skipped with a warning
//! (matching the crate's `eprintln!("[mml] ...")` convention) rather than
//! aborting the whole cascade. One bad plugin MML file must not prevent the rest
//! of the scenario's overrides from loading.
//!
//! ## Modeling note: plugin MML sources
//!
//! The existing [`crate::plugin::PluginMetadata`] type lists a plugin's MML
//! files *by name* (`mml_files`), but not their bytes — resolving those names to
//! bytes requires plugin-directory filesystem context that lives in the game
//! shell, not here. To keep this module free of a plugin-discovery subsystem,
//! the cascade consumes a caller-provided list of [`PluginMmlSource`] values,
//! each pairing a plugin `name` (for alphabetical ordering) with already-read
//! `mml` bytes. The game shell is responsible for walking `PluginMetadata` +
//! `discover_plugins()` and producing this flat list; this module just layers
//! whatever it is handed.

use std::path::{Path, PathBuf};

use crate::mml::MmlDocument;
use crate::wad::WadEntry;

/// One plugin's MML contribution: a name (used for alphabetical ordering) and
/// the raw MML bytes to parse and layer into the cascade.
///
/// This is a deliberately minimal model — see the module docs for why the cascade
/// consumes pre-read bytes rather than re-deriving them from [`PluginMetadata`].
///
/// [`PluginMetadata`]: crate::plugin::PluginMetadata
#[derive(Debug, Clone)]
pub struct PluginMmlSource {
    /// Plugin name, used to order plugin layers alphabetically.
    pub name: String,
    /// Raw MML document bytes for this plugin.
    pub mml: Vec<u8>,
}

impl PluginMmlSource {
    /// Convenience constructor.
    pub fn new(name: impl Into<String>, mml: impl Into<Vec<u8>>) -> Self {
        Self {
            name: name.into(),
            mml: mml.into(),
        }
    }
}

/// Parse a file path into an [`MmlDocument`], returning `None` (with a warning)
/// if the file is missing, unreadable, or malformed. Never panics.
fn parse_path_or_warn(path: &Path) -> Option<MmlDocument> {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[mml] skipping unreadable MML file {}: {e}", path.display());
            return None;
        }
    };
    parse_bytes_or_warn(&data, &path.display().to_string())
}

/// Parse MML bytes into an [`MmlDocument`], returning `None` (with a warning) on
/// a parse error. Never panics.
fn parse_bytes_or_warn(data: &[u8], source: &str) -> Option<MmlDocument> {
    match MmlDocument::from_bytes(data) {
        Ok(doc) => Some(doc),
        Err(e) => {
            eprintln!("[mml] skipping malformed MML from {source}: {e}");
            None
        }
    }
}

/// Parse a WAD entry's embedded MML, returning `None` (with a warning) on a
/// parse error and `None` (silently) when the entry has no MMLS tag. Never panics.
fn parse_wad_entry_or_warn(entry: &WadEntry, source: &str) -> Option<MmlDocument> {
    match MmlDocument::from_wad_entry(entry) {
        Ok(doc) => doc,
        Err(e) => {
            eprintln!("[mml] skipping malformed embedded MML from {source}: {e}");
            None
        }
    }
}

/// Fold a sequence of already-parsed documents, lowest-priority first, into a
/// single merged document. This is the pure core of the cascade: it has no I/O
/// and so is directly drivable by tests with constructed [`MmlDocument`]s.
///
/// `docs_in_order[0]` is the lowest priority and `docs_in_order[last]` the
/// highest; later documents override earlier ones via [`MmlDocument::layer`].
pub fn cascade_documents(docs_in_order: Vec<MmlDocument>) -> MmlDocument {
    docs_in_order
        .into_iter()
        .fold(MmlDocument::default(), MmlDocument::layer)
}

/// Assemble the full MML override cascade from its file/WAD sources.
///
/// Layering order (lowest to highest priority):
/// `global < local < scenario < plugins (alphabetical by name) < level`.
///
/// * `global_paths` / `local_paths` — MML files read from disk, applied in the
///   order given (callers typically pass them pre-sorted alphabetically).
/// * `scenario_entry` — the scenario WAD entry whose embedded MMLS tag, if any,
///   forms the scenario layer.
/// * `plugins` — plugin MML sources; sorted alphabetically by `name` here so the
///   caller need not pre-sort.
/// * `level_entry` — the current level's WAD entry, whose embedded MMLS tag, if
///   any, is the highest-priority layer.
///
/// Any individual source that is missing or malformed is skipped with a warning;
/// the cascade never fails as a whole and never panics.
pub fn assemble_mml_cascade(
    global_paths: &[PathBuf],
    local_paths: &[PathBuf],
    scenario_entry: Option<&WadEntry>,
    plugins: &[PluginMmlSource],
    level_entry: Option<&WadEntry>,
) -> MmlDocument {
    let mut docs: Vec<MmlDocument> = Vec::new();

    // global
    for path in global_paths {
        if let Some(doc) = parse_path_or_warn(path) {
            docs.push(doc);
        }
    }
    // local
    for path in local_paths {
        if let Some(doc) = parse_path_or_warn(path) {
            docs.push(doc);
        }
    }
    // scenario
    if let Some(entry) = scenario_entry {
        if let Some(doc) = parse_wad_entry_or_warn(entry, "scenario WAD entry") {
            docs.push(doc);
        }
    }
    // plugins, alphabetical by name
    let mut ordered_plugins: Vec<&PluginMmlSource> = plugins.iter().collect();
    ordered_plugins.sort_by(|a, b| a.name.cmp(&b.name));
    for plugin in ordered_plugins {
        if let Some(doc) = parse_bytes_or_warn(&plugin.mml, &format!("plugin '{}'", plugin.name)) {
            docs.push(doc);
        }
    }
    // level (highest priority)
    if let Some(entry) = level_entry {
        if let Some(doc) = parse_wad_entry_or_warn(entry, "level WAD entry") {
            docs.push(doc);
        }
    }

    cascade_documents(docs)
}

/// Caches the scenario+plugin MML base (global < local < scenario < plugins) so
/// that level transitions only re-layer the level-embedded MML on top, rather
/// than re-reading and re-parsing every scenario/plugin source per level.
///
/// The cached `base` is never mutated by [`with_level_mml`](Self::with_level_mml):
/// each call clones the base and layers the new level's MML onto the clone, so a
/// previous level's overrides never accumulate across transitions — every level
/// resets from the same scenario+plugin base.
#[derive(Debug, Clone)]
pub struct MmlCascadeCache {
    base: MmlDocument,
}

impl MmlCascadeCache {
    /// Build the cache from the non-level sources. This performs the same
    /// `global < local < scenario < plugins` fold as [`assemble_mml_cascade`]
    /// but omits the level layer, storing the result as the reusable base.
    pub fn new(
        global_paths: &[PathBuf],
        local_paths: &[PathBuf],
        scenario_entry: Option<&WadEntry>,
        plugins: &[PluginMmlSource],
    ) -> Self {
        let base = assemble_mml_cascade(global_paths, local_paths, scenario_entry, plugins, None);
        Self { base }
    }

    /// Construct a cache directly from an already-assembled base document. Useful
    /// for tests and for callers that resolve the base by other means.
    pub fn from_base(base: MmlDocument) -> Self {
        Self { base }
    }

    /// The cached scenario+plugin base document (never includes any level MML).
    pub fn base(&self) -> &MmlDocument {
        &self.base
    }

    /// Produce the resolved document for a level by layering that level's
    /// embedded MML on top of a *fresh clone* of the cached base. The cache's
    /// base is left untouched, so calling this for successive levels resets from
    /// the same base each time (no cross-level accumulation).
    ///
    /// If the level entry has no MMLS tag (or it is malformed), the returned
    /// document equals the base.
    pub fn with_level_mml(&self, level_entry: &WadEntry) -> MmlDocument {
        match parse_wad_entry_or_warn(level_entry, "level WAD entry") {
            Some(level_doc) => MmlDocument::layer(self.base.clone(), level_doc),
            None => self.base.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tags::WadTag;
    use crate::test_helpers::{TagData, WadBuilder};
    use crate::wad::{WadEntry, WadFile};

    /// Build a standalone `WadFile` carrying a single entry whose MMLS tag holds
    /// the given MML bytes, so tests can hand out a real `&WadEntry`.
    fn wad_with_mml(mml: &[u8]) -> WadFile {
        let data = WadBuilder::new()
            .version(4)
            .add_entry(0, vec![TagData::new(WadTag::MmlScript, mml.to_vec())])
            .build();
        WadFile::from_bytes(&data).unwrap()
    }

    fn entry_of(wad: &WadFile) -> &WadEntry {
        wad.entry(0).unwrap()
    }

    fn doc(xml: &[u8]) -> MmlDocument {
        MmlDocument::from_bytes(xml).unwrap()
    }

    fn monster_vitality(d: &MmlDocument, index: &str) -> Option<String> {
        d.monsters
            .as_ref()?
            .find_element("monster", index)
            .and_then(|m| m.attributes.get("vitality").cloned())
    }

    // ── 3.5: cascade ordering — a later layer overrides an earlier one ──

    #[test]
    fn test_cascade_documents_later_layer_wins() {
        // Same monster index, different vitality at each level of the cascade.
        // Order: global < local < scenario < plugin < level.
        let global = doc(
            b"<marathon><monsters><monster index=\"0\" vitality=\"10\"/></monsters></marathon>",
        );
        let local = doc(
            b"<marathon><monsters><monster index=\"0\" vitality=\"20\"/></monsters></marathon>",
        );
        let scenario = doc(
            b"<marathon><monsters><monster index=\"0\" vitality=\"30\"/></monsters></marathon>",
        );
        let plugin = doc(
            b"<marathon><monsters><monster index=\"0\" vitality=\"40\"/></monsters></marathon>",
        );
        let level = doc(
            b"<marathon><monsters><monster index=\"0\" vitality=\"50\"/></monsters></marathon>",
        );

        let result = cascade_documents(vec![global, local, scenario, plugin, level]);
        assert_eq!(
            monster_vitality(&result, "0").as_deref(),
            Some("50"),
            "highest-priority (level) layer must win"
        );
    }

    #[test]
    fn test_cascade_each_layer_overrides_only_predecessor() {
        // Verify the precedence is strictly ordered: removing the level layer
        // exposes the plugin value, etc.
        let global = doc(
            b"<marathon><monsters><monster index=\"0\" vitality=\"10\"/></monsters></marathon>",
        );
        let local = doc(
            b"<marathon><monsters><monster index=\"0\" vitality=\"20\"/></monsters></marathon>",
        );
        let scenario = doc(
            b"<marathon><monsters><monster index=\"0\" vitality=\"30\"/></monsters></marathon>",
        );

        assert_eq!(
            monster_vitality(&cascade_documents(vec![global.clone()]), "0").as_deref(),
            Some("10")
        );
        assert_eq!(
            monster_vitality(&cascade_documents(vec![global.clone(), local.clone()]), "0")
                .as_deref(),
            Some("20"),
            "local overrides global"
        );
        assert_eq!(
            monster_vitality(&cascade_documents(vec![global, local, scenario]), "0").as_deref(),
            Some("30"),
            "scenario overrides local"
        );
    }

    // ── 3.5: plugins applied in alphabetical order ──

    #[test]
    fn test_plugins_applied_in_alphabetical_order() {
        // Two plugins both override the same monster's vitality. The one that is
        // alphabetically *last* must win (applied last). Pass them out of order
        // to confirm the cascade sorts by name, not insertion order.
        let plugins = vec![
            PluginMmlSource::new(
                "Zeta",
                b"<marathon><monsters><monster index=\"0\" vitality=\"99\"/></monsters></marathon>"
                    .to_vec(),
            ),
            PluginMmlSource::new(
                "Alpha",
                b"<marathon><monsters><monster index=\"0\" vitality=\"11\"/></monsters></marathon>"
                    .to_vec(),
            ),
        ];

        let result = assemble_mml_cascade(&[], &[], None, &plugins, None);
        assert_eq!(
            monster_vitality(&result, "0").as_deref(),
            Some("99"),
            "alphabetically-last plugin (Zeta) applied last and wins"
        );
    }

    #[test]
    fn test_plugins_alphabetical_distinct_indices_all_preserved() {
        // Alpha touches monster 0, Beta touches monster 5 — both survive, in
        // alphabetical application order.
        let plugins = vec![
            PluginMmlSource::new(
                "Beta",
                b"<marathon><monsters><monster index=\"5\" vitality=\"55\"/></monsters></marathon>"
                    .to_vec(),
            ),
            PluginMmlSource::new(
                "Alpha",
                b"<marathon><monsters><monster index=\"0\" vitality=\"50\"/></monsters></marathon>"
                    .to_vec(),
            ),
        ];

        let result = assemble_mml_cascade(&[], &[], None, &plugins, None);
        assert_eq!(monster_vitality(&result, "0").as_deref(), Some("50"));
        assert_eq!(monster_vitality(&result, "5").as_deref(), Some("55"));
    }

    // ── 3.5: cached base unchanged across with_level_mml (transition resets) ──

    #[test]
    fn test_cache_base_unchanged_across_levels() {
        // Base from a scenario WAD entry; two different levels override player
        // energy differently. Each with_level_mml call must re-layer from the
        // same base, not accumulate the previous level's override.
        let scenario_wad = wad_with_mml(
            b"<marathon><player><item index=\"0\" energy=\"100\"/></player></marathon>",
        );
        let cache = MmlCascadeCache::new(&[], &[], Some(entry_of(&scenario_wad)), &[]);

        let level2_wad = wad_with_mml(
            b"<marathon><player><item index=\"0\" energy=\"200\"/></player></marathon>",
        );
        let level3_wad = wad_with_mml(
            b"<marathon><player><item index=\"0\" energy=\"300\"/></player></marathon>",
        );

        let player_energy = |d: &MmlDocument| -> Option<String> {
            d.player
                .as_ref()?
                .find_element("item", "0")
                .and_then(|e| e.attributes.get("energy").cloned())
        };

        let l2 = cache.with_level_mml(entry_of(&level2_wad));
        assert_eq!(player_energy(&l2).as_deref(), Some("200"));

        // Transition to level 3: must reset from base (energy 100), then apply
        // level 3 (energy 300) — NOT carry level 2's 200.
        let l3 = cache.with_level_mml(entry_of(&level3_wad));
        assert_eq!(player_energy(&l3).as_deref(), Some("300"));

        // The cached base itself is untouched: still the scenario's energy 100.
        assert_eq!(
            player_energy(cache.base()).as_deref(),
            Some("100"),
            "cached base must remain the scenario value, not a level override"
        );

        // Re-applying level 2 again yields 200 (clean reset, no accumulation).
        let l2_again = cache.with_level_mml(entry_of(&level2_wad));
        assert_eq!(player_energy(&l2_again).as_deref(), Some("200"));
    }

    #[test]
    fn test_with_level_mml_no_embedded_mml_equals_base() {
        let scenario_wad = wad_with_mml(
            b"<marathon><monsters><monster index=\"0\" vitality=\"10\"/></monsters></marathon>",
        );
        let cache = MmlCascadeCache::new(&[], &[], Some(entry_of(&scenario_wad)), &[]);

        // A level WAD entry with no MMLS tag at all.
        let bare = WadBuilder::new()
            .version(4)
            .add_entry(0, vec![TagData::new(WadTag::MapInfo, vec![0u8; 88])])
            .build();
        let bare_wad = WadFile::from_bytes(&bare).unwrap();

        let resolved = cache.with_level_mml(bare_wad.entry(0).unwrap());
        assert_eq!(monster_vitality(&resolved, "0").as_deref(), Some("10"));
    }

    // ── full cascade through WAD entries (scenario + level layers) ──

    #[test]
    fn test_full_cascade_scenario_and_level_via_wad() {
        let scenario_wad = wad_with_mml(
            b"<marathon><monsters><monster index=\"0\" vitality=\"30\"/></monsters></marathon>",
        );
        let level_wad = wad_with_mml(
            b"<marathon><monsters><monster index=\"0\" vitality=\"50\"/></monsters></marathon>",
        );
        let plugins = vec![PluginMmlSource::new(
            "Plug",
            b"<marathon><monsters><monster index=\"0\" vitality=\"40\"/></monsters></marathon>"
                .to_vec(),
        )];

        let result = assemble_mml_cascade(
            &[],
            &[],
            Some(entry_of(&scenario_wad)),
            &plugins,
            Some(entry_of(&level_wad)),
        );
        assert_eq!(
            monster_vitality(&result, "0").as_deref(),
            Some("50"),
            "level wins over plugin wins over scenario"
        );
    }

    // ── resilience: missing/malformed sources skipped, never panic ──

    #[test]
    fn test_missing_path_skipped() {
        let missing = PathBuf::from("/nonexistent/does/not/exist.mml");
        // Should not panic; the cascade simply yields an empty document.
        let result = assemble_mml_cascade(&[missing], &[], None, &[], None);
        assert!(result.is_empty());
    }

    #[test]
    fn test_malformed_plugin_skipped_others_survive() {
        let plugins = vec![
            PluginMmlSource::new(
                "Good",
                b"<marathon><monsters><monster index=\"0\" vitality=\"7\"/></monsters></marathon>"
                    .to_vec(),
            ),
            PluginMmlSource::new("Zbad", b"<marathon><weapons <<broken".to_vec()),
        ];
        let result = assemble_mml_cascade(&[], &[], None, &plugins, None);
        // Good plugin's override survives despite the malformed one.
        assert_eq!(monster_vitality(&result, "0").as_deref(), Some("7"));
    }

    #[test]
    fn test_empty_cascade_is_empty_document() {
        let result = assemble_mml_cascade(&[], &[], None, &[], None);
        assert!(result.is_empty());
    }
}
