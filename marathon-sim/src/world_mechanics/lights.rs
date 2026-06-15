use crate::components::{
    Light, LightFunction, LightFunctionSpec, LightState, LIGHT_HAS_SLAVED_INTENSITIES,
};
use rand::Rng;

/// Smooth (cosine-eased) interpolation from `initial` to `final` across the
/// `[0, period]` phase window: `initial + (final - initial) * (cos(phase * PI /
/// period + PI) + 1) / 2`. At `phase == 0` it returns `initial`; at
/// `phase == period` it returns `final`. Used directly by `Smooth` and as the
/// deterministic base of `Flicker`.
fn smooth_intensity(initial: f32, final_intensity: f32, phase: u32, period: u32) -> f32 {
    let period = period.max(1) as f32;
    let eased =
        ((phase as f32 * std::f32::consts::PI / period + std::f32::consts::PI).cos() + 1.0) / 2.0;
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

/// Roll a state's effective `(period, final_intensity)` with Alephone-style
/// delta randomization: `period = base + rand(0..=delta_period)` and
/// `final = intensity + rand[0,1) * delta_intensity`. `period_spec` supplies the
/// timing; `intensity_spec` supplies the intensity endpoints (which may differ
/// from `period_spec` under slaved intensities).
fn roll_state(
    period_spec: &LightFunctionSpec,
    intensity_spec: &LightFunctionSpec,
    rng: &mut impl Rng,
) -> (u32, f32) {
    let span = period_spec.delta_period as u32 + 1;
    let period = (period_spec.period as u32 + rng.gen_range(0..span)).max(1);
    let final_intensity =
        intensity_spec.intensity + rng.gen::<f32>() * intensity_spec.delta_intensity;
    (period, final_intensity)
}

/// Index of the spec that supplies intensity values for `state`, honoring the
/// slaved-intensities flag: secondary states borrow the matching primary
/// state's intensity values (`secondary_active` ← `primary_active`,
/// `secondary_inactive` ← `primary_inactive`).
fn intensity_spec_index(light: &Light, state: LightState) -> usize {
    if light.flags & LIGHT_HAS_SLAVED_INTENSITIES != 0 {
        match state {
            LightState::SecondaryActive => LightState::PrimaryActive.as_index(),
            LightState::SecondaryInactive => LightState::PrimaryInactive.as_index(),
            _ => state.as_index(),
        }
    } else {
        state.as_index()
    }
}

/// Transition a light to its next state: snapshot the current intensity as the
/// new ramp's starting point, roll a fresh period and target intensity (with
/// delta randomization and slaved-intensity handling), and reset the phase.
pub fn advance_light_state(light: &mut Light, rng: &mut impl Rng) {
    let next = light.state.next_state();
    light.initial_intensity = light.current_intensity;
    let period_spec = light.functions[next.as_index()];
    let intensity_spec = light.functions[intensity_spec_index(light, next)];
    let (period, final_intensity) = roll_state(&period_spec, &intensity_spec, rng);
    light.period = period;
    light.final_intensity = final_intensity;
    light.phase = 0;
    light.state = next;
}

/// Advance one light by a single tick: increment the phase, transition to the
/// next state once the period elapses, then evaluate the current state's
/// function to update `current_intensity`.
pub fn update_single_light(light: &mut Light, rng: &mut impl Rng) {
    light.phase += 1;
    if light.phase >= light.period {
        advance_light_state(light, rng);
    }
    let function = light.functions[light.state.as_index()].function;
    light.current_intensity = compute_light_intensity(
        light.initial_intensity,
        light.final_intensity,
        light.phase,
        light.period,
        function,
        rng,
    );
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
            assert!(
                v >= base - 1e-4 && v <= 0.9 + 1e-4,
                "flicker {v} out of [{base},0.9]"
            );
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
        let at_period =
            compute_light_intensity(0.25, 0.75, 100, 100, LightFunction::Linear, &mut r);
        assert!((at_0 - 0.25).abs() < 0.001, "linear@0 = {at_0}");
        assert!(
            (at_period - 0.75).abs() < 0.001,
            "linear@period = {at_period}"
        );
    }

    // ── box 2.11: smooth at phase=0 -> initial, at phase=period -> final ──

    #[test]
    fn smooth_endpoints() {
        let mut r = rng();
        let at_0 = compute_light_intensity(0.25, 0.75, 0, 100, LightFunction::Smooth, &mut r);
        let at_period =
            compute_light_intensity(0.25, 0.75, 100, 100, LightFunction::Smooth, &mut r);
        assert!((at_0 - 0.25).abs() < 0.001, "smooth@0 = {at_0}");
        assert!(
            (at_period - 0.75).abs() < 0.001,
            "smooth@period = {at_period}"
        );
    }

    #[test]
    fn zero_period_is_safe() {
        // period=0 must not divide-by-zero; degenerates without panicking.
        let mut r = rng();
        let _ = compute_light_intensity(0.0, 1.0, 0, 0, LightFunction::Linear, &mut r);
        let _ = compute_light_intensity(0.0, 1.0, 0, 0, LightFunction::Smooth, &mut r);
    }

    // ── Light state machine (boxes 3.1–3.8) ──

    use crate::components::LightType;

    fn spec(
        function: LightFunction,
        period: u16,
        delta_period: u16,
        intensity: f32,
        delta_intensity: f32,
    ) -> LightFunctionSpec {
        LightFunctionSpec {
            function,
            period,
            delta_period,
            intensity,
            delta_intensity,
        }
    }

    fn test_light(flags: u16, functions: [LightFunctionSpec; 6]) -> Light {
        Light {
            light_index: 0,
            light_type: LightType::Normal,
            state: LightState::BecomingActive,
            flags,
            phase: 0,
            period: (functions[0].period as u32).max(1),
            current_intensity: 0.0,
            initial_intensity: 0.0,
            final_intensity: functions[0].intensity,
            functions,
            tag: 0,
        }
    }

    fn const_specs(period: u16) -> [LightFunctionSpec; 6] {
        [
            spec(LightFunction::Constant, period, 0, 1.0, 0.0),
            spec(LightFunction::Constant, period, 0, 1.0, 0.0),
            spec(LightFunction::Constant, period, 0, 1.0, 0.0),
            spec(LightFunction::Constant, period, 0, 0.0, 0.0),
            spec(LightFunction::Constant, period, 0, 0.0, 0.0),
            spec(LightFunction::Constant, period, 0, 0.0, 0.0),
        ]
    }

    #[test]
    fn transitions_to_next_state_when_period_elapses() {
        // box 3.4: becoming_active -> primary_active at phase == period.
        let mut light = test_light(0, const_specs(5));
        let mut r = rng();
        for _ in 0..4 {
            update_single_light(&mut light, &mut r);
            assert_eq!(light.state, LightState::BecomingActive);
        }
        update_single_light(&mut light, &mut r); // 5th tick: phase reaches period
        assert_eq!(light.state, LightState::PrimaryActive);
    }

    #[test]
    fn full_six_state_cycle_returns_to_becoming_active() {
        // box 3.5: six transitions wrap back to BecomingActive.
        let mut light = test_light(0, const_specs(1));
        let mut r = rng();
        let order = [
            LightState::PrimaryActive,
            LightState::SecondaryActive,
            LightState::BecomingInactive,
            LightState::PrimaryInactive,
            LightState::SecondaryInactive,
            LightState::BecomingActive,
        ];
        for expected in order {
            advance_light_state(&mut light, &mut r);
            assert_eq!(light.state, expected);
        }
    }

    #[test]
    fn delta_period_varies_period_across_transitions() {
        // box 3.6: delta_period > 0 yields varied rolled periods.
        let mut funcs = const_specs(10);
        for f in funcs.iter_mut() {
            f.delta_period = 20;
        }
        let mut light = test_light(0, funcs);
        let mut r = rng();
        let mut periods = Vec::new();
        for _ in 0..8 {
            advance_light_state(&mut light, &mut r);
            periods.push(light.period);
        }
        assert!(
            periods.iter().any(|&p| p != periods[0]),
            "expected varied periods, got {periods:?}"
        );
        assert!(periods.iter().all(|&p| (10..=30).contains(&p)));
    }

    #[test]
    fn delta_intensity_varies_final_across_transitions() {
        // box 3.7: delta_intensity > 0 yields varied rolled final intensities.
        let mut funcs = const_specs(1);
        for f in funcs.iter_mut() {
            f.intensity = 0.0;
            f.delta_intensity = 1.0;
        }
        let mut light = test_light(0, funcs);
        let mut r = rng();
        let mut finals = Vec::new();
        for _ in 0..8 {
            advance_light_state(&mut light, &mut r);
            finals.push(light.final_intensity);
        }
        assert!(finals.iter().any(|&v| (v - finals[0]).abs() > f32::EPSILON));
        assert!(finals.iter().all(|&v| (0.0..=1.0).contains(&v)));
    }

    #[test]
    fn initial_intensity_inherits_previous_current() {
        // box 3.8: on transition, initial_intensity = prior current_intensity.
        let mut light = test_light(0, const_specs(1));
        light.current_intensity = 0.42;
        let mut r = rng();
        advance_light_state(&mut light, &mut r);
        assert!((light.initial_intensity - 0.42).abs() < f32::EPSILON);
    }

    #[test]
    fn slaved_intensities_borrow_primary_values() {
        // box 3.2: with the slaved flag, secondary states use the matching
        // primary state's intensity endpoints.
        let mut funcs = const_specs(1);
        funcs[LightState::PrimaryActive.as_index()].intensity = 0.7;
        funcs[LightState::SecondaryActive.as_index()].intensity = 0.1; // ignored when slaved
        let mut light = test_light(LIGHT_HAS_SLAVED_INTENSITIES, funcs);
        light.state = LightState::PrimaryActive; // next_state -> SecondaryActive
        let mut r = rng();
        advance_light_state(&mut light, &mut r);
        assert_eq!(light.state, LightState::SecondaryActive);
        assert!(
            (light.final_intensity - 0.7).abs() < f32::EPSILON,
            "slaved secondary should use primary's 0.7, got {}",
            light.final_intensity
        );
    }
}
