use super::{GameMode, PlayerScore, SpawnPoint, WinCheckResult};

/// Single-player campaign mode.
pub struct CampaignMode {
    /// Current level index.
    current_level: usize,
    /// Total number of levels in the scenario.
    total_levels: usize,
    /// Whether the current level has been completed.
    level_complete: bool,
    /// Single player score.
    scores: Vec<PlayerScore>,
}

impl CampaignMode {
    pub fn new(starting_level: usize, total_levels: usize) -> Self {
        Self {
            current_level: starting_level,
            total_levels,
            level_complete: false,
            scores: vec![PlayerScore {
                player_id: 0,
                ..Default::default()
            }],
        }
    }

    pub fn current_level(&self) -> usize {
        self.current_level
    }

    pub fn advance_level(&mut self) -> Option<usize> {
        let next = self.current_level + 1;
        if next < self.total_levels {
            self.current_level = next;
            self.level_complete = false;
            Some(next)
        } else {
            None
        }
    }

    pub fn mark_level_complete(&mut self) {
        self.level_complete = true;
    }
}

impl GameMode for CampaignMode {
    fn on_kill(&mut self, _killer: usize, _victim: usize) {
        self.scores[0].kills += 1;
    }

    fn check_win_condition(&self) -> WinCheckResult {
        if self.level_complete {
            WinCheckResult::LevelComplete
        } else {
            WinCheckResult::InProgress
        }
    }

    fn get_spawn_point(&self, _player_id: usize, spawn_points: &[SpawnPoint]) -> Option<usize> {
        // Campaign: use first available spawn point
        if spawn_points.is_empty() {
            None
        } else {
            Some(0)
        }
    }

    fn respawn_delay(&self) -> u32 {
        0
    }

    fn scores(&self) -> &[PlayerScore] {
        &self.scores
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_progression() {
        let mut mode = CampaignMode::new(0, 5);
        assert_eq!(mode.current_level(), 0);
        assert_eq!(mode.check_win_condition(), WinCheckResult::InProgress);

        mode.mark_level_complete();
        assert_eq!(mode.check_win_condition(), WinCheckResult::LevelComplete);

        let next = mode.advance_level();
        assert_eq!(next, Some(1));
        assert_eq!(mode.check_win_condition(), WinCheckResult::InProgress);
    }

    #[test]
    fn last_level_no_advance() {
        let mut mode = CampaignMode::new(4, 5);
        assert_eq!(mode.advance_level(), None);
    }

    #[test]
    fn kill_tracking() {
        let mut mode = CampaignMode::new(0, 5);
        mode.on_kill(0, 1);
        mode.on_kill(0, 2);
        assert_eq!(mode.scores()[0].kills, 2);
    }
}
