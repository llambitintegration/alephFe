use marathon_formats::{MapData, WadFile};

/// Data needed to load and initialize a level.
#[derive(Debug)]
pub struct LevelLoadRequest {
    /// Index of the level within the scenario WAD.
    pub level_index: usize,
}

/// Result of loading a level's map data from the WAD file.
pub struct LoadedLevel {
    /// Parsed map geometry and metadata.
    pub map_data: MapData,
    /// The level index that was loaded.
    pub level_index: usize,
}

/// Load a level's map data from a WAD file.
///
/// Reads the WAD entry at the given index and parses it into MapData.
pub fn load_level_map(wad: &WadFile, level_index: usize) -> Result<LoadedLevel, LevelLoadError> {
    let entries = wad.entries();
    let entry = entries
        .get(level_index)
        .ok_or(LevelLoadError::InvalidLevelIndex {
            index: level_index,
            total: entries.len(),
        })?;

    let map_data =
        MapData::from_entry(entry).map_err(|e| LevelLoadError::MapParseFailed {
            level_index,
            message: e.to_string(),
        })?;

    Ok(LoadedLevel {
        map_data,
        level_index,
    })
}

/// Detect if the player has triggered a level transition.
///
/// Returns the target level index if a transition was triggered, or None.
pub fn check_level_transition(
    _map_data: &MapData,
    _player_polygon: usize,
) -> Option<LevelTransition> {
    // TODO: Check inter-level teleporter polygons and terminal teleports
    // once marathon-sim provides the necessary state queries.
    None
}

/// A level transition event.
#[derive(Debug, Clone)]
pub struct LevelTransition {
    /// Target level index to load.
    pub target_level: usize,
    /// Whether this transition came from a terminal (vs. a teleporter polygon).
    pub from_terminal: bool,
}

/// Errors during level loading.
#[derive(Debug, thiserror::Error)]
pub enum LevelLoadError {
    #[error("Level index {index} out of range (WAD has {total} entries)")]
    InvalidLevelIndex { index: usize, total: usize },

    #[error("Failed to parse map data for level {level_index}: {message}")]
    MapParseFailed { level_index: usize, message: String },
}
