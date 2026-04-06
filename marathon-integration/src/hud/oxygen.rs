use super::HudLayout;

/// Oxygen meter rendering data.
pub struct OxygenMeter {
    /// Whether the oxygen meter should be visible.
    pub visible: bool,
    /// Normalized oxygen level (0.0 to 1.0).
    pub oxygen_fraction: f32,
    /// Whether oxygen is critically low (below 25%).
    pub critical: bool,
    /// Screen-space rectangle [x, y, width, height].
    pub rect: [f32; 4],
}

impl OxygenMeter {
    /// Compute oxygen meter rendering data.
    pub fn compute(
        oxygen: i16,
        max_oxygen: i16,
        in_vacuum: bool,
        layout: &HudLayout,
    ) -> Self {
        let oxygen_fraction = if max_oxygen > 0 {
            (oxygen as f32 / max_oxygen as f32).clamp(0.0, 1.0)
        } else {
            1.0
        };

        let critical = oxygen_fraction < 0.25;

        let bar_width = 120.0 * layout.scale;
        let bar_height = 12.0 * layout.scale;
        let x = 240.0 * layout.scale;
        let y = layout.screen_height as f32 - 50.0 * layout.scale;

        Self {
            visible: in_vacuum,
            oxygen_fraction,
            critical,
            rect: [x, y, bar_width, bar_height],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_in_normal_atmosphere() {
        let layout = HudLayout::for_resolution(640, 480);
        let meter = OxygenMeter::compute(100, 100, false, &layout);
        assert!(!meter.visible);
    }

    #[test]
    fn visible_in_vacuum() {
        let layout = HudLayout::for_resolution(640, 480);
        let meter = OxygenMeter::compute(50, 100, true, &layout);
        assert!(meter.visible);
        assert!((meter.oxygen_fraction - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn critical_below_25_percent() {
        let layout = HudLayout::for_resolution(640, 480);
        let meter = OxygenMeter::compute(20, 100, true, &layout);
        assert!(meter.critical);
    }

    #[test]
    fn not_critical_above_25_percent() {
        let layout = HudLayout::for_resolution(640, 480);
        let meter = OxygenMeter::compute(30, 100, true, &layout);
        assert!(!meter.critical);
    }
}
