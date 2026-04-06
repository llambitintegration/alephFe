use bitflags::bitflags;
use serde::{Deserialize, Serialize};

/// Top-level game state machine.
///
/// Transitions:
/// - Loading -> MainMenu (initial load complete)
/// - MainMenu -> Loading (start game / load save)
/// - Loading -> Playing (level ready)
/// - Playing <-> Paused
/// - Playing <-> Terminal
/// - Playing -> Intermission (level complete)
/// - Intermission -> Loading (next level)
/// - Playing -> GameOver (death / campaign end)
/// - GameOver -> MainMenu
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Loading,
    MainMenu,
    Playing,
    Paused,
    Terminal,
    Intermission,
    GameOver,
}

bitflags! {
    /// Per-tick action flags consumed by the simulation.
    ///
    /// Each flag represents a player action that is either active or inactive
    /// for a given simulation tick. Marathon's simulation reads these flags
    /// to advance player movement, weapon fire, and interactions.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct ActionFlags: u32 {
        const MOVE_FORWARD       = 1 << 0;
        const MOVE_BACKWARD      = 1 << 1;
        const STRAFE_LEFT        = 1 << 2;
        const STRAFE_RIGHT       = 1 << 3;
        const TURN_LEFT          = 1 << 4;
        const TURN_RIGHT         = 1 << 5;
        const LOOK_UP            = 1 << 6;
        const LOOK_DOWN          = 1 << 7;
        const FIRE_PRIMARY       = 1 << 8;
        const FIRE_SECONDARY     = 1 << 9;
        const ACTION             = 1 << 10;
        const CYCLE_WEAPON_FWD   = 1 << 11;
        const CYCLE_WEAPON_BACK  = 1 << 12;
        const TOGGLE_MAP         = 1 << 13;
        const MICROPHONE         = 1 << 14;
    }
}

/// Marathon difficulty levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Difficulty {
    Kindergarten,
    EasyStreet,
    Normal,
    MajorDamage,
    TotalCarnage,
}

/// Multiplayer game mode variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameModeType {
    Campaign,
    Cooperative,
    EveryManForHimself,
    KingOfTheHill,
    KillTheManWithTheBall,
    Tag,
}

/// Top-level game configuration set before starting a game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub difficulty: Difficulty,
    pub game_mode: GameModeType,
    /// Index of the starting level within the scenario WAD.
    pub starting_level: usize,
    /// Enable film recording for this session.
    pub record_film: bool,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            difficulty: Difficulty::Normal,
            game_mode: GameModeType::Campaign,
            starting_level: 0,
            record_film: false,
        }
    }
}
