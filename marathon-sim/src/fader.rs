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

impl ActiveFader {
    /// The fader's live intensity, decayed from `initial_intensity` as the
    /// fader ages: `initial_intensity * (remaining_ticks / total_ticks)`.
    ///
    /// Ramps linearly from `initial_intensity` (when freshly triggered) down to
    /// `0.0` (when `remaining_ticks` reaches `0`). A fader with `total_ticks == 0`
    /// has no lifetime, so its current intensity is `0.0`.
    pub fn current_intensity(&self) -> f32 {
        if self.total_ticks == 0 {
            return 0.0;
        }
        self.initial_intensity * (self.remaining_ticks as f32 / self.total_ticks as f32)
    }
}

/// Owns and drives the set of active full-screen faders.
///
/// The manager holds a flat `Vec<ActiveFader>` that the game loop drives each
/// frame: `trigger()` adds a fader (replacing any existing fader with a matching
/// dedup tag), `tick()` ages faders down and removes expired ones, and
/// `active_faders()` exposes the live set for the renderer to composite. See
/// `openspec/changes/implement-fullscreen-effects/design.md` (Decision 1).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FaderManager {
    /// All currently-active faders, oldest first.
    faders: Vec<ActiveFader>,
}

impl FaderManager {
    /// Create an empty `FaderManager`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new active fader, replacing any existing fader with the same tag.
    ///
    /// For a tagged fader (`tag != FaderTag::None`), any existing fader carrying
    /// the same tag is removed before the new one is pushed, so sustained effects
    /// that re-trigger every frame (oxygen, invincibility, lava, infravision)
    /// never accumulate duplicates. Untagged faders (`FaderTag::None`) are one-shot
    /// and always appended, allowing multiple concurrent one-shots (e.g. stacked
    /// damage flashes). The new fader is appended last, preserving oldest-first
    /// insertion order for the renderer.
    pub fn trigger(&mut self, fader: ActiveFader) {
        if fader.tag != FaderTag::None {
            self.faders.retain(|f| f.tag != fader.tag);
        }
        self.faders.push(fader);
    }

    /// Advance all active faders by one tick and drop expired ones.
    ///
    /// Each surviving fader has its `remaining_ticks` decremented (saturating at
    /// `0`); the live intensity ramp is `initial_intensity * remaining / total`,
    /// recomputed on demand by [`ActiveFader::current_intensity`]. Faders whose
    /// `remaining_ticks` reaches `0` are expired and removed from the active set.
    pub fn tick(&mut self) {
        for fader in &mut self.faders {
            fader.remaining_ticks = fader.remaining_ticks.saturating_sub(1);
        }
        self.faders.retain(|fader| fader.remaining_ticks > 0);
    }

    /// The currently-active faders, for the renderer to composite.
    pub fn active_faders(&self) -> &[ActiveFader] {
        &self.faders
    }

    /// Remove every active fader carrying the given dedup tag.
    pub fn remove_by_tag(&mut self, tag: FaderTag) {
        self.faders.retain(|f| f.tag != tag);
    }

    /// Remove all active faders.
    pub fn clear(&mut self) {
        self.faders.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Box 1.3: the `FaderManager` surface exists and the trivial accessors
    /// behave. `tick()`/`trigger()` behaviour is deferred to boxes 1.4/1.5; here
    /// we only assert the method surface is present and `clear()`/`active_faders()`/
    /// `remove_by_tag()` are usable.
    #[test]
    fn test_fader_manager_surface() {
        // Construct an empty manager.
        let mut manager = FaderManager::default();
        assert!(
            manager.active_faders().is_empty(),
            "a fresh FaderManager has no active faders"
        );

        // `remove_by_tag()` is callable on an empty manager (no-op, no panic).
        manager.remove_by_tag(FaderTag::Oxygen);
        assert!(manager.active_faders().is_empty());

        // Seed a fader directly so we can prove `clear()` empties the store.
        manager.faders.push(ActiveFader {
            color: [1.0, 0.0, 0.0, 1.0],
            blend_mode: FaderBlendMode::Tint,
            initial_intensity: 1.0,
            remaining_ticks: 5,
            total_ticks: 5,
            tag: FaderTag::Damage,
        });
        assert_eq!(manager.active_faders().len(), 1);

        // `remove_by_tag()` drops the matching fader.
        manager.remove_by_tag(FaderTag::Damage);
        assert!(
            manager.active_faders().is_empty(),
            "remove_by_tag drops the matching fader"
        );

        // `clear()` empties any remaining faders.
        manager.faders.push(ActiveFader {
            color: [0.0, 0.0, 1.0, 1.0],
            blend_mode: FaderBlendMode::SoftTint,
            initial_intensity: 0.5,
            remaining_ticks: 3,
            total_ticks: 3,
            tag: FaderTag::Oxygen,
        });
        manager.clear();
        assert!(
            manager.active_faders().is_empty(),
            "clear empties the store"
        );
    }

    /// Box 1.4: `tick()` decays intensity. A fader with `total_ticks = 10` and
    /// `initial_intensity = 1.0`, ticked 5 times, has `remaining_ticks = 5` and a
    /// current intensity of `1.0 * (5 / 10) = 0.5`. (Mirrors box 1.6.)
    #[test]
    fn test_tick_decays_intensity_to_half() {
        let mut manager = FaderManager::new();
        manager.trigger(ActiveFader {
            color: [1.0, 0.0, 0.0, 1.0],
            blend_mode: FaderBlendMode::Tint,
            initial_intensity: 1.0,
            remaining_ticks: 10,
            total_ticks: 10,
            tag: FaderTag::Damage,
        });

        for _ in 0..5 {
            manager.tick();
        }

        let faders = manager.active_faders();
        assert_eq!(faders.len(), 1, "fader is still active after 5 of 10 ticks");
        assert_eq!(faders[0].remaining_ticks, 5);
        let intensity = faders[0].current_intensity();
        assert!(
            (intensity - 0.5).abs() < 1e-6,
            "intensity should be ~0.5 after 5/10 ticks, got {intensity}"
        );
    }

    /// Box 1.4: a fader with `total_ticks = 3`, ticked 4 times, expires and is
    /// removed from `active_faders()`. (Mirrors box 1.7.)
    #[test]
    fn test_tick_removes_expired_fader() {
        let mut manager = FaderManager::new();
        manager.trigger(ActiveFader {
            color: [0.0, 0.0, 1.0, 1.0],
            blend_mode: FaderBlendMode::SoftTint,
            initial_intensity: 0.8,
            remaining_ticks: 3,
            total_ticks: 3,
            tag: FaderTag::Oxygen,
        });

        for _ in 0..4 {
            manager.tick();
        }

        assert!(
            manager.active_faders().is_empty(),
            "a fader of duration 3 is removed after 4 ticks"
        );
    }

    /// Box 1.5: `trigger()` dedups by tag. Triggering a tagged fader (Oxygen)
    /// twice replaces the older with the newer, leaving exactly one Oxygen fader.
    /// Untagged faders (`FaderTag::None`) are never deduped: two None triggers
    /// produce two faders.
    #[test]
    fn test_trigger_dedups_tagged_keeps_untagged() {
        let mut manager = FaderManager::new();

        // First Oxygen fader.
        manager.trigger(ActiveFader {
            color: [0.5, 0.5, 0.6, 1.0],
            blend_mode: FaderBlendMode::SoftTint,
            initial_intensity: 0.3,
            remaining_ticks: 5,
            total_ticks: 5,
            tag: FaderTag::Oxygen,
        });
        // Second Oxygen fader with distinct values — should REPLACE the first.
        manager.trigger(ActiveFader {
            color: [0.5, 0.5, 0.6, 1.0],
            blend_mode: FaderBlendMode::SoftTint,
            initial_intensity: 0.9,
            remaining_ticks: 12,
            total_ticks: 12,
            tag: FaderTag::Oxygen,
        });

        let oxygen: Vec<_> = manager
            .active_faders()
            .iter()
            .filter(|f| f.tag == FaderTag::Oxygen)
            .collect();
        assert_eq!(
            oxygen.len(),
            1,
            "a second Oxygen trigger must replace the first, not duplicate"
        );
        assert_eq!(
            oxygen[0].remaining_ticks, 12,
            "the surviving Oxygen fader is the newer one"
        );
        assert_eq!(oxygen[0].initial_intensity, 0.9);

        // Untagged faders are never deduped.
        manager.trigger(ActiveFader {
            color: [1.0, 0.0, 0.0, 1.0],
            blend_mode: FaderBlendMode::Tint,
            initial_intensity: 1.0,
            remaining_ticks: 4,
            total_ticks: 4,
            tag: FaderTag::None,
        });
        manager.trigger(ActiveFader {
            color: [1.0, 0.0, 0.0, 1.0],
            blend_mode: FaderBlendMode::Tint,
            initial_intensity: 1.0,
            remaining_ticks: 4,
            total_ticks: 4,
            tag: FaderTag::None,
        });
        let untagged = manager
            .active_faders()
            .iter()
            .filter(|f| f.tag == FaderTag::None)
            .count();
        assert_eq!(
            untagged, 2,
            "untagged (None) faders are never deduped: two triggers -> two faders"
        );
    }

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
