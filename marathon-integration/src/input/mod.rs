pub mod action_flags;
pub mod bindings;

mod context;

pub use action_flags::translate_gameplay_input;
pub use bindings::KeyBindings;
pub use context::InputContext;

use serde::{Deserialize, Serialize};

/// A raw input event normalized from winit.
#[derive(Debug, Clone)]
pub enum RawInput {
    KeyPress(KeyCode),
    KeyRelease(KeyCode),
    MouseDelta(f64, f64),
    MouseButtonPress(MouseButton),
    MouseButtonRelease(MouseButton),
    GamepadAxis(GamepadAxisType, f32),
    GamepadButtonPress(GamepadButtonType),
    GamepadButtonRelease(GamepadButtonType),
}

/// Keyboard key codes (subset covering Marathon-relevant keys).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyCode {
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9,
    Up, Down, Left, Right,
    Space, Enter, Escape, Tab,
    LShift, RShift, LCtrl, RCtrl, LAlt, RAlt,
    Backspace, Delete,
    PageUp, PageDown, Home, End,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    Backtick,
}

/// Mouse button identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Button4,
    Button5,
}

/// Gamepad axis types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GamepadAxisType {
    LeftStickX,
    LeftStickY,
    RightStickX,
    RightStickY,
    LeftTrigger,
    RightTrigger,
}

/// Gamepad button types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GamepadButtonType {
    South,
    East,
    North,
    West,
    LeftBumper,
    RightBumper,
    Select,
    Start,
    LeftStick,
    RightStick,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

/// Actions emitted during menu navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MenuAction {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
}

/// Actions emitted during terminal viewing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalAction {
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    Exit,
}

/// Per-frame input buffer collecting all raw events.
#[derive(Debug, Default)]
pub struct InputBuffer {
    pub events: Vec<RawInput>,
}

impl InputBuffer {
    pub fn clear(&mut self) {
        self.events.clear();
    }

    pub fn push(&mut self, event: RawInput) {
        self.events.push(event);
    }
}

/// Input configuration (sensitivity, dead zones).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    pub mouse_sensitivity: f64,
    pub gamepad_dead_zone: f32,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 1.0,
            gamepad_dead_zone: 0.15,
        }
    }
}
