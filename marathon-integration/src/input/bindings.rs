use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{InputContext, KeyCode, MenuAction, MouseButton, TerminalAction};
use crate::types::ActionFlags;

/// A physical input that can be bound to an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PhysicalInput {
    Key(KeyCode),
    Mouse(MouseButton),
}

/// A gameplay action that maps to one or more ActionFlags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameplayAction {
    MoveForward,
    MoveBackward,
    StrafeLeft,
    StrafeRight,
    TurnLeft,
    TurnRight,
    LookUp,
    LookDown,
    FirePrimary,
    FireSecondary,
    Action,
    CycleWeaponForward,
    CycleWeaponBackward,
    ToggleMap,
    Microphone,
}

impl GameplayAction {
    /// Convert to the corresponding ActionFlags bit.
    pub fn to_flag(self) -> ActionFlags {
        match self {
            Self::MoveForward => ActionFlags::MOVE_FORWARD,
            Self::MoveBackward => ActionFlags::MOVE_BACKWARD,
            Self::StrafeLeft => ActionFlags::STRAFE_LEFT,
            Self::StrafeRight => ActionFlags::STRAFE_RIGHT,
            Self::TurnLeft => ActionFlags::TURN_LEFT,
            Self::TurnRight => ActionFlags::TURN_RIGHT,
            Self::LookUp => ActionFlags::LOOK_UP,
            Self::LookDown => ActionFlags::LOOK_DOWN,
            Self::FirePrimary => ActionFlags::FIRE_PRIMARY,
            Self::FireSecondary => ActionFlags::FIRE_SECONDARY,
            Self::Action => ActionFlags::ACTION,
            Self::CycleWeaponForward => ActionFlags::CYCLE_WEAPON_FWD,
            Self::CycleWeaponBackward => ActionFlags::CYCLE_WEAPON_BACK,
            Self::ToggleMap => ActionFlags::TOGGLE_MAP,
            Self::Microphone => ActionFlags::MICROPHONE,
        }
    }
}

/// User-configurable key bindings for all input contexts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindings {
    pub gameplay: HashMap<PhysicalInput, GameplayAction>,
    pub menu: HashMap<PhysicalInput, MenuAction>,
    pub terminal: HashMap<PhysicalInput, TerminalAction>,
}

impl Default for KeyBindings {
    /// Marathon-standard default bindings.
    fn default() -> Self {
        let mut gameplay = HashMap::new();
        gameplay.insert(PhysicalInput::Key(KeyCode::W), GameplayAction::MoveForward);
        gameplay.insert(PhysicalInput::Key(KeyCode::Up), GameplayAction::MoveForward);
        gameplay.insert(
            PhysicalInput::Key(KeyCode::S),
            GameplayAction::MoveBackward,
        );
        gameplay.insert(
            PhysicalInput::Key(KeyCode::Down),
            GameplayAction::MoveBackward,
        );
        gameplay.insert(PhysicalInput::Key(KeyCode::A), GameplayAction::StrafeLeft);
        gameplay.insert(PhysicalInput::Key(KeyCode::D), GameplayAction::StrafeRight);
        gameplay.insert(PhysicalInput::Key(KeyCode::Left), GameplayAction::TurnLeft);
        gameplay.insert(
            PhysicalInput::Key(KeyCode::Right),
            GameplayAction::TurnRight,
        );
        gameplay.insert(
            PhysicalInput::Mouse(MouseButton::Left),
            GameplayAction::FirePrimary,
        );
        gameplay.insert(
            PhysicalInput::Mouse(MouseButton::Right),
            GameplayAction::FireSecondary,
        );
        gameplay.insert(PhysicalInput::Key(KeyCode::Space), GameplayAction::Action);
        gameplay.insert(
            PhysicalInput::Key(KeyCode::Tab),
            GameplayAction::CycleWeaponForward,
        );
        gameplay.insert(
            PhysicalInput::Key(KeyCode::M),
            GameplayAction::ToggleMap,
        );
        gameplay.insert(
            PhysicalInput::Key(KeyCode::Backtick),
            GameplayAction::Microphone,
        );

        let mut menu = HashMap::new();
        menu.insert(PhysicalInput::Key(KeyCode::Up), MenuAction::Up);
        menu.insert(PhysicalInput::Key(KeyCode::Down), MenuAction::Down);
        menu.insert(PhysicalInput::Key(KeyCode::Left), MenuAction::Left);
        menu.insert(PhysicalInput::Key(KeyCode::Right), MenuAction::Right);
        menu.insert(PhysicalInput::Key(KeyCode::Enter), MenuAction::Select);
        menu.insert(PhysicalInput::Key(KeyCode::Space), MenuAction::Select);
        menu.insert(PhysicalInput::Key(KeyCode::Escape), MenuAction::Back);

        let mut terminal = HashMap::new();
        terminal.insert(PhysicalInput::Key(KeyCode::Up), TerminalAction::ScrollUp);
        terminal.insert(
            PhysicalInput::Key(KeyCode::Down),
            TerminalAction::ScrollDown,
        );
        terminal.insert(
            PhysicalInput::Key(KeyCode::PageUp),
            TerminalAction::PageUp,
        );
        terminal.insert(
            PhysicalInput::Key(KeyCode::PageDown),
            TerminalAction::PageDown,
        );
        terminal.insert(PhysicalInput::Key(KeyCode::Escape), TerminalAction::Exit);

        Self {
            gameplay,
            menu,
            terminal,
        }
    }
}

impl KeyBindings {
    /// Look up which gameplay action a physical input maps to, if any.
    pub fn resolve_gameplay(&self, input: &PhysicalInput) -> Option<GameplayAction> {
        self.gameplay.get(input).copied()
    }

    /// Look up which menu action a physical input maps to, if any.
    pub fn resolve_menu(&self, input: &PhysicalInput) -> Option<MenuAction> {
        self.menu.get(input).copied()
    }

    /// Look up which terminal action a physical input maps to, if any.
    pub fn resolve_terminal(&self, input: &PhysicalInput) -> Option<TerminalAction> {
        self.terminal.get(input).copied()
    }

    /// Get the active binding map for a given context (returns a list of
    /// bound physical inputs for introspection / rebinding UI).
    pub fn bound_inputs_for_context(&self, context: InputContext) -> Vec<PhysicalInput> {
        match context {
            InputContext::Gameplay => self.gameplay.keys().copied().collect(),
            InputContext::Menu => self.menu.keys().copied().collect(),
            InputContext::Terminal => self.terminal.keys().copied().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bindings_have_movement() {
        let bindings = KeyBindings::default();
        assert_eq!(
            bindings.resolve_gameplay(&PhysicalInput::Key(KeyCode::W)),
            Some(GameplayAction::MoveForward)
        );
        assert_eq!(
            bindings.resolve_gameplay(&PhysicalInput::Key(KeyCode::S)),
            Some(GameplayAction::MoveBackward)
        );
    }

    #[test]
    fn default_bindings_have_menu_nav() {
        let bindings = KeyBindings::default();
        assert_eq!(
            bindings.resolve_menu(&PhysicalInput::Key(KeyCode::Enter)),
            Some(MenuAction::Select)
        );
        assert_eq!(
            bindings.resolve_menu(&PhysicalInput::Key(KeyCode::Escape)),
            Some(MenuAction::Back)
        );
    }

    #[test]
    fn default_bindings_have_terminal_nav() {
        let bindings = KeyBindings::default();
        assert_eq!(
            bindings.resolve_terminal(&PhysicalInput::Key(KeyCode::Escape)),
            Some(TerminalAction::Exit)
        );
        assert_eq!(
            bindings.resolve_terminal(&PhysicalInput::Key(KeyCode::PageDown)),
            Some(TerminalAction::PageDown)
        );
    }

    #[test]
    fn unbound_key_returns_none() {
        let bindings = KeyBindings::default();
        assert_eq!(
            bindings.resolve_gameplay(&PhysicalInput::Key(KeyCode::F12)),
            None
        );
    }

    #[test]
    fn action_to_flag_mapping() {
        assert_eq!(GameplayAction::MoveForward.to_flag(), ActionFlags::MOVE_FORWARD);
        assert_eq!(GameplayAction::FirePrimary.to_flag(), ActionFlags::FIRE_PRIMARY);
        assert_eq!(GameplayAction::ToggleMap.to_flag(), ActionFlags::TOGGLE_MAP);
    }
}
