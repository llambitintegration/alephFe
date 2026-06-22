//! Full-screen fader effect system (shared between `marathon-game` and `marathon-web`).
//!
//! Faders are ephemeral screen-space visual overlays triggered by game events
//! (damage, teleport, invincibility, etc.). This module holds the pure-data
//! types for the fader system; the renderers own a `FaderManager` and translate
//! its active faders into post-process draw calls. See
//! `openspec/changes/implement-fullscreen-effects/design.md`.

/// Compositing mode for a full-screen fader overlay.
///
/// The six Marathon blend modes, each applied in the post-process fragment
/// shader (`fader.wgsl`). The discriminant doubles as the `mode` index written
/// into the fader uniform buffer and consumed by the shader's mode switch — see
/// the Blend Mode Specifications table in
/// `openspec/changes/implement-fullscreen-effects/design.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum FaderBlendMode {
    /// Push scene colors toward the fader color: `mix(scene, scene * color, intensity)`.
    Tint = 0,
    /// Per-pixel noise-modulated tint (static / interference).
    Randomize = 1,
    /// Color inversion blended by intensity: `mix(scene, 1.0 - scene, intensity)`.
    Negate = 2,
    /// Additive brightening: `scene + color * intensity`.
    Dodge = 3,
    /// Subtractive darkening: `scene - color * intensity`.
    Burn = 4,
    /// Gentle tint for sustained effects: `mix(scene, scene * color, intensity * 0.5)`.
    SoftTint = 5,
}

/// Deduplication tag identifying which game effect owns a fader.
///
/// Sustained effects (invincibility, oxygen, lava, infravision) trigger a fader
/// every frame; the tag lets `FaderManager` replace the existing fader with the
/// same tag instead of accumulating duplicates. `None` marks one-shot faders
/// (e.g. damage, teleport, shield) that are never deduplicated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FaderTag {
    /// Player took damage (red tint flash).
    Damage,
    /// Level teleport (white randomize static).
    Teleport,
    /// Invincibility powerup active (gold-green soft tint glow).
    Invincibility,
    /// Low oxygen warning (blue-gray soft tint).
    Oxygen,
    /// Shield recharge (blue-white dodge).
    Shield,
    /// Lava submersion (orange-red burn).
    Lava,
    /// Infravision active (green soft tint).
    Infravision,
    /// No dedup tag; a one-shot fader that is never replaced.
    None,
}

/// A single active full-screen fader overlay.
///
/// Pure-data record of one in-flight fader effect. The renderer reads
/// `color`/`blend_mode`/intensity to drive the post-process pass; `FaderManager`
/// owns a `Vec<ActiveFader>`, ticks `remaining_ticks` down each frame, and
/// recomputes the live intensity from `initial_intensity * (remaining / total)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActiveFader {
    /// RGBA fader color written into the fader uniform buffer.
    pub color: [f32; 4],
    /// Compositing mode applied in the fader fragment shader.
    pub blend_mode: FaderBlendMode,
    /// Intensity at the moment the fader was triggered (1.0 = full strength).
    pub initial_intensity: f32,
    /// Ticks remaining before the fader expires.
    pub remaining_ticks: u16,
    /// Total lifetime in ticks, used to compute the intensity ramp.
    pub total_ticks: u16,
    /// Dedup tag identifying the owning effect.
    pub tag: FaderTag,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fader_blend_mode_six_distinct_variants() {
        // All six Marathon blend modes must exist and be distinct.
        let variants = [
            FaderBlendMode::Tint,
            FaderBlendMode::Randomize,
            FaderBlendMode::Negate,
            FaderBlendMode::Dodge,
            FaderBlendMode::Burn,
            FaderBlendMode::SoftTint,
        ];

        // Six variants present.
        assert_eq!(variants.len(), 6);

        // Pairwise distinct (relies on PartialEq/Eq).
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b, "variants {i} and {j} must be distinct");
                }
            }
        }

        // Discriminants match the shader mode indices documented in design.md.
        assert_eq!(FaderBlendMode::Tint as u32, 0);
        assert_eq!(FaderBlendMode::Randomize as u32, 1);
        assert_eq!(FaderBlendMode::Negate as u32, 2);
        assert_eq!(FaderBlendMode::Dodge as u32, 3);
        assert_eq!(FaderBlendMode::Burn as u32, 4);
        assert_eq!(FaderBlendMode::SoftTint as u32, 5);

        // Copy + Debug derives are usable.
        let copied = variants[0];
        let _ = format!("{copied:?}");
        assert_eq!(copied, FaderBlendMode::Tint);
    }

    #[test]
    fn test_active_fader_fields_and_dedup_tags() {
        // Construct an ActiveFader with all documented fields.
        let fader = ActiveFader {
            color: [1.0, 0.0, 0.0, 1.0],
            blend_mode: FaderBlendMode::Tint,
            initial_intensity: 0.75,
            remaining_ticks: 8,
            total_ticks: 10,
            tag: FaderTag::Damage,
        };

        // Field values round-trip exactly.
        assert_eq!(fader.color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(fader.blend_mode, FaderBlendMode::Tint);
        assert_eq!(fader.initial_intensity, 0.75);
        assert_eq!(fader.remaining_ticks, 8u16);
        assert_eq!(fader.total_ticks, 10u16);
        assert_eq!(fader.tag, FaderTag::Damage);

        // The dedup tag enum has exactly the eight documented variants, all distinct.
        let tags = [
            FaderTag::Damage,
            FaderTag::Teleport,
            FaderTag::Invincibility,
            FaderTag::Oxygen,
            FaderTag::Shield,
            FaderTag::Lava,
            FaderTag::Infravision,
            FaderTag::None,
        ];
        assert_eq!(tags.len(), 8);
        for (i, a) in tags.iter().enumerate() {
            for (j, b) in tags.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b, "tags {i} and {j} must be distinct");
                }
            }
        }

        // Copy + Clone + Debug derives are usable on the struct.
        let cloned = fader.clone();
        let copied = fader;
        let _ = format!("{copied:?}");
        assert_eq!(cloned.tag, copied.tag);
    }
}
