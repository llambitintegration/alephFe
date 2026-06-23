//! Full-screen fader effect system (shared between `marathon-game` and `marathon-web`).
//!
//! Faders are ephemeral screen-space visual overlays triggered by game events
//! (damage, teleport, invincibility, etc.). This module holds the pure-data
//! types for the fader system; the renderers own a `FaderManager` and translate
//! its active faders into post-process draw calls. See
//! `openspec/changes/implement-fullscreen-effects/design.md`.

use marathon_formats::MmlSection;

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

impl FaderBlendMode {
    /// The [`FaderBlendMode`] for a shader mode index (`0..=5`), or `None` if the
    /// index is out of range. The mapping mirrors the enum discriminants:
    /// `0 = Tint`, `1 = Randomize`, `2 = Negate`, `3 = Dodge`, `4 = Burn`,
    /// `5 = SoftTint` (the Blend Mode Specifications table in `design.md`).
    pub fn from_index(index: u32) -> Option<Self> {
        match index {
            0 => Some(FaderBlendMode::Tint),
            1 => Some(FaderBlendMode::Randomize),
            2 => Some(FaderBlendMode::Negate),
            3 => Some(FaderBlendMode::Dodge),
            4 => Some(FaderBlendMode::Burn),
            5 => Some(FaderBlendMode::SoftTint),
            _ => None,
        }
    }
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

/// Per-fader-type default parameters used when triggering a fader.
///
/// A `FaderConfig` carries the canonical defaults for one fader type (the
/// values used unless a trigger overrides them): the overlay `color`, the
/// compositing `blend_mode`, the `duration` in ticks, and the `base_intensity`
/// at trigger time. Box 2.2 collects these into a `FaderConfigTable` keyed by
/// fader-type index (hardcoded Marathon 2 defaults), and box 2.3 lets the MML
/// `faders` section override individual fields. See
/// `openspec/changes/implement-fullscreen-effects/design.md` (Decision 6).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaderConfig {
    /// Default RGBA overlay color for this fader type.
    pub color: [f32; 4],
    /// Default compositing mode applied in the fader fragment shader.
    pub blend_mode: FaderBlendMode,
    /// Default lifetime in ticks (maps to `ActiveFader::total_ticks`).
    pub duration: u16,
    /// Default intensity at trigger time (1.0 = full strength).
    pub base_intensity: f32,
}

/// The number of distinct fader types (one `FaderConfig` slot per type).
///
/// Indices follow the `FaderTag` declaration order: Damage=0, Teleport=1,
/// Invincibility=2, Oxygen=3, Shield=4, Lava=5, Infravision=6. The `None`
/// tag is a dedup sentinel, not a fader type, so it has no table slot.
const FADER_TYPE_COUNT: usize = 7;

/// Lookup table from fader-type index to its default [`FaderConfig`].
///
/// Holds one [`FaderConfig`] per fader type, keyed by the type index (the
/// `FaderTag` discriminant: Damage=0 .. Infravision=6). [`marathon2_defaults`]
/// constructs the table with the hardcoded Marathon 2 defaults (red tint for
/// damage, white randomize static for teleport, soft tints for the sustained
/// glows, etc.). Box 2.3 lets the MML `faders` section override individual
/// entries; lookups go through [`config`] (by tag) or [`config_by_index`].
///
/// [`marathon2_defaults`]: FaderConfigTable::marathon2_defaults
/// [`config`]: FaderConfigTable::config
/// [`config_by_index`]: FaderConfigTable::config_by_index
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaderConfigTable {
    /// Per-fader-type defaults, indexed by the `FaderTag` discriminant.
    configs: [FaderConfig; FADER_TYPE_COUNT],
}

impl FaderConfigTable {
    /// Build the table with the hardcoded Marathon 2 per-fader-type defaults.
    ///
    /// The defaults mirror the original Marathon 2 fader behaviour described in
    /// `openspec/changes/implement-fullscreen-effects/design.md` (Decision 6 and
    /// the sim-event wiring in tasks.md §6):
    ///
    /// | Index | Type          | Blend mode | Color       | Duration | Intensity |
    /// |-------|---------------|------------|-------------|----------|-----------|
    /// | 0     | Damage        | Tint       | red         | 12       | 0.8       |
    /// | 1     | Teleport      | Randomize  | white       | 15       | 1.0       |
    /// | 2     | Invincibility | SoftTint   | gold-green  | 30       | 0.5       |
    /// | 3     | Oxygen        | SoftTint   | blue-gray   | 15       | 0.4       |
    /// | 4     | Shield        | Dodge      | blue-white  | 4        | 0.4       |
    /// | 5     | Lava          | Burn       | orange-red  | 15       | 0.6       |
    /// | 6     | Infravision   | SoftTint   | green       | 30       | 0.4       |
    pub fn marathon2_defaults() -> Self {
        Self {
            configs: [
                // 0: Damage — red tint flash.
                FaderConfig {
                    color: [0.8, 0.0, 0.0, 1.0],
                    blend_mode: FaderBlendMode::Tint,
                    duration: 12,
                    base_intensity: 0.8,
                },
                // 1: Teleport — white randomize static.
                FaderConfig {
                    color: [1.0, 1.0, 1.0, 1.0],
                    blend_mode: FaderBlendMode::Randomize,
                    duration: 15,
                    base_intensity: 1.0,
                },
                // 2: Invincibility — gold-green soft-tint glow.
                FaderConfig {
                    color: [0.8, 0.9, 0.2, 1.0],
                    blend_mode: FaderBlendMode::SoftTint,
                    duration: 30,
                    base_intensity: 0.5,
                },
                // 3: Oxygen — blue-gray soft-tint warning.
                FaderConfig {
                    color: [0.4, 0.5, 0.6, 1.0],
                    blend_mode: FaderBlendMode::SoftTint,
                    duration: 15,
                    base_intensity: 0.4,
                },
                // 4: Shield — blue-white dodge.
                FaderConfig {
                    color: [0.6, 0.8, 1.0, 1.0],
                    blend_mode: FaderBlendMode::Dodge,
                    duration: 4,
                    base_intensity: 0.4,
                },
                // 5: Lava — orange-red burn.
                FaderConfig {
                    color: [0.9, 0.3, 0.1, 1.0],
                    blend_mode: FaderBlendMode::Burn,
                    duration: 15,
                    base_intensity: 0.6,
                },
                // 6: Infravision — green soft-tint.
                FaderConfig {
                    color: [0.2, 0.9, 0.2, 1.0],
                    blend_mode: FaderBlendMode::SoftTint,
                    duration: 30,
                    base_intensity: 0.4,
                },
            ],
        }
    }

    /// The default [`FaderConfig`] for the given fader tag.
    ///
    /// Maps the tag to its type index (`FaderTag::Damage` -> 0, etc.). The
    /// `FaderTag::None` sentinel is not a fader type and has no config; it falls
    /// back to the damage config (index 0) so the accessor is total.
    pub fn config(&self, tag: FaderTag) -> FaderConfig {
        let index = tag as usize;
        // `FaderTag::None` (index 7) is a dedup sentinel, not a fader type.
        self.configs.get(index).copied().unwrap_or(self.configs[0])
    }

    /// The [`FaderConfig`] at the given fader-type index, or `None` if out of
    /// range (valid indices are `0..FADER_TYPE_COUNT`).
    pub fn config_by_index(&self, index: usize) -> Option<FaderConfig> {
        self.configs.get(index).copied()
    }

    /// Override the M2 defaults with a parsed MML `<faders>` section.
    ///
    /// Reads the parsed `faders` [`MmlSection`] produced by `marathon-formats`
    /// (Decision 6 in `openspec/changes/implement-fullscreen-effects/design.md`)
    /// and overrides the matching entries in this table. Each `<fader index="N">`
    /// element maps to the fader-type slot at index `N` (`0 = Damage` ..
    /// `6 = Infravision`, mirroring the [`FaderTag`] discriminant order); a
    /// `<fader>` with a missing or out-of-range `index` is skipped.
    ///
    /// Per-fader overrides are applied only for the attributes that are present,
    /// so a partial element (e.g. only `duration`) leaves the other fields at
    /// their default. Recognized attributes:
    ///
    /// - `red`/`green`/`blue`/`alpha` — RGBA color channels (`f32`, 0.0..=1.0).
    ///   A channel with no attribute keeps the default channel value.
    /// - `blend_mode` — the [`FaderBlendMode`] index (`0..=5`); out-of-range or
    ///   malformed values are ignored.
    /// - `duration` — lifetime in ticks (`u16`).
    /// - `intensity` — base intensity at trigger time (`f32`).
    ///
    /// Attribute values follow AlephOne's lenient parsing conventions (see
    /// [`marathon_formats::mml_interpret`]); a malformed value warns and leaves
    /// the corresponding field unchanged rather than failing.
    pub fn apply_mml_faders(&mut self, section: &MmlSection) {
        use marathon_formats::mml_interpret::{parse_mml_f32, parse_mml_u32};

        for el in &section.elements {
            if el.name != "fader" {
                continue;
            }
            // A `<fader>` without a parseable, in-range index is skipped.
            let index = match el.attributes.get("index").and_then(|s| parse_mml_u32(s)) {
                Some(i) => i as usize,
                None => continue,
            };
            let Some(config) = self.configs.get_mut(index) else {
                continue;
            };

            // Color channels: override only the channels that are present.
            for (chan, key) in ["red", "green", "blue", "alpha"].iter().enumerate() {
                if let Some(v) = el.attributes.get(*key).and_then(|s| parse_mml_f32(s)) {
                    config.color[chan] = v;
                }
            }

            // Blend mode: map the mode index (0..=5) onto a `FaderBlendMode`.
            if let Some(mode) = el
                .attributes
                .get("blend_mode")
                .and_then(|s| parse_mml_u32(s))
                .and_then(FaderBlendMode::from_index)
            {
                config.blend_mode = mode;
            }

            // Duration (ticks) and base intensity.
            if let Some(d) = el.attributes.get("duration").and_then(|s| parse_mml_u32(s)) {
                config.duration = d.min(u16::MAX as u32) as u16;
            }
            if let Some(i) = el
                .attributes
                .get("intensity")
                .and_then(|s| parse_mml_f32(s))
            {
                config.base_intensity = i;
            }
        }
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

    /// Box 2.1: `FaderConfig` holds the per-fader-type default parameters
    /// (color, blend mode, duration, base intensity) used when triggering a
    /// fader of a given type. This test constructs one and asserts its fields
    /// round-trip; box 2.2 (FaderConfigTable) and 2.3 (MML override) build on it.
    #[test]
    fn test_fader_config_fields() {
        let config = FaderConfig {
            color: [1.0, 0.0, 0.0, 1.0],
            blend_mode: FaderBlendMode::Tint,
            duration: 12,
            base_intensity: 0.75,
        };

        assert_eq!(config.color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(config.blend_mode, FaderBlendMode::Tint);
        assert_eq!(config.duration, 12u16);
        assert_eq!(config.base_intensity, 0.75);

        // Copy + Clone + Debug derives are usable.
        let cloned = config.clone();
        let copied = config;
        let _ = format!("{copied:?}");
        assert_eq!(cloned, copied);
    }

    /// Box 2.2: `FaderConfigTable` returns the hardcoded Marathon 2 default
    /// `FaderConfig` for each known fader-type index. Per design.md's per-fader
    /// defaults: index 0 (damage) is a red tint, teleport is a white randomize,
    /// shield is a blue-white dodge, lava is an orange-red burn, and the sustained
    /// effects (invincibility, oxygen, infravision) are soft tints. Box 2.4 adds
    /// the canonical damage/teleport assertion test; box 2.3 (MML override) builds
    /// on this lookup.
    #[test]
    fn test_fader_config_table_marathon2_defaults() {
        let table = FaderConfigTable::marathon2_defaults();

        // Damage (index 0) is a red tint.
        let damage = table.config(FaderTag::Damage);
        assert_eq!(
            damage.blend_mode,
            FaderBlendMode::Tint,
            "damage default is a tint"
        );
        assert!(
            damage.color[0] > 0.5 && damage.color[1] < 0.3 && damage.color[2] < 0.3,
            "damage default color is red-ish, got {:?}",
            damage.color
        );
        // Damage type-index is 0.
        assert_eq!(FaderTag::Damage as usize, 0);
        assert_eq!(
            table.config_by_index(0),
            Some(damage),
            "index 0 maps to the damage config"
        );

        // Teleport is a white randomize (static/interference).
        let teleport = table.config(FaderTag::Teleport);
        assert_eq!(
            teleport.blend_mode,
            FaderBlendMode::Randomize,
            "teleport default is randomize"
        );
        assert!(
            teleport.color[0] > 0.8 && teleport.color[1] > 0.8 && teleport.color[2] > 0.8,
            "teleport default color is white-ish, got {:?}",
            teleport.color
        );
        // Teleport duration is 15 ticks (design.md box 6.2).
        assert_eq!(teleport.duration, 15, "teleport duration is 15 ticks");

        // Shield is a blue-white dodge with intensity 0.4, duration 4 (box 6.5).
        let shield = table.config(FaderTag::Shield);
        assert_eq!(shield.blend_mode, FaderBlendMode::Dodge);
        assert!(
            (shield.base_intensity - 0.4).abs() < 1e-6,
            "shield base intensity is 0.4, got {}",
            shield.base_intensity
        );
        assert_eq!(shield.duration, 4, "shield duration is 4 ticks");

        // Lava is an orange-red burn.
        let lava = table.config(FaderTag::Lava);
        assert_eq!(lava.blend_mode, FaderBlendMode::Burn);
        assert!(
            lava.color[0] > 0.5 && lava.color[2] < 0.3,
            "lava default color is orange-red, got {:?}",
            lava.color
        );

        // Sustained effects are soft tints.
        assert_eq!(
            table.config(FaderTag::Invincibility).blend_mode,
            FaderBlendMode::SoftTint
        );
        assert_eq!(
            table.config(FaderTag::Oxygen).blend_mode,
            FaderBlendMode::SoftTint
        );
        assert_eq!(
            table.config(FaderTag::Infravision).blend_mode,
            FaderBlendMode::SoftTint
        );

        // Out-of-range index returns None.
        assert_eq!(table.config_by_index(99), None);
    }

    /// Box 2.3: the MML `faders` section overrides matching `FaderConfigTable`
    /// entries. We build the M2 defaults, then apply a parsed `<faders>` section
    /// that overrides the damage fader (index 0) — color to blue, blend mode to
    /// Negate, duration and intensity — and asserts the overridden fields change
    /// while a non-overridden entry (teleport, index 1) keeps its default.
    #[test]
    fn test_apply_mml_faders_overrides_matching_entries() {
        use marathon_formats::MmlDocument;

        let mut table = FaderConfigTable::marathon2_defaults();
        let damage_default = table.config(FaderTag::Damage);
        let teleport_default = table.config(FaderTag::Teleport);

        // Sanity: the damage default is a red tint (so the override is observable).
        assert_eq!(damage_default.blend_mode, FaderBlendMode::Tint);
        assert!(damage_default.color[2] < 0.3, "damage default blue is low");

        // Parse a `<faders>` section overriding fader index 0 (damage):
        // color -> blue, blend_mode -> 2 (Negate), duration -> 20, intensity -> 0.3.
        let doc = MmlDocument::from_bytes(
            br#"<marathon><faders><fader index="0" red="0.0" green="0.0" blue="1.0" alpha="1.0" blend_mode="2" duration="20" intensity="0.3"/></faders></marathon>"#,
        )
        .unwrap();
        let section = doc.faders.expect("faders section parsed");

        table.apply_mml_faders(&section);

        // Damage (index 0) reflects every overridden field.
        let damage = table.config(FaderTag::Damage);
        assert_eq!(
            damage.color,
            [0.0, 0.0, 1.0, 1.0],
            "damage color overridden to blue"
        );
        assert_eq!(
            damage.blend_mode,
            FaderBlendMode::Negate,
            "damage blend mode overridden to Negate"
        );
        assert_eq!(damage.duration, 20, "damage duration overridden");
        assert!(
            (damage.base_intensity - 0.3).abs() < 1e-6,
            "damage base intensity overridden, got {}",
            damage.base_intensity
        );

        // Teleport (index 1) was not mentioned: it keeps its default unchanged.
        assert_eq!(
            table.config(FaderTag::Teleport),
            teleport_default,
            "non-overridden entry keeps its default"
        );
    }

    /// Box 2.3: a partial `<fader>` override touches only the named attributes;
    /// fields with no corresponding attribute keep the M2 default. A `<fader>`
    /// element without a parseable `index` is skipped (no panic, no change).
    #[test]
    fn test_apply_mml_faders_partial_and_skips_unindexed() {
        use marathon_formats::MmlDocument;

        let mut table = FaderConfigTable::marathon2_defaults();
        let lava_default = table.config(FaderTag::Lava); // index 5

        // Override only the lava duration; leave color/blend_mode/intensity alone.
        // The index-less <fader> is ignored.
        let doc = MmlDocument::from_bytes(
            br#"<marathon><faders><fader index="5" duration="99"/><fader duration="1"/></faders></marathon>"#,
        )
        .unwrap();
        let section = doc.faders.unwrap();

        table.apply_mml_faders(&section);

        let lava = table.config(FaderTag::Lava);
        assert_eq!(lava.duration, 99, "lava duration overridden");
        assert_eq!(
            lava.color, lava_default.color,
            "lava color untouched by a duration-only override"
        );
        assert_eq!(
            lava.blend_mode, lava_default.blend_mode,
            "lava blend mode untouched"
        );
        assert_eq!(
            lava.base_intensity, lava_default.base_intensity,
            "lava base intensity untouched"
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
