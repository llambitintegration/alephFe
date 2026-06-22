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
}
