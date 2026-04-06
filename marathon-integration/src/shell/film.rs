use serde::{Deserialize, Serialize};

use crate::types::{ActionFlags, Difficulty, GameModeType};

/// Film file header containing metadata for replay initialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilmHeader {
    /// Format version.
    pub version: u32,
    /// Level index within the scenario WAD.
    pub level_index: usize,
    /// Difficulty setting used during recording.
    pub difficulty: Difficulty,
    /// Game mode used during recording.
    pub game_mode: GameModeType,
    /// Random seed used to initialize the simulation.
    pub random_seed: u64,
}

/// A complete film recording: header plus per-tick action flags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilmData {
    pub header: FilmHeader,
    /// Action flags for each simulation tick, in order.
    pub ticks: Vec<ActionFlags>,
}

/// Current film format version.
const FILM_VERSION: u32 = 1;

/// Records action flags during gameplay for later replay.
pub struct FilmRecorder {
    header: FilmHeader,
    ticks: Vec<ActionFlags>,
    recording: bool,
}

impl FilmRecorder {
    /// Start a new recording session.
    pub fn new(
        level_index: usize,
        difficulty: Difficulty,
        game_mode: GameModeType,
        random_seed: u64,
    ) -> Self {
        Self {
            header: FilmHeader {
                version: FILM_VERSION,
                level_index,
                difficulty,
                game_mode,
                random_seed,
            },
            ticks: Vec::new(),
            recording: true,
        }
    }

    /// Record one tick's action flags.
    pub fn record_tick(&mut self, flags: ActionFlags) {
        if self.recording {
            self.ticks.push(flags);
        }
    }

    /// Stop recording and return the completed film data.
    pub fn finish(mut self) -> FilmData {
        self.recording = false;
        FilmData {
            header: self.header,
            ticks: self.ticks,
        }
    }

    /// Number of ticks recorded so far.
    pub fn tick_count(&self) -> usize {
        self.ticks.len()
    }

    /// Whether the recorder is actively recording.
    pub fn is_recording(&self) -> bool {
        self.recording
    }
}

/// Plays back a recorded film by providing action flags per tick.
pub struct FilmPlayer {
    film: FilmData,
    current_tick: usize,
}

impl FilmPlayer {
    /// Create a new player from film data.
    pub fn new(film: FilmData) -> Self {
        Self {
            film,
            current_tick: 0,
        }
    }

    /// Get the film header (for initializing the level).
    pub fn header(&self) -> &FilmHeader {
        &self.film.header
    }

    /// Get the action flags for the next tick, advancing the playhead.
    /// Returns None when the film is exhausted.
    pub fn next_tick(&mut self) -> Option<ActionFlags> {
        if self.current_tick < self.film.ticks.len() {
            let flags = self.film.ticks[self.current_tick];
            self.current_tick += 1;
            Some(flags)
        } else {
            None
        }
    }

    /// Whether playback has finished (all ticks consumed).
    pub fn is_finished(&self) -> bool {
        self.current_tick >= self.film.ticks.len()
    }

    /// Current playback position.
    pub fn current_tick(&self) -> usize {
        self.current_tick
    }

    /// Total number of ticks in the film.
    pub fn total_ticks(&self) -> usize {
        self.film.ticks.len()
    }
}

/// Serialize film data to bytes for writing to disk.
pub fn serialize_film(film: &FilmData) -> Result<Vec<u8>, FilmError> {
    bincode::serialize(film).map_err(|e| FilmError::SerializeError(e.to_string()))
}

/// Deserialize film data from bytes.
pub fn deserialize_film(bytes: &[u8]) -> Result<FilmData, FilmError> {
    let film: FilmData =
        bincode::deserialize(bytes).map_err(|e| FilmError::DeserializeError(e.to_string()))?;

    if film.header.version != FILM_VERSION {
        return Err(FilmError::VersionMismatch {
            expected: FILM_VERSION,
            found: film.header.version,
        });
    }

    Ok(film)
}

/// Errors during film operations.
#[derive(Debug, thiserror::Error)]
pub enum FilmError {
    #[error("Film serialization error: {0}")]
    SerializeError(String),

    #[error("Film deserialization error: {0}")]
    DeserializeError(String),

    #[error("Film version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: u32, found: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_playback() {
        let mut recorder = FilmRecorder::new(0, Difficulty::Normal, GameModeType::Campaign, 42);

        recorder.record_tick(ActionFlags::MOVE_FORWARD);
        recorder.record_tick(ActionFlags::MOVE_FORWARD | ActionFlags::FIRE_PRIMARY);
        recorder.record_tick(ActionFlags::empty());

        assert_eq!(recorder.tick_count(), 3);
        assert!(recorder.is_recording());

        let film = recorder.finish();
        let mut player = FilmPlayer::new(film);

        assert_eq!(player.header().level_index, 0);
        assert_eq!(player.header().random_seed, 42);
        assert_eq!(player.total_ticks(), 3);

        assert_eq!(player.next_tick(), Some(ActionFlags::MOVE_FORWARD));
        assert_eq!(
            player.next_tick(),
            Some(ActionFlags::MOVE_FORWARD | ActionFlags::FIRE_PRIMARY)
        );
        assert_eq!(player.next_tick(), Some(ActionFlags::empty()));
        assert_eq!(player.next_tick(), None);
        assert!(player.is_finished());
    }

    #[test]
    fn serialize_deserialize_round_trip() {
        let mut recorder = FilmRecorder::new(5, Difficulty::TotalCarnage, GameModeType::Campaign, 12345);
        recorder.record_tick(ActionFlags::STRAFE_LEFT | ActionFlags::FIRE_SECONDARY);
        recorder.record_tick(ActionFlags::ACTION);
        let film = recorder.finish();

        let bytes = serialize_film(&film).unwrap();
        let restored = deserialize_film(&bytes).unwrap();

        assert_eq!(restored.header.level_index, 5);
        assert_eq!(restored.header.difficulty, Difficulty::TotalCarnage);
        assert_eq!(restored.header.random_seed, 12345);
        assert_eq!(restored.ticks.len(), 2);
        assert_eq!(
            restored.ticks[0],
            ActionFlags::STRAFE_LEFT | ActionFlags::FIRE_SECONDARY
        );
    }

    #[test]
    fn empty_film() {
        let recorder = FilmRecorder::new(0, Difficulty::Normal, GameModeType::Campaign, 0);
        let film = recorder.finish();
        let mut player = FilmPlayer::new(film);
        assert!(player.is_finished());
        assert_eq!(player.next_tick(), None);
    }
}
