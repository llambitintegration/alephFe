use crate::types::Difficulty;

/// Statistics displayed on the intermission screen after completing a level.
#[derive(Debug, Clone)]
pub struct LevelStats {
    /// Level index that was just completed.
    pub level_index: usize,
    /// Difficulty setting.
    pub difficulty: Difficulty,
    /// Total kills achieved by the player.
    pub kills: u32,
    /// Total monsters present on the level.
    pub total_monsters: u32,
    /// Items collected by the player.
    pub items_collected: u32,
    /// Total items available on the level.
    pub total_items: u32,
    /// Secrets found by the player.
    pub secrets_found: u32,
    /// Total secrets available on the level.
    pub total_secrets: u32,
    /// Time spent on the level in simulation ticks.
    pub ticks_elapsed: u64,
}

impl LevelStats {
    /// Kill percentage (0..100).
    pub fn kill_percentage(&self) -> u32 {
        if self.total_monsters == 0 {
            100
        } else {
            ((self.kills as u64 * 100) / self.total_monsters as u64) as u32
        }
    }

    /// Item collection percentage (0..100).
    pub fn item_percentage(&self) -> u32 {
        if self.total_items == 0 {
            100
        } else {
            ((self.items_collected as u64 * 100) / self.total_items as u64) as u32
        }
    }

    /// Secret discovery percentage (0..100).
    pub fn secret_percentage(&self) -> u32 {
        if self.total_secrets == 0 {
            100
        } else {
            ((self.secrets_found as u64 * 100) / self.total_secrets as u64) as u32
        }
    }

    /// Format elapsed time as "MM:SS".
    pub fn formatted_time(&self) -> String {
        let total_seconds = self.ticks_elapsed / crate::shell::TICKS_PER_SECOND as u64;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{minutes}:{seconds:02}")
    }
}

/// State of the intermission screen.
#[derive(Debug)]
pub struct IntermissionScreen {
    /// Level completion statistics.
    pub stats: LevelStats,
    /// Target level to load when the player advances.
    pub next_level: usize,
    /// Whether the player has pressed a key to continue.
    pub ready_to_advance: bool,
}

impl IntermissionScreen {
    /// Create a new intermission screen.
    pub fn new(stats: LevelStats, next_level: usize) -> Self {
        Self {
            stats,
            next_level,
            ready_to_advance: false,
        }
    }

    /// Mark the player as ready to advance to the next level.
    pub fn advance(&mut self) {
        self.ready_to_advance = true;
    }

    /// Whether the screen should transition to loading the next level.
    pub fn should_load_next(&self) -> bool {
        self.ready_to_advance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kill_percentage_all_killed() {
        let stats = LevelStats {
            level_index: 0,
            difficulty: Difficulty::Normal,
            kills: 20,
            total_monsters: 20,
            items_collected: 0,
            total_items: 0,
            secrets_found: 0,
            total_secrets: 0,
            ticks_elapsed: 0,
        };
        assert_eq!(stats.kill_percentage(), 100);
    }

    #[test]
    fn kill_percentage_partial() {
        let stats = LevelStats {
            level_index: 0,
            difficulty: Difficulty::Normal,
            kills: 15,
            total_monsters: 20,
            items_collected: 0,
            total_items: 0,
            secrets_found: 0,
            total_secrets: 0,
            ticks_elapsed: 0,
        };
        assert_eq!(stats.kill_percentage(), 75);
    }

    #[test]
    fn kill_percentage_no_monsters() {
        let stats = LevelStats {
            level_index: 0,
            difficulty: Difficulty::Normal,
            kills: 0,
            total_monsters: 0,
            items_collected: 0,
            total_items: 0,
            secrets_found: 0,
            total_secrets: 0,
            ticks_elapsed: 0,
        };
        assert_eq!(stats.kill_percentage(), 100);
    }

    #[test]
    fn formatted_time_displays_correctly() {
        let stats = LevelStats {
            level_index: 0,
            difficulty: Difficulty::Normal,
            kills: 0,
            total_monsters: 0,
            items_collected: 0,
            total_items: 0,
            secrets_found: 0,
            total_secrets: 0,
            ticks_elapsed: 30 * 125, // 125 seconds = 2:05
        };
        assert_eq!(stats.formatted_time(), "2:05");
    }

    #[test]
    fn intermission_advance() {
        let stats = LevelStats {
            level_index: 2,
            difficulty: Difficulty::MajorDamage,
            kills: 10,
            total_monsters: 10,
            items_collected: 5,
            total_items: 8,
            secrets_found: 1,
            total_secrets: 3,
            ticks_elapsed: 3000,
        };
        let mut screen = IntermissionScreen::new(stats, 3);
        assert!(!screen.should_load_next());
        screen.advance();
        assert!(screen.should_load_next());
        assert_eq!(screen.next_level, 3);
    }
}
