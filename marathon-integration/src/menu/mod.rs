use crate::types::Difficulty;

/// Menu screen types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuScreen {
    MainMenu,
    NewGame,
    LoadGame,
    Preferences,
    PauseMenu,
}

/// A menu item with a label and an associated action.
#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    pub action: MenuItemAction,
    pub enabled: bool,
}

/// What happens when a menu item is selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuItemAction {
    StartNewGame(Difficulty),
    LoadSaveSlot(usize),
    OpenScreen(MenuScreen),
    ResumeGame,
    SaveGame,
    QuitToMenu,
    QuitGame,
}

/// Menu navigation stack.
#[derive(Debug)]
pub struct MenuStack {
    screens: Vec<MenuScreenState>,
}

/// State of a single menu screen.
#[derive(Debug)]
pub struct MenuScreenState {
    pub screen: MenuScreen,
    pub items: Vec<MenuItem>,
    pub cursor: usize,
}

impl MenuStack {
    pub fn new() -> Self {
        Self {
            screens: vec![Self::main_menu_state()],
        }
    }

    fn main_menu_state() -> MenuScreenState {
        MenuScreenState {
            screen: MenuScreen::MainMenu,
            items: vec![
                MenuItem {
                    label: "New Game".into(),
                    action: MenuItemAction::OpenScreen(MenuScreen::NewGame),
                    enabled: true,
                },
                MenuItem {
                    label: "Load Game".into(),
                    action: MenuItemAction::OpenScreen(MenuScreen::LoadGame),
                    enabled: true,
                },
                MenuItem {
                    label: "Preferences".into(),
                    action: MenuItemAction::OpenScreen(MenuScreen::Preferences),
                    enabled: true,
                },
                MenuItem {
                    label: "Quit".into(),
                    action: MenuItemAction::QuitGame,
                    enabled: true,
                },
            ],
            cursor: 0,
        }
    }

    /// Push a new screen onto the stack.
    pub fn push(&mut self, state: MenuScreenState) {
        self.screens.push(state);
    }

    /// Pop the current screen. Returns false if at root.
    pub fn pop(&mut self) -> bool {
        if self.screens.len() > 1 {
            self.screens.pop();
            true
        } else {
            false
        }
    }

    /// Current active screen.
    pub fn current(&self) -> &MenuScreenState {
        self.screens.last().expect("menu stack should never be empty")
    }

    /// Mutable reference to the current screen.
    pub fn current_mut(&mut self) -> &mut MenuScreenState {
        self.screens
            .last_mut()
            .expect("menu stack should never be empty")
    }

    /// Move cursor up.
    pub fn cursor_up(&mut self) {
        let current = self.current_mut();
        if current.cursor > 0 {
            current.cursor -= 1;
        }
    }

    /// Move cursor down.
    pub fn cursor_down(&mut self) {
        let current = self.current_mut();
        if current.cursor + 1 < current.items.len() {
            current.cursor += 1;
        }
    }

    /// Select the current item. Returns the action if the item is enabled.
    pub fn select(&self) -> Option<MenuItemAction> {
        let current = self.current();
        let item = current.items.get(current.cursor)?;
        if item.enabled {
            Some(item.action.clone())
        } else {
            None
        }
    }

    /// Build the new-game difficulty selection screen.
    pub fn new_game_screen() -> MenuScreenState {
        MenuScreenState {
            screen: MenuScreen::NewGame,
            items: vec![
                MenuItem {
                    label: "Kindergarten".into(),
                    action: MenuItemAction::StartNewGame(Difficulty::Kindergarten),
                    enabled: true,
                },
                MenuItem {
                    label: "Easy Street".into(),
                    action: MenuItemAction::StartNewGame(Difficulty::EasyStreet),
                    enabled: true,
                },
                MenuItem {
                    label: "Normal".into(),
                    action: MenuItemAction::StartNewGame(Difficulty::Normal),
                    enabled: true,
                },
                MenuItem {
                    label: "Major Damage".into(),
                    action: MenuItemAction::StartNewGame(Difficulty::MajorDamage),
                    enabled: true,
                },
                MenuItem {
                    label: "Total Carnage".into(),
                    action: MenuItemAction::StartNewGame(Difficulty::TotalCarnage),
                    enabled: true,
                },
            ],
            cursor: 2, // Default to Normal
        }
    }

    /// Build the pause menu screen.
    pub fn pause_menu_screen() -> MenuScreenState {
        MenuScreenState {
            screen: MenuScreen::PauseMenu,
            items: vec![
                MenuItem {
                    label: "Resume".into(),
                    action: MenuItemAction::ResumeGame,
                    enabled: true,
                },
                MenuItem {
                    label: "Save Game".into(),
                    action: MenuItemAction::SaveGame,
                    enabled: true,
                },
                MenuItem {
                    label: "Preferences".into(),
                    action: MenuItemAction::OpenScreen(MenuScreen::Preferences),
                    enabled: true,
                },
                MenuItem {
                    label: "Quit to Menu".into(),
                    action: MenuItemAction::QuitToMenu,
                    enabled: true,
                },
            ],
            cursor: 0,
        }
    }
}

impl Default for MenuStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_menu_initial_state() {
        let stack = MenuStack::new();
        assert_eq!(stack.current().screen, MenuScreen::MainMenu);
        assert_eq!(stack.current().cursor, 0);
        assert_eq!(stack.current().items.len(), 4);
    }

    #[test]
    fn cursor_navigation() {
        let mut stack = MenuStack::new();
        stack.cursor_down();
        assert_eq!(stack.current().cursor, 1);

        stack.cursor_down();
        stack.cursor_down();
        assert_eq!(stack.current().cursor, 3);

        // Can't go past end
        stack.cursor_down();
        assert_eq!(stack.current().cursor, 3);

        stack.cursor_up();
        assert_eq!(stack.current().cursor, 2);
    }

    #[test]
    fn push_pop_navigation() {
        let mut stack = MenuStack::new();
        stack.push(MenuStack::new_game_screen());
        assert_eq!(stack.current().screen, MenuScreen::NewGame);

        stack.pop();
        assert_eq!(stack.current().screen, MenuScreen::MainMenu);

        // Can't pop past root
        assert!(!stack.pop());
        assert_eq!(stack.current().screen, MenuScreen::MainMenu);
    }

    #[test]
    fn select_returns_action() {
        let stack = MenuStack::new();
        let action = stack.select();
        assert_eq!(
            action,
            Some(MenuItemAction::OpenScreen(MenuScreen::NewGame))
        );
    }

    #[test]
    fn new_game_screen_defaults_to_normal() {
        let screen = MenuStack::new_game_screen();
        assert_eq!(screen.cursor, 2);
        assert_eq!(
            screen.items[2].action,
            MenuItemAction::StartNewGame(Difficulty::Normal)
        );
    }
}
