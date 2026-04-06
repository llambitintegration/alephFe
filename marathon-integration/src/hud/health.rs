use super::{HudLayout, ShieldTier};

/// Health and shield bar rendering data.
pub struct HealthBar {
    /// Normalized health (0.0 to 1.0).
    pub health_fraction: f32,
    /// Normalized shield (0.0 to 1.0) within current tier.
    pub shield_fraction: f32,
    /// Shield color tier.
    pub shield_tier: ShieldTier,
    /// Screen-space rectangle for health bar [x, y, width, height].
    pub health_rect: [f32; 4],
    /// Screen-space rectangle for shield bar.
    pub shield_rect: [f32; 4],
}

impl HealthBar {
    /// Compute health bar rendering data from sim state and layout.
    pub fn compute(
        health: i16,
        max_health: i16,
        shield: i16,
        layout: &HudLayout,
    ) -> Self {
        let health_fraction = if max_health > 0 {
            (health as f32 / max_health as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let shield_tier = ShieldTier::from_shield_value(shield);
        let tier_max: f32 = 150.0;
        let shield_in_tier = match shield_tier {
            ShieldTier::None => 0.0,
            ShieldTier::Single => shield as f32,
            ShieldTier::Double => (shield - 150) as f32,
            ShieldTier::Triple => (shield - 300) as f32,
        };
        let shield_fraction = (shield_in_tier / tier_max).clamp(0.0, 1.0);

        // Position bars at bottom-left of screen, scaled by layout
        let bar_width = 200.0 * layout.scale;
        let bar_height = 16.0 * layout.scale;
        let x = 20.0 * layout.scale;
        let health_y = layout.screen_height as f32 - 50.0 * layout.scale;
        let shield_y = health_y - bar_height - 4.0 * layout.scale;

        Self {
            health_fraction,
            shield_fraction,
            shield_tier,
            health_rect: [x, health_y, bar_width, bar_height],
            shield_rect: [x, shield_y, bar_width, bar_height],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_health() {
        let layout = HudLayout::for_resolution(640, 480);
        let bar = HealthBar::compute(150, 150, 0, &layout);
        assert!((bar.health_fraction - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn zero_health() {
        let layout = HudLayout::for_resolution(640, 480);
        let bar = HealthBar::compute(0, 150, 0, &layout);
        assert!((bar.health_fraction - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn shield_tier_detection() {
        assert_eq!(ShieldTier::from_shield_value(0), ShieldTier::None);
        assert_eq!(ShieldTier::from_shield_value(100), ShieldTier::Single);
        assert_eq!(ShieldTier::from_shield_value(200), ShieldTier::Double);
        assert_eq!(ShieldTier::from_shield_value(400), ShieldTier::Triple);
    }

    #[test]
    fn high_res_scaling() {
        let layout = HudLayout::for_resolution(1920, 1080);
        let bar = HealthBar::compute(75, 150, 100, &layout);
        assert!((bar.health_fraction - 0.5).abs() < f32::EPSILON);
        // Bars should be wider at higher resolution
        assert!(bar.health_rect[2] > 200.0);
    }
}
