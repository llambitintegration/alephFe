use crate::components::{Light, LightFunction};
use rand::Rng;

/// Compute a light's intensity for the current tick.
pub fn compute_light_intensity(light: &Light, tick: u64, rng: &mut impl Rng) -> f32 {
    let range = light.intensity_max - light.intensity_min;

    match light.function {
        LightFunction::Constant => light.intensity_max,
        LightFunction::Linear => {
            let t = ((tick + light.phase as u64) % light.period as u64) as f32
                / light.period as f32;
            // Triangle wave: 0->1->0 over one period
            let wave = if t < 0.5 { t * 2.0 } else { 2.0 - t * 2.0 };
            light.intensity_min + range * wave
        }
        LightFunction::Smooth => {
            let t = ((tick + light.phase as u64) % light.period as u64) as f32
                / light.period as f32;
            // Cosine wave: smooth oscillation
            let wave = (1.0 - (t * std::f32::consts::TAU).cos()) * 0.5;
            light.intensity_min + range * wave
        }
        LightFunction::Flicker => {
            // Random value each tick
            light.intensity_min + range * rng.gen::<f32>()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn make_light(function: LightFunction, period: u32) -> Light {
        Light {
            light_index: 0,
            function,
            period,
            phase: 0,
            intensity_min: 0.0,
            intensity_max: 1.0,
            current_intensity: 1.0,
        }
    }

    #[test]
    fn constant_light() {
        let light = make_light(LightFunction::Constant, 60);
        let mut rng = StdRng::seed_from_u64(42);
        assert_eq!(compute_light_intensity(&light, 0, &mut rng), 1.0);
        assert_eq!(compute_light_intensity(&light, 100, &mut rng), 1.0);
    }

    #[test]
    fn linear_light_oscillates() {
        let light = make_light(LightFunction::Linear, 100);
        let mut rng = StdRng::seed_from_u64(42);

        let at_0 = compute_light_intensity(&light, 0, &mut rng);
        let at_25 = compute_light_intensity(&light, 25, &mut rng);
        let at_50 = compute_light_intensity(&light, 50, &mut rng);

        assert!((at_0 - 0.0).abs() < 0.01); // start of period
        assert!((at_25 - 0.5).abs() < 0.01); // quarter period
        assert!((at_50 - 1.0).abs() < 0.01); // half period = peak
    }

    #[test]
    fn smooth_light_oscillates() {
        let light = make_light(LightFunction::Smooth, 60);
        let mut rng = StdRng::seed_from_u64(42);

        let at_0 = compute_light_intensity(&light, 0, &mut rng);
        let at_30 = compute_light_intensity(&light, 30, &mut rng);

        // Cosine: at t=0 wave=0 (min), at t=0.5 wave=1 (max)
        assert!((at_0 - 0.0).abs() < 0.01);
        assert!((at_30 - 1.0).abs() < 0.01);
    }

    #[test]
    fn flicker_in_range() {
        let light = make_light(LightFunction::Flicker, 60);
        let mut rng = StdRng::seed_from_u64(42);

        for tick in 0..100 {
            let intensity = compute_light_intensity(&light, tick, &mut rng);
            assert!(intensity >= 0.0 && intensity <= 1.0);
        }
    }

    #[test]
    fn phase_offset() {
        let light_a = make_light(LightFunction::Linear, 100);
        let mut light_b = make_light(LightFunction::Linear, 100);
        light_b.phase = 25;
        let mut rng = StdRng::seed_from_u64(42);

        let _a_at_0 = compute_light_intensity(&light_a, 0, &mut rng);
        let b_at_0 = compute_light_intensity(&light_b, 0, &mut rng);

        // light_b at tick 0 should equal light_a at tick 25
        let a_at_25 = compute_light_intensity(&light_a, 25, &mut rng);
        assert!((b_at_0 - a_at_25).abs() < 0.01);
    }
}
