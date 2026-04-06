use crate::input::InputConfig;

use super::{MenuItem, MenuItemAction, MenuScreen, MenuScreenState};

/// User-facing preferences that can be changed in the settings screen.
#[derive(Debug, Clone)]
pub struct Preferences {
    /// Audio volume (0.0 to 1.0).
    pub master_volume: f32,
    /// Music volume (0.0 to 1.0).
    pub music_volume: f32,
    /// Sound effects volume (0.0 to 1.0).
    pub sfx_volume: f32,
    /// Display resolution width.
    pub resolution_width: u32,
    /// Display resolution height.
    pub resolution_height: u32,
    /// Fullscreen mode.
    pub fullscreen: bool,
    /// Input configuration (sensitivity, dead zones).
    pub input: InputConfig,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            music_volume: 0.8,
            sfx_volume: 1.0,
            resolution_width: 1280,
            resolution_height: 720,
            fullscreen: false,
            input: InputConfig::default(),
        }
    }
}

/// Which preferences sub-category is being edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreferencesCategory {
    Controls,
    Audio,
    Video,
}

/// A single preference setting that can be adjusted.
#[derive(Debug, Clone)]
pub enum PreferenceValue {
    /// A slider value (current, min, max, step).
    Slider { current: f32, min: f32, max: f32, step: f32 },
    /// A toggle (on/off).
    Toggle(bool),
    /// A choice among discrete options (current index, labels).
    Choice { current: usize, options: Vec<String> },
}

/// A preference item displayed in the preferences screen.
#[derive(Debug, Clone)]
pub struct PreferenceItem {
    pub label: String,
    pub category: PreferencesCategory,
    pub value: PreferenceValue,
}

/// Build the preferences screen with controls, audio, and video settings.
pub fn preferences_screen(prefs: &Preferences) -> MenuScreenState {
    let items = build_preference_menu_items(prefs);
    MenuScreenState {
        screen: MenuScreen::Preferences,
        items,
        cursor: 0,
    }
}

fn build_preference_menu_items(_prefs: &Preferences) -> Vec<MenuItem> {
    vec![
        MenuItem {
            label: "Controls".into(),
            action: MenuItemAction::OpenScreen(MenuScreen::Preferences),
            enabled: true,
        },
        MenuItem {
            label: "Audio".into(),
            action: MenuItemAction::OpenScreen(MenuScreen::Preferences),
            enabled: true,
        },
        MenuItem {
            label: "Video".into(),
            action: MenuItemAction::OpenScreen(MenuScreen::Preferences),
            enabled: true,
        },
    ]
}

/// Build the list of adjustable preference items for a given category.
pub fn preference_items(prefs: &Preferences, category: PreferencesCategory) -> Vec<PreferenceItem> {
    match category {
        PreferencesCategory::Controls => vec![
            PreferenceItem {
                label: "Mouse Sensitivity".into(),
                category,
                value: PreferenceValue::Slider {
                    current: prefs.input.mouse_sensitivity as f32,
                    min: 0.1,
                    max: 5.0,
                    step: 0.1,
                },
            },
            PreferenceItem {
                label: "Gamepad Dead Zone".into(),
                category,
                value: PreferenceValue::Slider {
                    current: prefs.input.gamepad_dead_zone,
                    min: 0.05,
                    max: 0.50,
                    step: 0.05,
                },
            },
        ],
        PreferencesCategory::Audio => vec![
            PreferenceItem {
                label: "Master Volume".into(),
                category,
                value: PreferenceValue::Slider {
                    current: prefs.master_volume,
                    min: 0.0,
                    max: 1.0,
                    step: 0.05,
                },
            },
            PreferenceItem {
                label: "Music Volume".into(),
                category,
                value: PreferenceValue::Slider {
                    current: prefs.music_volume,
                    min: 0.0,
                    max: 1.0,
                    step: 0.05,
                },
            },
            PreferenceItem {
                label: "SFX Volume".into(),
                category,
                value: PreferenceValue::Slider {
                    current: prefs.sfx_volume,
                    min: 0.0,
                    max: 1.0,
                    step: 0.05,
                },
            },
        ],
        PreferencesCategory::Video => {
            let resolution_options = vec![
                "640x480".into(),
                "800x600".into(),
                "1024x768".into(),
                "1280x720".into(),
                "1920x1080".into(),
                "2560x1440".into(),
            ];
            let current_res = format!("{}x{}", prefs.resolution_width, prefs.resolution_height);
            let current_idx = resolution_options
                .iter()
                .position(|r| *r == current_res)
                .unwrap_or(3);

            vec![
                PreferenceItem {
                    label: "Resolution".into(),
                    category,
                    value: PreferenceValue::Choice {
                        current: current_idx,
                        options: resolution_options,
                    },
                },
                PreferenceItem {
                    label: "Fullscreen".into(),
                    category,
                    value: PreferenceValue::Toggle(prefs.fullscreen),
                },
            ]
        }
    }
}

/// Apply a preference value change back to the Preferences struct.
pub fn apply_preference(prefs: &mut Preferences, label: &str, value: &PreferenceValue) {
    match (label, value) {
        ("Mouse Sensitivity", PreferenceValue::Slider { current, .. }) => {
            prefs.input.mouse_sensitivity = *current as f64;
        }
        ("Gamepad Dead Zone", PreferenceValue::Slider { current, .. }) => {
            prefs.input.gamepad_dead_zone = *current;
        }
        ("Master Volume", PreferenceValue::Slider { current, .. }) => {
            prefs.master_volume = *current;
        }
        ("Music Volume", PreferenceValue::Slider { current, .. }) => {
            prefs.music_volume = *current;
        }
        ("SFX Volume", PreferenceValue::Slider { current, .. }) => {
            prefs.sfx_volume = *current;
        }
        ("Fullscreen", PreferenceValue::Toggle(val)) => {
            prefs.fullscreen = *val;
        }
        ("Resolution", PreferenceValue::Choice { current, options }) => {
            if let Some(res_str) = options.get(*current) {
                if let Some((w, h)) = parse_resolution(res_str) {
                    prefs.resolution_width = w;
                    prefs.resolution_height = h;
                }
            }
        }
        _ => {}
    }
}

fn parse_resolution(s: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() == 2 {
        let w = parts[0].parse().ok()?;
        let h = parts[1].parse().ok()?;
        Some((w, h))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_preferences() {
        let prefs = Preferences::default();
        assert!((prefs.master_volume - 1.0).abs() < f32::EPSILON);
        assert_eq!(prefs.resolution_width, 1280);
        assert!(!prefs.fullscreen);
    }

    #[test]
    fn controls_preference_items() {
        let prefs = Preferences::default();
        let items = preference_items(&prefs, PreferencesCategory::Controls);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].label, "Mouse Sensitivity");
        assert_eq!(items[1].label, "Gamepad Dead Zone");
    }

    #[test]
    fn audio_preference_items() {
        let prefs = Preferences::default();
        let items = preference_items(&prefs, PreferencesCategory::Audio);
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn video_preference_items() {
        let prefs = Preferences::default();
        let items = preference_items(&prefs, PreferencesCategory::Video);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].label, "Resolution");
        assert_eq!(items[1].label, "Fullscreen");
    }

    #[test]
    fn apply_mouse_sensitivity() {
        let mut prefs = Preferences::default();
        apply_preference(
            &mut prefs,
            "Mouse Sensitivity",
            &PreferenceValue::Slider { current: 2.5, min: 0.1, max: 5.0, step: 0.1 },
        );
        assert!((prefs.input.mouse_sensitivity - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn apply_resolution() {
        let mut prefs = Preferences::default();
        apply_preference(
            &mut prefs,
            "Resolution",
            &PreferenceValue::Choice {
                current: 4,
                options: vec![
                    "640x480".into(), "800x600".into(), "1024x768".into(),
                    "1280x720".into(), "1920x1080".into(), "2560x1440".into(),
                ],
            },
        );
        assert_eq!(prefs.resolution_width, 1920);
        assert_eq!(prefs.resolution_height, 1080);
    }

    #[test]
    fn apply_fullscreen_toggle() {
        let mut prefs = Preferences::default();
        assert!(!prefs.fullscreen);
        apply_preference(&mut prefs, "Fullscreen", &PreferenceValue::Toggle(true));
        assert!(prefs.fullscreen);
    }

    #[test]
    fn parse_resolution_valid() {
        assert_eq!(parse_resolution("1920x1080"), Some((1920, 1080)));
    }

    #[test]
    fn parse_resolution_invalid() {
        assert_eq!(parse_resolution("invalid"), None);
        assert_eq!(parse_resolution("x"), None);
    }
}
