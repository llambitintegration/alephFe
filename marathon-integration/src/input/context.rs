use crate::types::GameState;

/// Input context determines which key binding map is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputContext {
    Gameplay,
    Menu,
    Terminal,
}

impl InputContext {
    /// Derive the active input context from the current game state.
    pub fn from_game_state(state: GameState) -> Self {
        match state {
            GameState::Playing => InputContext::Gameplay,
            GameState::Terminal => InputContext::Terminal,
            GameState::MainMenu | GameState::Paused | GameState::GameOver => InputContext::Menu,
            // During loading/intermission, default to menu context for any UI
            GameState::Loading | GameState::Intermission => InputContext::Menu,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playing_maps_to_gameplay() {
        assert_eq!(
            InputContext::from_game_state(GameState::Playing),
            InputContext::Gameplay
        );
    }

    #[test]
    fn terminal_maps_to_terminal() {
        assert_eq!(
            InputContext::from_game_state(GameState::Terminal),
            InputContext::Terminal
        );
    }

    #[test]
    fn menu_states_map_to_menu() {
        for state in [GameState::MainMenu, GameState::Paused, GameState::GameOver] {
            assert_eq!(InputContext::from_game_state(state), InputContext::Menu);
        }
    }
}
