use crate::components::Media;

/// Compute the current media height based on an associated light's intensity.
///
/// The height interpolates between `height_low` and `height_high` based on intensity (0.0-1.0).
pub fn compute_media_height(media: &Media, light_intensity: f32) -> f32 {
    media.height_low + (media.height_high - media.height_low) * light_intensity
}

/// Media type constants.
pub const MEDIA_WATER: i16 = 0;
pub const MEDIA_LAVA: i16 = 1;
pub const MEDIA_GOO: i16 = 2;
pub const MEDIA_SEWAGE: i16 = 3;
pub const MEDIA_JJARO: i16 = 4;

/// Whether this media type deals damage to submerged entities.
pub fn media_deals_damage(media_type: i16) -> bool {
    matches!(media_type, MEDIA_LAVA | MEDIA_GOO | MEDIA_JJARO)
}

/// Drag factor for movement in media (0.0 = full drag, 1.0 = no drag).
pub fn media_drag_factor(media_type: i16) -> f32 {
    match media_type {
        MEDIA_WATER => 0.5,
        MEDIA_LAVA => 0.3,
        MEDIA_GOO => 0.2,
        MEDIA_SEWAGE => 0.4,
        MEDIA_JJARO => 0.3,
        _ => 1.0,
    }
}

/// Whether an eye/viewpoint at `eye_height` is submerged beneath a media surface
/// whose current height is `polygon_media_height`.
///
/// Submersion is strictly "below the surface": an eye exactly at the surface
/// height is treated as *not* submerged (matching the projectile-crossing
/// convention `z < media_height` used elsewhere in the sim).
pub fn is_submerged(eye_height: f32, polygon_media_height: f32) -> bool {
    eye_height < polygon_media_height
}

/// Per-type underwater tint colour applied as a fullscreen overlay when the
/// camera is submerged (render box 7.x consumes this). Returns straight RGBA in
/// 0.0..=1.0. The alpha is the overlay strength, not a physical opacity.
///
/// Tints are Marathon-flavoured: water is a cool blue, lava a hot orange/red,
/// goo a sickly green, sewage a murky brown-green, and JjaroGoo a teal/cyan.
/// Unknown media types fall back to a neutral, fully transparent tint.
pub fn media_tint_color(media_type: i16) -> [f32; 4] {
    match media_type {
        MEDIA_WATER => [0.10, 0.30, 0.70, 0.45],
        MEDIA_LAVA => [0.85, 0.25, 0.05, 0.70],
        MEDIA_GOO => [0.15, 0.65, 0.20, 0.55],
        MEDIA_SEWAGE => [0.35, 0.40, 0.20, 0.50],
        MEDIA_JJARO => [0.10, 0.60, 0.60, 0.50],
        _ => [0.0, 0.0, 0.0, 0.0],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_media(media_type: i16) -> Media {
        Media {
            index: 0,
            polygon_index: 0,
            media_type,
            height_low: 0.0,
            height_high: 2.0,
            light_index: 0,
            current_height: 1.0,
            current_direction: 0.0,
            current_magnitude: 0.0,
        }
    }

    #[test]
    fn media_height_at_min_intensity() {
        let media = make_media(MEDIA_WATER);
        assert_eq!(compute_media_height(&media, 0.0), 0.0);
    }

    #[test]
    fn media_height_at_max_intensity() {
        let media = make_media(MEDIA_WATER);
        assert_eq!(compute_media_height(&media, 1.0), 2.0);
    }

    #[test]
    fn media_height_at_half_intensity() {
        let media = make_media(MEDIA_WATER);
        assert_eq!(compute_media_height(&media, 0.5), 1.0);
    }

    #[test]
    fn lava_deals_damage() {
        assert!(media_deals_damage(MEDIA_LAVA));
        assert!(media_deals_damage(MEDIA_GOO));
        assert!(!media_deals_damage(MEDIA_WATER));
    }

    #[test]
    fn drag_factors() {
        assert!(media_drag_factor(MEDIA_WATER) > media_drag_factor(MEDIA_GOO));
    }

    #[test]
    fn submerged_when_eye_below_surface() {
        // Eye at 0.5, surface at 1.0 -> submerged.
        assert!(is_submerged(0.5, 1.0));
    }

    #[test]
    fn not_submerged_when_eye_above_surface() {
        // Eye at 1.5, surface at 1.0 -> not submerged.
        assert!(!is_submerged(1.5, 1.0));
    }

    #[test]
    fn not_submerged_when_eye_exactly_at_surface() {
        // Strictly-below convention: eye exactly at surface is not submerged.
        assert!(!is_submerged(1.0, 1.0));
    }

    #[test]
    fn tint_colors_per_media_type() {
        assert_eq!(media_tint_color(MEDIA_WATER), [0.10, 0.30, 0.70, 0.45]);
        assert_eq!(media_tint_color(MEDIA_LAVA), [0.85, 0.25, 0.05, 0.70]);
        assert_eq!(media_tint_color(MEDIA_GOO), [0.15, 0.65, 0.20, 0.55]);
        assert_eq!(media_tint_color(MEDIA_SEWAGE), [0.35, 0.40, 0.20, 0.50]);
        assert_eq!(media_tint_color(MEDIA_JJARO), [0.10, 0.60, 0.60, 0.50]);
        // Unknown type -> neutral, transparent.
        assert_eq!(media_tint_color(99), [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn water_tint_is_bluish() {
        // Sanity: water's blue channel dominates red/green.
        let [r, g, b, _a] = media_tint_color(MEDIA_WATER);
        assert!(b > r && b > g);
    }

    #[test]
    fn lava_tint_is_reddish() {
        // Sanity: lava's red channel dominates green/blue.
        let [r, g, b, _a] = media_tint_color(MEDIA_LAVA);
        assert!(r > g && r > b);
    }
}
