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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_media(media_type: i16) -> Media {
        Media {
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
}
