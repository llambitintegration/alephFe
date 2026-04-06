use super::{GameMode, PlayerScore, SpawnPoint, WinCheckResult};

/// Every Man for Himself (free-for-all deathmatch).
pub struct EveryManForHimself {
    scores: Vec<PlayerScore>,
    kill_limit: u32,
}

impl EveryManForHimself {
    pub fn new(num_players: usize, kill_limit: u32) -> Self {
        let scores = (0..num_players)
            .map(|id| PlayerScore {
                player_id: id,
                ..Default::default()
            })
            .collect();
        Self { scores, kill_limit }
    }
}

impl GameMode for EveryManForHimself {
    fn on_kill(&mut self, killer: usize, victim: usize) {
        if let Some(score) = self.scores.get_mut(killer) {
            score.kills += 1;
        }
        if let Some(score) = self.scores.get_mut(victim) {
            score.deaths += 1;
        }
    }

    fn check_win_condition(&self) -> WinCheckResult {
        for score in &self.scores {
            if score.kills >= self.kill_limit {
                return WinCheckResult::Winner(score.player_id);
            }
        }
        WinCheckResult::InProgress
    }

    fn get_spawn_point(&self, _player_id: usize, spawn_points: &[SpawnPoint]) -> Option<usize> {
        if spawn_points.is_empty() {
            None
        } else {
            // Simple: pick a random spawn point (deterministic seed in practice)
            Some(0)
        }
    }

    fn respawn_delay(&self) -> u32 {
        90 // 3 seconds
    }

    fn scores(&self) -> &[PlayerScore] {
        &self.scores
    }
}

/// King of the Hill: timed zone control.
pub struct KingOfTheHill {
    scores: Vec<PlayerScore>,
    /// Polygon index of the current hill.
    hill_polygon: usize,
    /// Time limit in seconds for winning.
    time_limit: f64,
}

impl KingOfTheHill {
    pub fn new(num_players: usize, hill_polygon: usize, time_limit: f64) -> Self {
        let scores = (0..num_players)
            .map(|id| PlayerScore {
                player_id: id,
                ..Default::default()
            })
            .collect();
        Self {
            scores,
            hill_polygon,
            time_limit,
        }
    }

    pub fn hill_polygon(&self) -> usize {
        self.hill_polygon
    }

    /// Award time to a player standing on the hill. Call each tick.
    pub fn award_hill_time(&mut self, player_id: usize, seconds: f64) {
        if let Some(score) = self.scores.get_mut(player_id) {
            score.time_score += seconds;
        }
    }
}

impl GameMode for KingOfTheHill {
    fn on_kill(&mut self, killer: usize, victim: usize) {
        if let Some(score) = self.scores.get_mut(killer) {
            score.kills += 1;
        }
        if let Some(score) = self.scores.get_mut(victim) {
            score.deaths += 1;
        }
    }

    fn check_win_condition(&self) -> WinCheckResult {
        for score in &self.scores {
            if score.time_score >= self.time_limit {
                return WinCheckResult::Winner(score.player_id);
            }
        }
        WinCheckResult::InProgress
    }

    fn get_spawn_point(&self, _player_id: usize, spawn_points: &[SpawnPoint]) -> Option<usize> {
        if spawn_points.is_empty() {
            None
        } else {
            Some(0)
        }
    }

    fn respawn_delay(&self) -> u32 {
        90
    }

    fn scores(&self) -> &[PlayerScore] {
        &self.scores
    }
}

/// Kill the Man with the Ball: possession-based scoring.
pub struct KillTheManWithTheBall {
    scores: Vec<PlayerScore>,
    /// Which player currently holds the ball (None if on ground).
    ball_holder: Option<usize>,
    /// Time limit in seconds.
    time_limit: f64,
}

impl KillTheManWithTheBall {
    pub fn new(num_players: usize, time_limit: f64) -> Self {
        let scores = (0..num_players)
            .map(|id| PlayerScore {
                player_id: id,
                ..Default::default()
            })
            .collect();
        Self {
            scores,
            ball_holder: None,
            time_limit,
        }
    }

    pub fn ball_holder(&self) -> Option<usize> {
        self.ball_holder
    }

    /// Player picks up the ball.
    pub fn pickup_ball(&mut self, player_id: usize) {
        self.ball_holder = Some(player_id);
    }

    /// Ball is dropped (player died or dropped it).
    pub fn drop_ball(&mut self) {
        self.ball_holder = None;
    }

    /// Award time to the ball holder. Call each tick.
    pub fn award_possession_time(&mut self, seconds: f64) {
        if let Some(holder) = self.ball_holder {
            if let Some(score) = self.scores.get_mut(holder) {
                score.time_score += seconds;
            }
        }
    }
}

impl GameMode for KillTheManWithTheBall {
    fn on_kill(&mut self, killer: usize, victim: usize) {
        if let Some(score) = self.scores.get_mut(killer) {
            score.kills += 1;
        }
        if let Some(score) = self.scores.get_mut(victim) {
            score.deaths += 1;
        }
        // If the victim had the ball, drop it
        if self.ball_holder == Some(victim) {
            self.ball_holder = None;
        }
    }

    fn check_win_condition(&self) -> WinCheckResult {
        for score in &self.scores {
            if score.time_score >= self.time_limit {
                return WinCheckResult::Winner(score.player_id);
            }
        }
        WinCheckResult::InProgress
    }

    fn get_spawn_point(&self, _player_id: usize, spawn_points: &[SpawnPoint]) -> Option<usize> {
        if spawn_points.is_empty() {
            None
        } else {
            Some(0)
        }
    }

    fn respawn_delay(&self) -> u32 {
        90
    }

    fn scores(&self) -> &[PlayerScore] {
        &self.scores
    }
}

/// Tag: tagged player accumulates score.
pub struct TagMode {
    scores: Vec<PlayerScore>,
    /// Which player is currently "it".
    tagged_player: Option<usize>,
    /// Time limit in seconds.
    time_limit: f64,
}

impl TagMode {
    pub fn new(num_players: usize, time_limit: f64) -> Self {
        let scores = (0..num_players)
            .map(|id| PlayerScore {
                player_id: id,
                ..Default::default()
            })
            .collect();
        Self {
            scores,
            tagged_player: None,
            time_limit,
        }
    }

    pub fn tagged_player(&self) -> Option<usize> {
        self.tagged_player
    }

    /// Set initial tagged player.
    pub fn set_tagged(&mut self, player_id: usize) {
        self.tagged_player = Some(player_id);
    }

    /// Award time to the tagged player. Call each tick.
    pub fn award_tag_time(&mut self, seconds: f64) {
        if let Some(tagged) = self.tagged_player {
            if let Some(score) = self.scores.get_mut(tagged) {
                score.time_score += seconds;
            }
        }
    }
}

impl GameMode for TagMode {
    fn on_kill(&mut self, killer: usize, victim: usize) {
        if let Some(score) = self.scores.get_mut(killer) {
            score.kills += 1;
        }
        if let Some(score) = self.scores.get_mut(victim) {
            score.deaths += 1;
        }
        // Tag transfers on kill: if the tagged player kills someone,
        // or if someone kills the tagged player
        if self.tagged_player == Some(victim) {
            self.tagged_player = Some(killer);
        }
    }

    fn check_win_condition(&self) -> WinCheckResult {
        for score in &self.scores {
            if score.time_score >= self.time_limit {
                return WinCheckResult::Winner(score.player_id);
            }
        }
        WinCheckResult::InProgress
    }

    fn get_spawn_point(&self, _player_id: usize, spawn_points: &[SpawnPoint]) -> Option<usize> {
        if spawn_points.is_empty() {
            None
        } else {
            Some(0)
        }
    }

    fn respawn_delay(&self) -> u32 {
        90
    }

    fn scores(&self) -> &[PlayerScore] {
        &self.scores
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deathmatch_kill_scoring() {
        let mut mode = EveryManForHimself::new(4, 10);
        mode.on_kill(0, 1);
        mode.on_kill(0, 2);
        assert_eq!(mode.scores()[0].kills, 2);
        assert_eq!(mode.scores()[1].deaths, 1);
        assert_eq!(mode.check_win_condition(), WinCheckResult::InProgress);
    }

    #[test]
    fn deathmatch_win_on_kill_limit() {
        let mut mode = EveryManForHimself::new(2, 3);
        mode.on_kill(0, 1);
        mode.on_kill(0, 1);
        mode.on_kill(0, 1);
        assert_eq!(mode.check_win_condition(), WinCheckResult::Winner(0));
    }

    #[test]
    fn koth_time_scoring() {
        let mut mode = KingOfTheHill::new(2, 5, 60.0);
        mode.award_hill_time(0, 30.0);
        assert_eq!(mode.check_win_condition(), WinCheckResult::InProgress);

        mode.award_hill_time(0, 31.0);
        assert_eq!(mode.check_win_condition(), WinCheckResult::Winner(0));
    }

    #[test]
    fn ktmwtb_ball_possession() {
        let mut mode = KillTheManWithTheBall::new(3, 120.0);
        assert_eq!(mode.ball_holder(), None);

        mode.pickup_ball(1);
        assert_eq!(mode.ball_holder(), Some(1));

        mode.award_possession_time(10.0);
        assert!((mode.scores()[1].time_score - 10.0).abs() < f64::EPSILON);

        // Kill ball holder drops ball
        mode.on_kill(2, 1);
        assert_eq!(mode.ball_holder(), None);
    }

    #[test]
    fn tag_transfer_on_kill() {
        let mut mode = TagMode::new(3, 120.0);
        mode.set_tagged(1);

        mode.award_tag_time(5.0);
        assert!((mode.scores()[1].time_score - 5.0).abs() < f64::EPSILON);

        // Player 2 kills tagged player 1 -> tag transfers
        mode.on_kill(2, 1);
        assert_eq!(mode.tagged_player(), Some(2));
    }
}
