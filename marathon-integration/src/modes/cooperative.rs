use super::{GameMode, PlayerScore, SpawnPoint, WinCheckResult};

/// Cooperative multiplayer mode: multiple players, shared campaign progression.
pub struct CooperativeMode {
    /// Current level index.
    current_level: usize,
    /// Total levels.
    total_levels: usize,
    /// Level completed flag.
    level_complete: bool,
    /// Per-player scores.
    scores: Vec<PlayerScore>,
    /// Respawn delay in ticks.
    respawn_delay_ticks: u32,
}

impl CooperativeMode {
    pub fn new(starting_level: usize, total_levels: usize, num_players: usize) -> Self {
        let scores = (0..num_players)
            .map(|id| PlayerScore {
                player_id: id,
                ..Default::default()
            })
            .collect();

        Self {
            current_level: starting_level,
            total_levels,
            level_complete: false,
            scores,
            respawn_delay_ticks: 150, // 5 seconds at 30 ticks/sec
        }
    }

    pub fn mark_level_complete(&mut self) {
        self.level_complete = true;
    }
}

impl GameMode for CooperativeMode {
    fn on_kill(&mut self, killer: usize, _victim: usize) {
        if let Some(score) = self.scores.get_mut(killer) {
            score.kills += 1;
        }
    }

    fn check_win_condition(&self) -> WinCheckResult {
        if self.level_complete {
            WinCheckResult::LevelComplete
        } else {
            WinCheckResult::InProgress
        }
    }

    fn get_spawn_point(&self, _player_id: usize, spawn_points: &[SpawnPoint]) -> Option<usize> {
        // Cooperative: use team spawn points if available
        spawn_points
            .iter()
            .position(|sp| sp.team.is_some())
            .or(if spawn_points.is_empty() { None } else { Some(0) })
    }

    fn respawn_delay(&self) -> u32 {
        self.respawn_delay_ticks
    }

    fn scores(&self) -> &[PlayerScore] {
        &self.scores
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooperative_respawn_delay() {
        let mode = CooperativeMode::new(0, 5, 2);
        assert_eq!(mode.respawn_delay(), 150);
    }

    #[test]
    fn cooperative_kill_tracking() {
        let mut mode = CooperativeMode::new(0, 5, 3);
        mode.on_kill(1, 0);
        assert_eq!(mode.scores()[1].kills, 1);
        assert_eq!(mode.scores()[0].kills, 0);
    }
}
