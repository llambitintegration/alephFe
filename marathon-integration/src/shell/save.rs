use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::types::{Difficulty, GameConfig, GameModeType};

/// Maximum number of save slots.
pub const MAX_SAVE_SLOTS: usize = 10;

/// Serializable save game data.
///
/// Contains everything needed to restore a game session:
/// the level, difficulty, game mode, and serialized simulation state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    /// Format version for forward compatibility.
    pub version: u32,
    /// Index of the current level.
    pub level_index: usize,
    /// Difficulty setting.
    pub difficulty: Difficulty,
    /// Game mode.
    pub game_mode: GameModeType,
    /// Which terminals have been read (by terminal index).
    pub terminals_read: Vec<usize>,
    /// Opaque serialized simulation state from marathon-sim.
    /// Stored as raw bytes to decouple save format from sim internals.
    pub sim_state: Vec<u8>,
}

/// Metadata for a save slot (shown in the load game screen).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveSlotInfo {
    pub slot_index: usize,
    pub level_index: usize,
    pub difficulty: Difficulty,
    pub game_mode: GameModeType,
}

/// Current save format version.
const SAVE_VERSION: u32 = 1;

/// Manages save/load operations for save slots on disk.
pub struct SaveManager {
    save_dir: PathBuf,
}

impl SaveManager {
    /// Create a new save manager using the given directory for save files.
    pub fn new(save_dir: PathBuf) -> Self {
        Self { save_dir }
    }

    /// Get the file path for a given slot index.
    fn slot_path(&self, slot: usize) -> PathBuf {
        self.save_dir.join(format!("save_{slot}.bin"))
    }

    /// List all occupied save slots with their metadata.
    pub fn list_slots(&self) -> Vec<Option<SaveSlotInfo>> {
        (0..MAX_SAVE_SLOTS)
            .map(|slot| {
                let path = self.slot_path(slot);
                if path.exists() {
                    self.read_slot_info(slot).ok()
                } else {
                    None
                }
            })
            .collect()
    }

    /// Read just the metadata from a save slot (without full sim state).
    fn read_slot_info(&self, slot: usize) -> Result<SaveSlotInfo, SaveError> {
        let data = self.load(slot)?;
        Ok(SaveSlotInfo {
            slot_index: slot,
            level_index: data.level_index,
            difficulty: data.difficulty,
            game_mode: data.game_mode,
        })
    }

    /// Save game data to a slot.
    pub fn save(&self, slot: usize, data: &SaveData) -> Result<(), SaveError> {
        if slot >= MAX_SAVE_SLOTS {
            return Err(SaveError::InvalidSlot(slot));
        }

        fs::create_dir_all(&self.save_dir).map_err(|e| SaveError::IoError(e.to_string()))?;

        let encoded =
            bincode::serialize(data).map_err(|e| SaveError::SerializeError(e.to_string()))?;

        fs::write(self.slot_path(slot), encoded)
            .map_err(|e| SaveError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Load game data from a slot.
    pub fn load(&self, slot: usize) -> Result<SaveData, SaveError> {
        if slot >= MAX_SAVE_SLOTS {
            return Err(SaveError::InvalidSlot(slot));
        }

        let path = self.slot_path(slot);
        let bytes = fs::read(&path).map_err(|e| SaveError::IoError(e.to_string()))?;

        let data: SaveData =
            bincode::deserialize(&bytes).map_err(|e| SaveError::DeserializeError(e.to_string()))?;

        if data.version != SAVE_VERSION {
            return Err(SaveError::VersionMismatch {
                expected: SAVE_VERSION,
                found: data.version,
            });
        }

        Ok(data)
    }

    /// Create a SaveData from current game state.
    pub fn create_save_data(
        level_index: usize,
        config: &GameConfig,
        terminals_read: Vec<usize>,
        sim_state: Vec<u8>,
    ) -> SaveData {
        SaveData {
            version: SAVE_VERSION,
            level_index,
            difficulty: config.difficulty,
            game_mode: config.game_mode,
            terminals_read,
            sim_state,
        }
    }
}

/// Errors during save/load operations.
#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    #[error("Invalid save slot: {0} (max {MAX_SAVE_SLOTS})")]
    InvalidSlot(usize),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Serialization error: {0}")]
    SerializeError(String),

    #[error("Deserialization error: {0}")]
    DeserializeError(String),

    #[error("Save version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: u32, found: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_save_dir() -> PathBuf {
        let mut dir = env::temp_dir();
        dir.push(format!("marathon_test_saves_{}", rand::random::<u32>()));
        dir
    }

    fn sample_save_data() -> SaveData {
        SaveData {
            version: SAVE_VERSION,
            level_index: 3,
            difficulty: Difficulty::Normal,
            game_mode: GameModeType::Campaign,
            terminals_read: vec![0, 2, 5],
            sim_state: vec![1, 2, 3, 4, 5],
        }
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = temp_save_dir();
        let mgr = SaveManager::new(dir.clone());
        let data = sample_save_data();

        mgr.save(0, &data).unwrap();
        let loaded = mgr.load(0).unwrap();

        assert_eq!(loaded.level_index, 3);
        assert_eq!(loaded.difficulty, Difficulty::Normal);
        assert_eq!(loaded.terminals_read, vec![0, 2, 5]);
        assert_eq!(loaded.sim_state, vec![1, 2, 3, 4, 5]);

        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_slot_rejected() {
        let dir = temp_save_dir();
        let mgr = SaveManager::new(dir);
        let data = sample_save_data();

        assert!(mgr.save(MAX_SAVE_SLOTS, &data).is_err());
        assert!(mgr.load(MAX_SAVE_SLOTS).is_err());
    }

    #[test]
    fn load_nonexistent_slot_fails() {
        let dir = temp_save_dir();
        let mgr = SaveManager::new(dir);
        assert!(mgr.load(0).is_err());
    }

    #[test]
    fn list_slots_with_saves() {
        let dir = temp_save_dir();
        let mgr = SaveManager::new(dir.clone());
        let data = sample_save_data();

        mgr.save(2, &data).unwrap();
        let slots = mgr.list_slots();

        assert!(slots[0].is_none());
        assert!(slots[1].is_none());
        assert!(slots[2].is_some());
        assert_eq!(slots[2].as_ref().unwrap().level_index, 3);

        let _ = fs::remove_dir_all(&dir);
    }
}
