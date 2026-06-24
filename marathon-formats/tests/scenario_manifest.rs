//! Scenario manifest parser (box 1.7).
//!
//! Parses `tests/scenarios.toml` into typed structs via `toml` + `serde`
//! and exposes the helpers the golden-data TDD framework builds on:
//! [`load_manifest`], [`levels_for_tier`], and [`source_path`].
//!
//! Representation note for the per-tier golden counts: the TOML uses a
//! dedicated top-level `[tier1.<level-id>]` table, deserialized into
//! `Manifest.tier1: HashMap<level-id, GoldenCounts>`. We deliberately
//! avoid `[levels.tier1.<id>]` because `[[levels]]` already makes
//! `levels` an array of tables -- a sibling `[levels.tier1]` would
//! attach to the LAST array element rather than forming a stable
//! top-level lookup. A separate `[tier1]` table keeps the two shapes
//! unambiguous and lets serde deserialize the whole manifest in one pass.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

/// An upstream data repository pinned to a commit.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Source {
    pub repo: String,
    pub commit: String,
}

/// A single golden level entry from `[[levels]]`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Level {
    pub id: String,
    pub source: String,
    pub wad_path: String,
    pub level_index: u32,
    pub name: String,
    #[serde(default)]
    pub tier: Option<String>,
    #[serde(default)]
    pub features: Vec<String>,
}

/// Golden geometry counts for a level at a given tier.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub struct GoldenCounts {
    pub endpoints: usize,
    pub lines: usize,
    pub polygons: usize,
}

/// The parsed scenario manifest.
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    /// Named source repos, keyed by source name (e.g. `"marathon-2"`).
    #[serde(default)]
    pub sources: HashMap<String, Source>,
    /// Golden level entries from the `[[levels]]` array.
    #[serde(default)]
    pub levels: Vec<Level>,
    /// Tier 1 golden geometry counts keyed by level id, from the
    /// dedicated top-level `[tier1.<id>]` table.
    #[serde(default)]
    pub tier1: HashMap<String, GoldenCounts>,
}

/// Parse a scenario manifest from a TOML string.
pub fn parse_manifest(text: &str) -> Result<Manifest, ManifestError> {
    Ok(toml::from_str(text)?)
}

/// Load and parse the scenario manifest at `path`.
pub fn load_manifest<P: AsRef<Path>>(path: P) -> Result<Manifest, ManifestError> {
    let text = std::fs::read_to_string(path.as_ref())?;
    parse_manifest(&text)
}

/// Return all levels whose `tier` field equals `tier`.
pub fn levels_for_tier<'a>(manifest: &'a Manifest, tier: &str) -> Vec<&'a Level> {
    manifest
        .levels
        .iter()
        .filter(|l| l.tier.as_deref() == Some(tier))
        .collect()
}

/// Return the repo path/name for a named source, or `None` if absent.
///
/// Returns the source's `repo` field (the on-disk fixtures subdir /
/// upstream repo name), which callers join under `tests/fixtures/`.
pub fn source_path(manifest: &Manifest, source_name: &str) -> Option<String> {
    manifest.sources.get(source_name).map(|s| s.repo.clone())
}

/// Errors from loading or parsing a scenario manifest.
#[derive(Debug)]
pub enum ManifestError {
    Io(std::io::Error),
    Toml(toml::de::Error),
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestError::Io(e) => write!(f, "manifest I/O error: {e}"),
            ManifestError::Toml(e) => write!(f, "manifest TOML parse error: {e}"),
        }
    }
}

impl std::error::Error for ManifestError {}

impl From<std::io::Error> for ManifestError {
    fn from(e: std::io::Error) -> Self {
        ManifestError::Io(e)
    }
}

impl From<toml::de::Error> for ManifestError {
    fn from(e: toml::de::Error) -> Self {
        ManifestError::Toml(e)
    }
}

// ───────────────────────────── Tests ──────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn manifest_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/scenarios.toml")
    }

    fn load() -> Manifest {
        load_manifest(manifest_path()).expect("scenarios.toml should load and parse")
    }

    #[test]
    fn load_manifest_parses_sources() {
        let m = load();
        let src = m
            .sources
            .get("marathon-2")
            .expect("marathon-2 source present");
        assert_eq!(src.repo, "data-marathon-2");
        assert_eq!(src.commit, "eaf21a7");
    }

    #[test]
    fn load_manifest_parses_levels() {
        let m = load();
        let waterloo = m
            .levels
            .iter()
            .find(|l| l.id == "m2-waterloo")
            .expect("waterloo level present");
        assert_eq!(waterloo.source, "marathon-2");
        assert_eq!(waterloo.wad_path, "Map");
        assert_eq!(waterloo.level_index, 0);
        assert_eq!(waterloo.name, "Waterloo Waterpark");
        assert_eq!(waterloo.tier.as_deref(), Some("tier1"));
        assert!(waterloo.features.contains(&"media".to_string()));
    }

    #[test]
    fn levels_for_tier_returns_tier1_levels() {
        let m = load();
        let tier1 = levels_for_tier(&m, "tier1");
        assert!(
            tier1.iter().any(|l| l.id == "m2-waterloo"),
            "tier1 should include Waterloo Waterpark"
        );
        // Every returned level must actually be tier1.
        assert!(tier1.iter().all(|l| l.tier.as_deref() == Some("tier1")));
    }

    #[test]
    fn source_path_returns_repo() {
        let m = load();
        assert_eq!(
            source_path(&m, "marathon-2").as_deref(),
            Some("data-marathon-2")
        );
        assert_eq!(source_path(&m, "does-not-exist"), None);
    }

    #[test]
    fn tier1_golden_counts_parse_correctly() {
        let m = load();
        let counts = m
            .tier1
            .get("m2-waterloo")
            .expect("golden counts for Waterloo present");
        assert_eq!(counts.endpoints, 716);
        assert_eq!(counts.lines, 1106);
        assert_eq!(counts.polygons, 369);
    }
}
