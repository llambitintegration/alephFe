use std::collections::HashMap;

use marathon_formats::map::{Line, LineFlags, Polygon};
use marathon_formats::sounds::SoundBehavior;

use crate::types::AttenuationParams;

// ─── Distance Attenuation ──────────────────────────────────────────────────

/// Compute volume multiplier from 2D distance for a given sound behavior.
///
/// Returns a value from 0.0 (inaudible) to 1.0 (full volume).
pub fn distance_attenuation(distance: f32, behavior: SoundBehavior) -> f32 {
    let params = AttenuationParams::for_behavior(behavior);
    if distance <= 0.0 {
        return 1.0;
    }
    if distance >= params.max_distance {
        return 0.0;
    }
    let normalized = distance / params.max_distance;
    (1.0 - normalized.powf(params.falloff_exponent)).max(0.0)
}

/// Compute the 2D distance between two points (XY plane only).
pub fn distance_2d(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    (dx * dx + dy * dy).sqrt()
}

/// Check if a sound at the given distance is within audible range for a behavior.
pub fn is_within_range(distance: f32, behavior: SoundBehavior) -> bool {
    let params = AttenuationParams::for_behavior(behavior);
    distance < params.max_distance
}

/// Get the maximum audible distance for a behavior type.
pub fn max_distance(behavior: SoundBehavior) -> f32 {
    AttenuationParams::for_behavior(behavior).max_distance
}

// ─── Directional Panning ───────────────────────────────────────────────────

/// Compute stereo panning based on the angle between listener facing and sound source.
///
/// `listener_facing`: listener's facing angle in radians (0 = east, CCW positive).
/// `listener_x`, `listener_y`: listener position.
/// `source_x`, `source_y`: sound source position.
///
/// Returns a pan value from -1.0 (full left) to 1.0 (full right).
/// Also returns a rear attenuation factor (1.0 = no attenuation, reduced for sounds behind).
pub fn directional_pan(
    listener_facing: f32,
    listener_x: f32,
    listener_y: f32,
    source_x: f32,
    source_y: f32,
) -> (f32, f32) {
    let dx = source_x - listener_x;
    let dy = source_y - listener_y;

    // If source is at listener position, center with no attenuation
    if dx == 0.0 && dy == 0.0 {
        return (0.0, 1.0);
    }

    let angle_to_source = dy.atan2(dx);
    let relative_angle = normalize_angle(angle_to_source - listener_facing);

    // Pan: negate sin of relative angle so that positive = right, negative = left.
    // In standard math coords, sin is positive for CCW (left) and negative for CW (right).
    let pan = -relative_angle.sin();

    // Rear attenuation: reduce volume for sounds behind the listener.
    // cos(relative_angle) is 1.0 for ahead, -1.0 for behind.
    // Map to attenuation: 1.0 ahead, 0.5 behind.
    let cos_angle = relative_angle.cos();
    let rear_attenuation = 0.75 + 0.25 * cos_angle;

    (pan.clamp(-1.0, 1.0), rear_attenuation)
}

/// Normalize an angle to the range [-PI, PI].
fn normalize_angle(angle: f32) -> f32 {
    let mut a = angle % std::f32::consts::TAU;
    if a > std::f32::consts::PI {
        a -= std::f32::consts::TAU;
    } else if a < -std::f32::consts::PI {
        a += std::f32::consts::TAU;
    }
    a
}

// ─── Permutation Selection ─────────────────────────────────────────────────

/// Tracks the last-played permutation index per sound definition.
pub struct PermutationTracker {
    last_played: HashMap<usize, usize>,
}

impl PermutationTracker {
    pub fn new() -> Self {
        Self {
            last_played: HashMap::new(),
        }
    }

    /// Select a random permutation for a sound definition, avoiding the last-played one.
    ///
    /// `sound_index`: index into the sound definitions array.
    /// `permutation_count`: total number of permutations for this definition.
    /// `rng_value`: a random f32 in [0.0, 1.0) for selection.
    ///
    /// Returns the selected permutation index.
    pub fn select(&mut self, sound_index: usize, permutation_count: usize, rng_value: f32) -> usize {
        if permutation_count == 0 {
            return 0;
        }
        if permutation_count == 1 {
            self.last_played.insert(sound_index, 0);
            return 0;
        }

        let last = self.last_played.get(&sound_index).copied();
        let candidates: Vec<usize> = (0..permutation_count)
            .filter(|&i| Some(i) != last)
            .collect();

        let idx = (rng_value * candidates.len() as f32) as usize;
        let selected = candidates[idx.min(candidates.len() - 1)];
        self.last_played.insert(sound_index, selected);
        selected
    }

    /// Clear all tracking state (e.g., on level transition).
    pub fn clear(&mut self) {
        self.last_played.clear();
    }
}

// ─── Volume/Pitch Randomization ────────────────────────────────────────────

/// Compute a randomized volume given base and delta values.
///
/// Returns base + rng_value * delta, clamped to [0.0, 1.0].
pub fn randomize_volume(base: f32, delta: f32, rng_value: f32) -> f32 {
    (base + rng_value * delta).clamp(0.0, 1.0)
}

/// Compute a randomized pitch given low and high pitch bounds.
///
/// Returns low_pitch + rng_value * (high_pitch - low_pitch).
pub fn randomize_pitch(low_pitch: f32, high_pitch: f32, rng_value: f32) -> f32 {
    if high_pitch <= low_pitch {
        return low_pitch;
    }
    low_pitch + rng_value * (high_pitch - low_pitch)
}

/// Check if a sound should play based on its chance value.
///
/// `chance`: the chance field from SoundDefinition (0 = always play).
/// `rng_value`: random u16 value for comparison.
pub fn passes_chance_gate(chance: u16, rng_value: u16) -> bool {
    chance == 0 || rng_value < chance
}

// ─── Wall Obstruction ──────────────────────────────────────────────────────

/// Full obstruction amount added per solid wall crossing.
const FULL_OBSTRUCTION: f32 = 0.25;
/// Partial obstruction for transparent/windowed walls.
const PARTIAL_OBSTRUCTION: f32 = 0.1;
/// Maximum total obstruction (clamped).
const MAX_OBSTRUCTION: f32 = 1.0;

/// Cache for obstruction values between polygon pairs.
pub struct ObstructionCache {
    cache: HashMap<(i16, i16), f32>,
    last_listener_polygon: i16,
}

impl ObstructionCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            last_listener_polygon: -1,
        }
    }

    /// Invalidate the cache if the listener moved to a new polygon.
    pub fn update_listener_polygon(&mut self, polygon_index: i16) {
        if polygon_index != self.last_listener_polygon {
            self.cache.clear();
            self.last_listener_polygon = polygon_index;
        }
    }

    /// Get cached obstruction value, or compute and cache it.
    pub fn get_or_compute(
        &mut self,
        source_polygon: i16,
        listener_polygon: i16,
        polygons: &[Polygon],
        lines: &[Line],
    ) -> f32 {
        let key = (source_polygon, listener_polygon);
        if let Some(&cached) = self.cache.get(&key) {
            return cached;
        }
        let value = compute_obstruction(source_polygon, listener_polygon, polygons, lines);
        self.cache.insert(key, value);
        value
    }

    /// Clear all cached values.
    pub fn clear(&mut self) {
        self.cache.clear();
        self.last_listener_polygon = -1;
    }
}

/// Compute sound obstruction between two polygons using BFS through the adjacency graph.
///
/// Returns an obstruction factor from 0.0 (no obstruction) to 1.0 (fully obstructed).
pub fn compute_obstruction(
    source_polygon: i16,
    listener_polygon: i16,
    polygons: &[Polygon],
    lines: &[Line],
) -> f32 {
    if source_polygon == listener_polygon {
        return 0.0;
    }
    if source_polygon < 0 || listener_polygon < 0 {
        return MAX_OBSTRUCTION;
    }

    let source_idx = source_polygon as usize;
    let listener_idx = listener_polygon as usize;
    if source_idx >= polygons.len() || listener_idx >= polygons.len() {
        return MAX_OBSTRUCTION;
    }

    // BFS from source to listener through polygon adjacency
    let mut visited = vec![false; polygons.len()];
    // (polygon_index, accumulated_obstruction)
    let mut queue = std::collections::VecDeque::new();
    queue.push_back((source_idx, 0.0f32));
    visited[source_idx] = true;

    // Track best (lowest) obstruction path to listener
    let mut best_obstruction = MAX_OBSTRUCTION;

    while let Some((current_idx, obstruction)) = queue.pop_front() {
        if current_idx == listener_idx {
            best_obstruction = best_obstruction.min(obstruction);
            continue;
        }

        // Prune paths that already exceed best known
        if obstruction >= best_obstruction {
            continue;
        }

        let polygon = &polygons[current_idx];
        let vertex_count = polygon.vertex_count as usize;

        for side in 0..vertex_count.min(8) {
            let line_idx = polygon.line_indexes[side];
            if line_idx < 0 {
                continue;
            }

            let adj_poly_idx = polygon.adjacent_polygon_indexes[side];
            if adj_poly_idx < 0 {
                continue;
            }
            let adj_idx = adj_poly_idx as usize;
            if adj_idx >= polygons.len() || visited[adj_idx] {
                continue;
            }

            let line_idx_usize = line_idx as usize;
            if line_idx_usize >= lines.len() {
                continue;
            }

            let line = &lines[line_idx_usize];
            let line_flags = line.line_flags();

            // Compute obstruction added by this line
            let added = if line_flags.contains(LineFlags::SOLID)
                && !line_flags.contains(LineFlags::HAS_TRANSPARENT_SIDE)
                && !line_flags.contains(LineFlags::TRANSPARENT)
            {
                FULL_OBSTRUCTION
            } else if line_flags.contains(LineFlags::HAS_TRANSPARENT_SIDE)
                || line_flags.contains(LineFlags::TRANSPARENT)
            {
                PARTIAL_OBSTRUCTION
            } else {
                0.0
            };

            let new_obstruction = (obstruction + added).min(MAX_OBSTRUCTION);

            visited[adj_idx] = true;
            queue.push_back((adj_idx, new_obstruction));
        }
    }

    best_obstruction
}

// ─── Media Obstruction ─────────────────────────────────────────────────────

/// Low-pass filter cutoff for moderate muffling (Water, Sewage).
pub const MEDIA_CUTOFF_MODERATE: f32 = 800.0;
/// Low-pass filter cutoff for heavy muffling (Lava, Goo, Jjaro).
pub const MEDIA_CUTOFF_HEAVY: f32 = 400.0;

/// Check if an entity at the given height is submerged in the media of a polygon.
///
/// `polygon_media_index`: the polygon's media_index field (-1 = no media).
/// `media_heights`: slice of media heights indexed by media index.
/// `entity_z`: the entity's vertical position.
///
/// Returns the media type index if submerged, or None.
pub fn check_submersion(
    polygon_media_index: i16,
    media_heights: &[i16],
    entity_z: f32,
) -> Option<usize> {
    if polygon_media_index < 0 {
        return None;
    }
    let idx = polygon_media_index as usize;
    if idx >= media_heights.len() {
        return None;
    }
    if entity_z < media_heights[idx] as f32 {
        Some(idx)
    } else {
        None
    }
}

/// Get the low-pass filter cutoff frequency for a media type.
///
/// `media_type`: the media_type field from MediaData (0=Water, 1=Lava, 2=Goo, 3=Sewage, 4=Jjaro).
pub fn media_filter_cutoff(media_type: i16) -> f32 {
    match media_type {
        0 | 3 => MEDIA_CUTOFF_MODERATE, // Water, Sewage
        1 | 2 | 4 => MEDIA_CUTOFF_HEAVY, // Lava, Goo, Jjaro
        _ => MEDIA_CUTOFF_MODERATE,
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // -- Distance attenuation tests --

    #[test]
    fn test_attenuation_at_zero_distance() {
        assert_eq!(distance_attenuation(0.0, SoundBehavior::Normal), 1.0);
        assert_eq!(distance_attenuation(0.0, SoundBehavior::Quiet), 1.0);
        assert_eq!(distance_attenuation(0.0, SoundBehavior::Loud), 1.0);
    }

    #[test]
    fn test_attenuation_at_max_distance() {
        let quiet_max = AttenuationParams::for_behavior(SoundBehavior::Quiet).max_distance;
        assert_eq!(distance_attenuation(quiet_max, SoundBehavior::Quiet), 0.0);

        let normal_max = AttenuationParams::for_behavior(SoundBehavior::Normal).max_distance;
        assert_eq!(distance_attenuation(normal_max, SoundBehavior::Normal), 0.0);

        let loud_max = AttenuationParams::for_behavior(SoundBehavior::Loud).max_distance;
        assert_eq!(distance_attenuation(loud_max, SoundBehavior::Loud), 0.0);
    }

    #[test]
    fn test_attenuation_beyond_max() {
        let max = AttenuationParams::for_behavior(SoundBehavior::Normal).max_distance;
        assert_eq!(distance_attenuation(max + 1000.0, SoundBehavior::Normal), 0.0);
    }

    #[test]
    fn test_attenuation_midpoint_normal() {
        let max = AttenuationParams::for_behavior(SoundBehavior::Normal).max_distance;
        let mid = max / 2.0;
        let vol = distance_attenuation(mid, SoundBehavior::Normal);
        // Linear falloff at half distance should give 0.5
        assert!((vol - 0.5).abs() < 0.01, "Normal midpoint volume: {vol}");
    }

    #[test]
    fn test_attenuation_quiet_steeper_than_normal() {
        let distance = 3.0 * 1024.0;
        let quiet_vol = distance_attenuation(distance, SoundBehavior::Quiet);
        let normal_vol = distance_attenuation(distance, SoundBehavior::Normal);
        assert!(quiet_vol < normal_vol, "Quiet {quiet_vol} should be less than Normal {normal_vol}");
    }

    #[test]
    fn test_attenuation_loud_more_than_normal() {
        let distance = 8.0 * 1024.0;
        let normal_vol = distance_attenuation(distance, SoundBehavior::Normal);
        let loud_vol = distance_attenuation(distance, SoundBehavior::Loud);
        assert!(loud_vol > normal_vol, "Loud {loud_vol} should be more than Normal {normal_vol}");
    }

    // -- Distance calculation tests --

    #[test]
    fn test_distance_2d_same_point() {
        assert_eq!(distance_2d(0.0, 0.0, 0.0, 0.0), 0.0);
    }

    #[test]
    fn test_distance_2d_known_values() {
        let d = distance_2d(0.0, 0.0, 3.0, 4.0);
        assert!((d - 5.0).abs() < 0.001);
    }

    // -- Panning tests --

    #[test]
    fn test_pan_sound_ahead() {
        let (pan, rear) = directional_pan(0.0, 0.0, 0.0, 10.0, 0.0);
        assert!(pan.abs() < 0.01, "Ahead should be centered, got {pan}");
        assert!(rear > 0.9, "No rear attenuation ahead, got {rear}");
    }

    #[test]
    fn test_pan_sound_right() {
        // Listener facing east (0 rad), sound to the south (negative Y)
        let (pan, _) = directional_pan(0.0, 0.0, 0.0, 0.0, -10.0);
        assert!(pan > 0.9, "Sound to the right should pan right, got {pan}");
    }

    #[test]
    fn test_pan_sound_left() {
        // Listener facing east (0 rad), sound to the north (positive Y)
        let (pan, _) = directional_pan(0.0, 0.0, 0.0, 0.0, 10.0);
        assert!(pan < -0.9, "Sound to the left should pan left, got {pan}");
    }

    #[test]
    fn test_pan_sound_behind() {
        let (pan, rear) = directional_pan(0.0, 0.0, 0.0, -10.0, 0.0);
        assert!(pan.abs() < 0.01, "Behind should be centered, got {pan}");
        assert!(rear < 0.6, "Behind should have rear attenuation, got {rear}");
    }

    #[test]
    fn test_pan_source_at_listener() {
        let (pan, rear) = directional_pan(0.0, 5.0, 5.0, 5.0, 5.0);
        assert_eq!(pan, 0.0);
        assert_eq!(rear, 1.0);
    }

    // -- Permutation selection tests --

    #[test]
    fn test_permutation_single() {
        let mut tracker = PermutationTracker::new();
        assert_eq!(tracker.select(0, 1, 0.5), 0);
    }

    #[test]
    fn test_permutation_avoids_repeat() {
        let mut tracker = PermutationTracker::new();
        // First play: any permutation
        let first = tracker.select(0, 3, 0.0);
        // Second play: should not get the same one
        let second = tracker.select(0, 3, 0.0);
        assert_ne!(first, second, "Should not repeat immediately");
    }

    #[test]
    fn test_permutation_different_sounds_independent() {
        let mut tracker = PermutationTracker::new();
        tracker.select(0, 3, 0.0);
        // Sound index 1 should not be affected by sound index 0
        let result = tracker.select(1, 3, 0.0);
        assert_eq!(result, 0); // First candidate with rng 0.0
    }

    #[test]
    fn test_permutation_clear() {
        let mut tracker = PermutationTracker::new();
        tracker.select(0, 3, 0.0);
        tracker.clear();
        // After clear, should select from all permutations again
        let result = tracker.select(0, 3, 0.0);
        assert_eq!(result, 0);
    }

    // -- Volume/pitch randomization tests --

    #[test]
    fn test_randomize_volume_no_delta() {
        assert!((randomize_volume(0.8, 0.0, 0.5) - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_randomize_volume_with_delta() {
        let vol = randomize_volume(0.5, 0.4, 0.5);
        assert!((vol - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_randomize_volume_clamped() {
        assert_eq!(randomize_volume(0.9, 0.5, 1.0), 1.0);
    }

    #[test]
    fn test_randomize_pitch_range() {
        let pitch = randomize_pitch(0.8, 1.2, 0.5);
        assert!((pitch - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_randomize_pitch_equal_bounds() {
        assert_eq!(randomize_pitch(1.0, 1.0, 0.5), 1.0);
    }

    // -- Chance gating tests --

    #[test]
    fn test_chance_zero_always_plays() {
        assert!(passes_chance_gate(0, 12345));
    }

    #[test]
    fn test_chance_gate_passes() {
        assert!(passes_chance_gate(100, 50));
    }

    #[test]
    fn test_chance_gate_fails() {
        assert!(!passes_chance_gate(100, 200));
    }

    // -- Obstruction tests --

    #[test]
    fn test_obstruction_same_polygon() {
        assert_eq!(compute_obstruction(0, 0, &[], &[]), 0.0);
    }

    #[test]
    fn test_obstruction_negative_index() {
        assert_eq!(compute_obstruction(-1, 0, &[], &[]), MAX_OBSTRUCTION);
    }

    #[test]
    fn test_obstruction_cache_invalidation() {
        let mut cache = ObstructionCache::new();
        cache.last_listener_polygon = 5;
        cache.cache.insert((0, 5), 0.3);

        // Same polygon -> cache should not be cleared
        cache.update_listener_polygon(5);
        assert!(cache.cache.contains_key(&(0, 5)));

        // Different polygon -> cache should be cleared
        cache.update_listener_polygon(6);
        assert!(cache.cache.is_empty());
    }

    // -- Media obstruction tests --

    #[test]
    fn test_submersion_no_media() {
        assert_eq!(check_submersion(-1, &[], 0.0), None);
    }

    #[test]
    fn test_submersion_above_media() {
        assert_eq!(check_submersion(0, &[100], 200.0), None);
    }

    #[test]
    fn test_submersion_below_media() {
        assert_eq!(check_submersion(0, &[200], 100.0), Some(0));
    }

    #[test]
    fn test_media_filter_cutoff_water() {
        assert_eq!(media_filter_cutoff(0), MEDIA_CUTOFF_MODERATE);
    }

    #[test]
    fn test_media_filter_cutoff_lava() {
        assert_eq!(media_filter_cutoff(1), MEDIA_CUTOFF_HEAVY);
    }

    #[test]
    fn test_media_filter_cutoff_sewage() {
        assert_eq!(media_filter_cutoff(3), MEDIA_CUTOFF_MODERATE);
    }

    #[test]
    fn test_is_within_range() {
        assert!(is_within_range(0.0, SoundBehavior::Quiet));
        let max = AttenuationParams::for_behavior(SoundBehavior::Quiet).max_distance;
        assert!(!is_within_range(max + 1.0, SoundBehavior::Quiet));
    }
}
