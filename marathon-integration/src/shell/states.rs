use crate::types::GameState;

/// Marathon's simulation tick rate (30 ticks per second).
pub const TICKS_PER_SECOND: u32 = 30;

/// Duration of one simulation tick in seconds.
pub const TICK_DURATION_SECS: f64 = 1.0 / TICKS_PER_SECOND as f64;

/// Duration of one simulation tick in microseconds.
pub const TICK_DURATION_MICROS: u64 = 1_000_000 / TICKS_PER_SECOND as u64;

/// Tracks fixed-tick timing and accumulation for the main loop.
#[derive(Debug)]
pub struct TickAccumulator {
    /// Accumulated time in microseconds since last tick.
    accumulated_micros: u64,
}

impl TickAccumulator {
    pub fn new() -> Self {
        Self {
            accumulated_micros: 0,
        }
    }

    /// Add elapsed time and return the number of ticks to run.
    pub fn accumulate(&mut self, elapsed_micros: u64) -> u32 {
        self.accumulated_micros += elapsed_micros;
        let ticks = (self.accumulated_micros / TICK_DURATION_MICROS) as u32;
        self.accumulated_micros %= TICK_DURATION_MICROS;
        ticks
    }

    /// Interpolation factor (0.0 to 1.0) for rendering between ticks.
    pub fn interpolation_factor(&self) -> f64 {
        self.accumulated_micros as f64 / TICK_DURATION_MICROS as f64
    }
}

impl Default for TickAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

/// Check whether a transition between two game states is valid.
pub fn is_valid_transition(from: GameState, to: GameState) -> bool {
    matches!(
        (from, to),
        (GameState::Loading, GameState::MainMenu)
            | (GameState::Loading, GameState::Playing)
            | (GameState::MainMenu, GameState::Loading)
            | (GameState::Playing, GameState::Paused)
            | (GameState::Paused, GameState::Playing)
            | (GameState::Paused, GameState::MainMenu)
            | (GameState::Playing, GameState::Terminal)
            | (GameState::Terminal, GameState::Playing)
            | (GameState::Terminal, GameState::Intermission)
            | (GameState::Playing, GameState::Intermission)
            | (GameState::Intermission, GameState::Loading)
            | (GameState::Playing, GameState::GameOver)
            | (GameState::GameOver, GameState::MainMenu)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulate_one_tick() {
        let mut acc = TickAccumulator::new();
        // One tick = 33333 micros
        let ticks = acc.accumulate(33_334);
        assert_eq!(ticks, 1);
    }

    #[test]
    fn accumulate_multiple_ticks() {
        let mut acc = TickAccumulator::new();
        // 100ms = 3 ticks worth
        let ticks = acc.accumulate(100_000);
        assert_eq!(ticks, 3);
    }

    #[test]
    fn accumulate_sub_tick() {
        let mut acc = TickAccumulator::new();
        let ticks = acc.accumulate(10_000);
        assert_eq!(ticks, 0);
    }

    #[test]
    fn accumulate_carries_remainder() {
        let mut acc = TickAccumulator::new();
        acc.accumulate(20_000); // 0 ticks, 20000 remaining
        let ticks = acc.accumulate(20_000); // 40000 total -> 1 tick
        assert_eq!(ticks, 1);
    }

    #[test]
    fn interpolation_factor_range() {
        let mut acc = TickAccumulator::new();
        acc.accumulate(16_667); // half a tick
        let factor = acc.interpolation_factor();
        assert!(factor > 0.4 && factor < 0.6);
    }

    #[test]
    fn valid_transitions() {
        assert!(is_valid_transition(GameState::Loading, GameState::MainMenu));
        assert!(is_valid_transition(GameState::Playing, GameState::Paused));
        assert!(is_valid_transition(GameState::Playing, GameState::Terminal));
        assert!(is_valid_transition(GameState::Terminal, GameState::Playing));
        assert!(is_valid_transition(
            GameState::Playing,
            GameState::Intermission
        ));
        assert!(is_valid_transition(GameState::Playing, GameState::GameOver));
        assert!(is_valid_transition(
            GameState::GameOver,
            GameState::MainMenu
        ));
    }

    #[test]
    fn invalid_transitions() {
        assert!(!is_valid_transition(GameState::Playing, GameState::Loading));
        assert!(!is_valid_transition(
            GameState::MainMenu,
            GameState::Playing
        ));
        assert!(!is_valid_transition(
            GameState::Terminal,
            GameState::GameOver
        ));
    }
}
