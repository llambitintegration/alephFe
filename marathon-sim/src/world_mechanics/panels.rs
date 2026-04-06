use glam::Vec2;

/// Types of control panel actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanelAction {
    ActivatePlatform { platform_index: usize },
    ToggleLight { light_index: usize },
    ActivateTerminal { terminal_index: usize },
}

/// A control panel on a wall side.
#[derive(Debug, Clone)]
pub struct ControlPanel {
    /// Line index of the side with the panel.
    pub line_index: usize,
    /// Which side of the line (0 = clockwise, 1 = counterclockwise).
    pub side: u8,
    /// Action triggered on activation.
    pub action: PanelAction,
    /// Maximum activation distance.
    pub max_distance: f32,
}

/// Check if a player can activate a control panel.
///
/// The player must be facing the panel's line, within range, and pressing action.
pub fn can_activate_panel(
    player_pos: Vec2,
    player_facing: f32,
    panel: &ControlPanel,
    line_endpoints: &[(Vec2, Vec2)],
) -> bool {
    let (la, lb) = line_endpoints[panel.line_index];
    let line_center = (la + lb) * 0.5;
    let to_panel = line_center - player_pos;
    let distance = to_panel.length();

    if distance > panel.max_distance || distance < 1e-6 {
        return false;
    }

    // Check if player is roughly facing the panel
    let angle_to_panel = to_panel.y.atan2(to_panel.x);
    let angle_diff = normalize_angle(angle_to_panel - player_facing);

    // Must be within ~60 degrees of facing
    angle_diff.abs() <= std::f32::consts::FRAC_PI_3
}

fn normalize_angle(angle: f32) -> f32 {
    let mut a = angle % std::f32::consts::TAU;
    if a > std::f32::consts::PI {
        a -= std::f32::consts::TAU;
    } else if a < -std::f32::consts::PI {
        a += std::f32::consts::TAU;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activate_panel_facing_it() {
        let panel = ControlPanel {
            line_index: 0,
            side: 0,
            action: PanelAction::ActivatePlatform { platform_index: 0 },
            max_distance: 2.0,
        };
        let endpoints = vec![(Vec2::new(1.0, -0.5), Vec2::new(1.0, 0.5))];

        // Player at origin, facing east (toward the panel)
        assert!(can_activate_panel(Vec2::ZERO, 0.0, &panel, &endpoints));
    }

    #[test]
    fn cant_activate_panel_facing_away() {
        let panel = ControlPanel {
            line_index: 0,
            side: 0,
            action: PanelAction::ActivatePlatform { platform_index: 0 },
            max_distance: 2.0,
        };
        let endpoints = vec![(Vec2::new(1.0, -0.5), Vec2::new(1.0, 0.5))];

        // Player facing west (away from panel)
        assert!(!can_activate_panel(
            Vec2::ZERO,
            std::f32::consts::PI,
            &panel,
            &endpoints,
        ));
    }

    #[test]
    fn cant_activate_panel_too_far() {
        let panel = ControlPanel {
            line_index: 0,
            side: 0,
            action: PanelAction::ActivateTerminal { terminal_index: 5 },
            max_distance: 1.0,
        };
        let endpoints = vec![(Vec2::new(5.0, -0.5), Vec2::new(5.0, 0.5))];

        assert!(!can_activate_panel(Vec2::ZERO, 0.0, &panel, &endpoints));
    }
}
