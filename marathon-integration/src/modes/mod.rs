mod campaign;
mod cooperative;
mod deathmatch;

pub use campaign::CampaignMode;
pub use cooperative::CooperativeMode;
pub use deathmatch::{EveryManForHimself, KillTheManWithTheBall, KingOfTheHill, TagMode};

/// Player score tracking.
#[derive(Debug, Clone, Default)]
pub struct PlayerScore {
    pub player_id: usize,
    pub kills: u32,
    pub deaths: u32,
    pub time_score: f64,
}

/// Spawn point definition.
#[derive(Debug, Clone)]
pub struct SpawnPoint {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub polygon_index: usize,
    pub facing: u16,
    pub team: Option<usize>,
}

/// Result of checking win conditions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WinCheckResult {
    /// No winner yet.
    InProgress,
    /// A specific player won.
    Winner(usize),
    /// Time limit expired, highest score wins.
    TimeLimitReached,
    /// Level completed (campaign/coop).
    LevelComplete,
}

/// Trait for game mode behavior.
pub trait GameMode {
    /// Update scores based on a game event.
    fn on_kill(&mut self, killer: usize, victim: usize);

    /// Check win conditions. Called each tick.
    fn check_win_condition(&self) -> WinCheckResult;

    /// Get the spawn point for a player entering the game.
    fn get_spawn_point(&self, player_id: usize, spawn_points: &[SpawnPoint]) -> Option<usize>;

    /// Respawn delay in ticks after death (0 = instant).
    fn respawn_delay(&self) -> u32;

    /// Get current scores for all players.
    fn scores(&self) -> &[PlayerScore];
}
