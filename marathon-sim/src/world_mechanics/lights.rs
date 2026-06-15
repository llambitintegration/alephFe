use crate::components::LightFunction;
use rand::Rng;

/// Smooth (cosine-eased) interpolation from `initial` to `final` across the
/// `[0, period]` phase window: `initial + (final - initial) * (cos(phase * PI /
/// period + PI) + 1) / 2`. At `phase == 0` it returns `initial`; at
/// `phase == period` it returns `final`. Used directly by `Smooth` and as the
/// deterministic base of `Flicker`.
fn smooth_intensity(initial: f32, final_intensity: f32, phase: u32, period: u32) -> f32 {
    let period = period.max(1) as f32;
    let eased = ((phase as f32 * std::f32::consts::PI / period + std::f32::consts::PI).cos() + 1.0)
        / 2.0;
    initial + (final_intensity - initial) * eased
}

/// Compute a light's intensity for a given phase within its current state,
/// using Alephone's `map.h` / `lightsource.cpp` lighting-function semantics.
///
/// * `initial_intensity` / `final_intensity` — the endpoints of the current
///   state's intensity ramp (0.0..=1.0).
/// * `phase` — ticks elapsed within the current period (0..=`period`).
/// * `period` — duration of the current state in ticks.
/// * `function` — the animation function to evaluate.
/// * `rng` — randomness source for the stochastic functions.
pub fn compute_light_intensity(
    initial_intensity: f32,
    final_intensity: f32,
    phase: u32,
    period: u32,
    function: LightFunction,
    rng: &mut impl Rng,
) -> f32 {
    match function {
        // Holds at the final intensity regardless of phase.
        LightFunction::Constant => final_intensity,
        // Straight ramp from initial to final across the period.
        LightFunction::Linear => {
            let period = period.max(1) as f32;
            initial_intensity + (final_intensity - initial_intensity) * phase as f32 / period
        }
        // Cosine-eased ramp from initial to final across the period.
        LightFunction::Smooth => {
            smooth_intensity(initial_intensity, final_intensity, phase, period)
        }
        // Smooth base perturbed toward `final` by a random fraction.
        LightFunction::Flicker => {
            let base = smooth_intensity(initial_intensity, final_intensity, phase, period);
            base + rng.gen::<f32>() * (final_intensity - base)
        }
        // Uniform random value across the [initial, final] range.
        LightFunction::Random => {
            initial_intensity + rng.gen::<f32>() * (final_intensity - initial_intensity)
        }
        // Snaps to one of the two endpoints each evaluation.
        LightFunction::Fluorescent => {
            if rng.gen::<f32>() > 0.5 {
                final_intensity
            } else {
                initial_intensity
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    // ── box 2.8: each of the 6 functions with known initial/final values ──

    #[test]
    fn constant_returns_final() {
        let mut r = rng();
        let v = compute_light_intensity(0.2, 0.8, 30, 60, LightFunction::Constant, &mut r);
        assert!((v - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn linear_midpoint() {
        let mut r = rng();
        // initial + (final-initial)*phase/period = 0.0 + 1.0*50/100 = 0.5
        let v = compute_light_intensity(0.0, 1.0, 50, 100, LightFunction::Linear, &mut r);
        assert!((v - 0.5).abs() < 0.001);
    }

    #[test]
    fn smooth_midpoint() {
        let mut r = rng();
        // cos(50*PI/100 + PI) = cos(1.5*PI) = 0 -> (0+1)/2 = 0.5
        let v = compute_light_intensity(0.0, 1.0, 50, 100, LightFunction::Smooth, &mut r);
        assert!((v - 0.5).abs() < 0.001);
    }

    #[test]
    fn flicker_bounded_by_smooth_and_final() {
        // Flicker = smooth_base + rng*(final - smooth_base); for final>=smooth
        // base it stays within [smooth_base, final].
        let mut r = rng();
        let base = smooth_intensity(0.2, 0.9, 25, 100);
        for _ in 0..200 {
            let v = compute_light_intensity(0.2, 0.9, 25, 100, LightFunction::Flicker, &mut r);
            assert!(v >= base - 1e-4 && v <= 0.9 + 1e-4, "flicker {v} out of [{base},0.9]");
        }
    }

    #[test]
    fn random_within_range() {
        let mut r = rng();
        for _ in 0..200 {
            let v = compute_light_intensity(0.3, 0.7, 0, 60, LightFunction::Random, &mut r);
            assert!((0.3..=0.7).contains(&v), "random {v} out of [0.3,0.7]");
        }
    }

    #[test]
    fn fluorescent_snaps_to_endpoints() {
        let mut r = rng();
        for _ in 0..200 {
            let v = compute_light_intensity(0.1, 0.95, 13, 60, LightFunction::Fluorescent, &mut r);
            assert!(
                (v - 0.1).abs() < f32::EPSILON || (v - 0.95).abs() < f32::EPSILON,
                "fluorescent {v} not an endpoint"
            );
        }
    }

    // ── box 2.9: constant returns final regardless of phase ──

    #[test]
    fn constant_independent_of_phase() {
        let mut r = rng();
        for phase in [0, 1, 30, 59, 60, 1000] {
            let v = compute_light_intensity(0.2, 0.8, phase, 60, LightFunction::Constant, &mut r);
            assert!((v - 0.8).abs() < f32::EPSILON);
        }
    }

    // ── box 2.10: linear at phase=0 -> initial, at phase=period -> final ──

    #[test]
    fn linear_endpoints() {
        let mut r = rng();
        let at_0 = compute_light_intensity(0.25, 0.75, 0, 100, LightFunction::Linear, &mut r);
        let at_period = compute_light_intensity(0.25, 0.75, 100, 100, LightFunction::Linear, &mut r);
        assert!((at_0 - 0.25).abs() < 0.001, "linear@0 = {at_0}");
        assert!((at_period - 0.75).abs() < 0.001, "linear@period = {at_period}");
    }

    // ── box 2.11: smooth at phase=0 -> initial, at phase=period -> final ──

    #[test]
    fn smooth_endpoints() {
        let mut r = rng();
        let at_0 = compute_light_intensity(0.25, 0.75, 0, 100, LightFunction::Smooth, &mut r);
        let at_period = compute_light_intensity(0.25, 0.75, 100, 100, LightFunction::Smooth, &mut r);
        assert!((at_0 - 0.25).abs() < 0.001, "smooth@0 = {at_0}");
        assert!((at_period - 0.75).abs() < 0.001, "smooth@period = {at_period}");
    }

    #[test]
    fn zero_period_is_safe() {
        // period=0 must not divide-by-zero; degenerates without panicking.
        let mut r = rng();
        let _ = compute_light_intensity(0.0, 1.0, 0, 0, LightFunction::Linear, &mut r);
        let _ = compute_light_intensity(0.0, 1.0, 0, 0, LightFunction::Smooth, &mut r);
    }
}
