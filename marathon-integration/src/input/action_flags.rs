use super::bindings::PhysicalInput;
use super::{InputBuffer, InputConfig, KeyBindings, MenuAction, RawInput, TerminalAction};
use crate::types::ActionFlags;

/// Translate the current frame's input buffer into ActionFlags for the simulation.
///
/// Processes all raw input events through the gameplay binding map, applying
/// mouse sensitivity scaling for turn/look actions and gamepad dead zone filtering.
pub fn translate_gameplay_input(
    buffer: &InputBuffer,
    bindings: &KeyBindings,
    config: &InputConfig,
) -> ActionFlags {
    let mut flags = ActionFlags::empty();

    for event in &buffer.events {
        match event {
            RawInput::KeyPress(key) => {
                let physical = PhysicalInput::Key(*key);
                if let Some(action) = bindings.resolve_gameplay(&physical) {
                    flags |= action.to_flag();
                }
            }
            RawInput::MouseButtonPress(button) => {
                let physical = PhysicalInput::Mouse(*button);
                if let Some(action) = bindings.resolve_gameplay(&physical) {
                    flags |= action.to_flag();
                }
            }
            RawInput::MouseDelta(dx, _dy) => {
                let scaled_dx = *dx * config.mouse_sensitivity;
                if scaled_dx > 0.0 {
                    flags |= ActionFlags::TURN_RIGHT;
                } else if scaled_dx < 0.0 {
                    flags |= ActionFlags::TURN_LEFT;
                }
            }
            RawInput::GamepadAxis(axis_type, value) => {
                let effective = apply_dead_zone(*value, config.gamepad_dead_zone);
                if effective.abs() > 0.0 {
                    use super::GamepadAxisType;
                    match axis_type {
                        GamepadAxisType::LeftStickX => {
                            if effective > 0.0 {
                                flags |= ActionFlags::STRAFE_RIGHT;
                            } else {
                                flags |= ActionFlags::STRAFE_LEFT;
                            }
                        }
                        GamepadAxisType::LeftStickY => {
                            if effective > 0.0 {
                                flags |= ActionFlags::MOVE_FORWARD;
                            } else {
                                flags |= ActionFlags::MOVE_BACKWARD;
                            }
                        }
                        GamepadAxisType::RightStickX => {
                            if effective > 0.0 {
                                flags |= ActionFlags::TURN_RIGHT;
                            } else {
                                flags |= ActionFlags::TURN_LEFT;
                            }
                        }
                        GamepadAxisType::RightStickY => {
                            if effective > 0.0 {
                                flags |= ActionFlags::LOOK_UP;
                            } else {
                                flags |= ActionFlags::LOOK_DOWN;
                            }
                        }
                        _ => {}
                    }
                }
            }
            // Key releases and gamepad button events don't set flags
            // (flags are per-tick, not toggle-based)
            _ => {}
        }
    }

    flags
}

/// Translate the current frame's input buffer into menu actions.
pub fn translate_menu_input(
    buffer: &InputBuffer,
    bindings: &KeyBindings,
) -> Vec<MenuAction> {
    let mut actions = Vec::new();

    for event in &buffer.events {
        if let RawInput::KeyPress(key) = event {
            let physical = PhysicalInput::Key(*key);
            if let Some(action) = bindings.resolve_menu(&physical) {
                actions.push(action);
            }
        }
    }

    actions
}

/// Translate the current frame's input buffer into terminal actions.
pub fn translate_terminal_input(
    buffer: &InputBuffer,
    bindings: &KeyBindings,
) -> Vec<TerminalAction> {
    let mut actions = Vec::new();

    for event in &buffer.events {
        if let RawInput::KeyPress(key) = event {
            let physical = PhysicalInput::Key(*key);
            if let Some(action) = bindings.resolve_terminal(&physical) {
                actions.push(action);
            }
        }
    }

    actions
}

/// Apply dead zone filtering to a gamepad axis value.
///
/// Values below the dead zone threshold are zeroed. Values above are
/// remapped from [dead_zone, 1.0] to [0.0, 1.0], preserving sign.
pub fn apply_dead_zone(value: f32, dead_zone: f32) -> f32 {
    let abs_val = value.abs();
    if abs_val < dead_zone {
        0.0
    } else {
        let remapped = (abs_val - dead_zone) / (1.0 - dead_zone);
        remapped.copysign(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{GamepadAxisType, KeyCode, MouseButton};

    fn make_buffer(events: Vec<RawInput>) -> InputBuffer {
        InputBuffer { events }
    }

    #[test]
    fn forward_key_sets_flag() {
        let buffer = make_buffer(vec![RawInput::KeyPress(KeyCode::W)]);
        let bindings = KeyBindings::default();
        let config = InputConfig::default();

        let flags = translate_gameplay_input(&buffer, &bindings, &config);
        assert!(flags.contains(ActionFlags::MOVE_FORWARD));
    }

    #[test]
    fn mouse_left_sets_fire_primary() {
        let buffer = make_buffer(vec![RawInput::MouseButtonPress(MouseButton::Left)]);
        let bindings = KeyBindings::default();
        let config = InputConfig::default();

        let flags = translate_gameplay_input(&buffer, &bindings, &config);
        assert!(flags.contains(ActionFlags::FIRE_PRIMARY));
    }

    #[test]
    fn mouse_delta_positive_turns_right() {
        let buffer = make_buffer(vec![RawInput::MouseDelta(10.0, 0.0)]);
        let bindings = KeyBindings::default();
        let config = InputConfig::default();

        let flags = translate_gameplay_input(&buffer, &bindings, &config);
        assert!(flags.contains(ActionFlags::TURN_RIGHT));
        assert!(!flags.contains(ActionFlags::TURN_LEFT));
    }

    #[test]
    fn mouse_sensitivity_scaling() {
        // With default sensitivity (1.0), a positive delta should turn right
        let buffer = make_buffer(vec![RawInput::MouseDelta(5.0, 0.0)]);
        let bindings = KeyBindings::default();
        let config = InputConfig {
            mouse_sensitivity: 2.0,
            gamepad_dead_zone: 0.15,
        };

        let flags = translate_gameplay_input(&buffer, &bindings, &config);
        assert!(flags.contains(ActionFlags::TURN_RIGHT));
    }

    #[test]
    fn no_input_gives_empty_flags() {
        let buffer = make_buffer(vec![]);
        let bindings = KeyBindings::default();
        let config = InputConfig::default();

        let flags = translate_gameplay_input(&buffer, &bindings, &config);
        assert!(flags.is_empty());
    }

    #[test]
    fn simultaneous_opposing_inputs() {
        let buffer = make_buffer(vec![
            RawInput::KeyPress(KeyCode::W),
            RawInput::KeyPress(KeyCode::S),
        ]);
        let bindings = KeyBindings::default();
        let config = InputConfig::default();

        let flags = translate_gameplay_input(&buffer, &bindings, &config);
        assert!(flags.contains(ActionFlags::MOVE_FORWARD));
        assert!(flags.contains(ActionFlags::MOVE_BACKWARD));
    }

    #[test]
    fn dead_zone_filters_small_values() {
        assert_eq!(apply_dead_zone(0.10, 0.15), 0.0);
        assert_eq!(apply_dead_zone(-0.05, 0.15), 0.0);
    }

    #[test]
    fn dead_zone_remaps_above_threshold() {
        let result = apply_dead_zone(0.50, 0.15);
        // (0.50 - 0.15) / (1.0 - 0.15) = 0.35 / 0.85 ≈ 0.4118
        assert!((result - 0.4118).abs() < 0.001);
    }

    #[test]
    fn dead_zone_preserves_sign() {
        let result = apply_dead_zone(-0.80, 0.15);
        assert!(result < 0.0);
    }

    #[test]
    fn gamepad_axis_below_dead_zone_no_flags() {
        let buffer = make_buffer(vec![RawInput::GamepadAxis(GamepadAxisType::LeftStickX, 0.10)]);
        let bindings = KeyBindings::default();
        let config = InputConfig {
            mouse_sensitivity: 1.0,
            gamepad_dead_zone: 0.15,
        };

        let flags = translate_gameplay_input(&buffer, &bindings, &config);
        assert!(flags.is_empty());
    }

    #[test]
    fn gamepad_left_stick_x_positive_strafes_right() {
        let buffer = make_buffer(vec![RawInput::GamepadAxis(GamepadAxisType::LeftStickX, 0.75)]);
        let bindings = KeyBindings::default();
        let config = InputConfig::default();

        let flags = translate_gameplay_input(&buffer, &bindings, &config);
        assert!(flags.contains(ActionFlags::STRAFE_RIGHT));
    }

    #[test]
    fn menu_input_translation() {
        let buffer = make_buffer(vec![
            RawInput::KeyPress(KeyCode::Down),
            RawInput::KeyPress(KeyCode::Enter),
        ]);
        let bindings = KeyBindings::default();

        let actions = translate_menu_input(&buffer, &bindings);
        assert_eq!(actions, vec![MenuAction::Down, MenuAction::Select]);
    }

    #[test]
    fn terminal_input_translation() {
        let buffer = make_buffer(vec![
            RawInput::KeyPress(KeyCode::PageDown),
            RawInput::KeyPress(KeyCode::Escape),
        ]);
        let bindings = KeyBindings::default();

        let actions = translate_terminal_input(&buffer, &bindings);
        assert_eq!(actions, vec![TerminalAction::PageDown, TerminalAction::Exit]);
    }
}
