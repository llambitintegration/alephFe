//! Embodiment stage: the PURE mapping from an agent's reconciled signals onto
//! the render-facing body channels of its in-world Marathon monster.
//!
//! Each function here is a total, deterministic, side-effect-free map from an
//! [`EntityDesc`] (and a small set of explicit hint inputs) onto one embodiment
//! channel. There is no clock, no RNG, no I/O, no async, and no `marathon-sim`
//! dependency: an "animation clock" is an explicit integer tick the caller
//! advances, and a "stable hash" is a fixed, deterministic hash of a stable
//! identifier so the same agent reads as the same creature across renders.
//!
//! What lives here (the four box-5.x channels implemented so far):
//! - **Persona-driven stable species (box 5.1):** [`species_for`] hashes a
//!   *stable* identifier (`laneId` / `session_id`) so the skin never flaps; tier
//!   is encoded by [`SpeciesColor`] (color = rank); a `merged` `pr` axis adopts
//!   an allied skin reserved for green/healthy states.
//! - **Discrete lifecycle pose (box 5.2):** [`pose_for`] maps the `work` state
//!   onto a discrete [`LifecyclePose`] driven by work-state + animation clock
//!   *only* — never inferred from the glow overlay; an unchanged work state
//!   holds the pose (only the clock advances).
//! - **Attention-driven orientation (box 5.3):** [`facing_for`] maps an optional
//!   attention hint onto a [`Facing`]; an absent hint holds a stable default
//!   facing with no error.
//! - **Glow graceful degradation (box 5.8):** [`glow_for`] ships
//!   [`Glow::Dark`] when no `progress` value is resolved (every other channel
//!   renders unaffected) and renders all five [`ProgressPhase`] values
//!   *distinctly* when a value is present.
//!
//! Determinism: every function depends only on its arguments; the species hash
//! uses a fixed (FNV-1a) algorithm so it is reproducible across processes and
//! never depends on `std`'s randomized `HashMap` seed.

use crate::event::{EntityDesc, EntityKind};

/// Marathon color = rank: the tier a species' skin is tinted to encode (box 5.1).
///
/// Marathon recolors a single creature model to denote rank/strength; we reuse
/// that convention so an observer reads tier from color, independent of species.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpeciesColor {
    /// Lowest tier.
    Tan,
    /// Mid-low tier.
    Gold,
    /// Mid-high tier.
    Red,
    /// Highest hostile tier.
    Purple,
    /// The allied/healthy tint, reserved for green/merged/successful states.
    Green,
}

/// A resolved species/skin: the coarse [`EntityKind`] taxonomy plus the Marathon
/// color=rank tier and an `allied` flag (box 5.1).
///
/// `allied == true` is reserved for green/healthy/merged states and always pairs
/// with [`SpeciesColor::Green`]; a hostile body never carries the allied flag.
///
/// Only `Clone`/`PartialEq` (not `Copy`/`Eq`/`Hash`): it embeds [`EntityKind`],
/// whose shared taxonomy carries only those derives.
#[derive(Debug, Clone, PartialEq)]
pub struct Species {
    /// The coarse species/category this body embodies.
    pub kind: EntityKind,
    /// The color=rank tier tint.
    pub color: SpeciesColor,
    /// `true` for the allied skin reserved for green/healthy/merged states.
    pub allied: bool,
}

/// The discrete lifecycle pose driven by the `work` axis only (box 5.2).
///
/// One pose per `work` value plus an [`LifecyclePose::Unknown`] fall-through for
/// an unrecognized work string. The pose is a function of work-state alone; the
/// animation clock advances *within* a pose but never selects it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LifecyclePose {
    /// `spawning`: materializing into the world.
    Spawning,
    /// `working`: the active work posture.
    Working,
    /// `idle`: live but at rest.
    Idle,
    /// `blocked`: stalled/awaiting, a visibly distinct stuck posture.
    Blocked,
    /// `finished`: work complete.
    Finished,
    /// Unrecognized `work` value — a neutral default posture, no error.
    Unknown,
}

/// The body's orientation/lean, carried by motion (face-free) (box 5.3).
///
/// `yaw` is in turns ∈ [0.0, 1.0): 0.0 faces the stable default direction; a
/// non-zero `yaw` leans the body toward the attention target. The
/// [`Facing::DEFAULT`] is the stable facing held when no attention hint is
/// present.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Facing {
    /// Orientation in turns, ∈ [0.0, 1.0).
    pub yaw: f32,
    /// `true` when this facing was driven by an attention hint; `false` when it
    /// is the held stable default.
    pub from_attention: bool,
}

impl Facing {
    /// The stable default facing held when no attention hint is present.
    pub const DEFAULT: Facing = Facing {
        yaw: 0.0,
        from_attention: false,
    };
}

/// The producer's per-lane `progress` classifier phase (box 5.8, producer N1).
///
/// Until the N1 classifier lands, no `progress` value resolves and the glow
/// ships [`Glow::Dark`]; when present, all five phases render distinctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProgressPhase {
    /// Healthy forward progress.
    Productive,
    /// Stalled but not regressing.
    Plateau,
    /// Suspected backward motion.
    RegressionSuspected,
    /// Churn without signal.
    NoiseAmplification,
    /// Out of useful work.
    Exhausted,
}

impl ProgressPhase {
    /// Parse the on-wire `progressPhase` string into a phase, if recognized.
    ///
    /// Accepts the five canonical kebab-case spellings from the spec; an
    /// unrecognized value resolves to `None` (treated as no progress → dark).
    #[must_use]
    pub fn parse(s: &str) -> Option<ProgressPhase> {
        match s {
            "productive" => Some(ProgressPhase::Productive),
            "plateau" => Some(ProgressPhase::Plateau),
            "regression-suspected" => Some(ProgressPhase::RegressionSuspected),
            "noise-amplification" => Some(ProgressPhase::NoiseAmplification),
            "exhausted" => Some(ProgressPhase::Exhausted),
            _ => None,
        }
    }
}

/// The glow/confidence overlay channel (box 5.8).
///
/// [`Glow::Dark`] is the graceful-degradation default shipped whenever no
/// `progress` value is resolved; [`Glow::Phase`] carries a per-[`ProgressPhase`]
/// distinct appearance once the producer classifier lands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Glow {
    /// No resolved `progress`: dark/neutral, the degraded default.
    Dark,
    /// A resolved phase with a distinct visual appearance.
    Phase {
        /// The classified phase this glow renders.
        phase: ProgressPhase,
        /// Distinct emissive saturation ∈ [0.0, 1.0] per phase.
        saturation: f32,
        /// Distinct flicker rate per phase (0 = steady).
        flicker_hz: f32,
    },
}

/// A fixed, deterministic FNV-1a hash of a stable identifier (box 5.1).
///
/// Used so the persona→species map never depends on `std`'s randomized
/// `HashMap` seed: the same `laneId`/`session_id` hashes identically across
/// ticks and across processes, so the skin never flaps.
#[must_use]
fn stable_hash(input: &str) -> u64 {
    // FNV-1a 64-bit.
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// The stable identifier a body hashes its species from (box 5.1).
///
/// Prefers an explicit `session_id` (the producer-bound stable id) and falls
/// back to the opaque `lane_id`; both are stable for the agent's life, so the
/// selected species never flaps between ticks. The `persona` label is *not*
/// hashed directly because it is a mutable human-facing string.
#[must_use]
fn stable_identity(desc: &EntityDesc) -> &str {
    desc.meta
        .get("session_id")
        .map(String::as_str)
        .unwrap_or(&desc.lane_id)
}

/// The ordered hostile tier palette (color = rank, box 5.1).
const HOSTILE_TIERS: [SpeciesColor; 4] = [
    SpeciesColor::Tan,
    SpeciesColor::Gold,
    SpeciesColor::Red,
    SpeciesColor::Purple,
];

/// Map a lane's persona/identity onto a stable Marathon species/skin (box 5.1).
///
/// The species is a *stable* function of a stable identifier (`session_id` /
/// `lane_id`), so the same agent reads as the same creature across every tick
/// and render — the skin never flaps. Marathon's color = rank convention encodes
/// tier via [`SpeciesColor`]. When the `pr` axis is `merged` the body adopts the
/// allied skin reserved for green/healthy/successful states, never a hostile
/// species.
///
/// Pure: depends only on `desc` and the fixed [`stable_hash`].
#[must_use]
pub fn species_for(desc: &EntityDesc) -> Species {
    // A merged PR (or any green/healthy/merged signal) adopts the allied skin,
    // reserved for successful states — never a hostile species.
    let merged = desc.meta.get("pr").map(String::as_str) == Some("merged");
    if merged {
        return Species {
            kind: EntityKind::Agent,
            color: SpeciesColor::Green,
            allied: true,
        };
    }

    // Hostile body: tier is the color=rank, derived from an explicit `tier`/
    // `rank` hint when present, else stable-hashed so it is fixed per agent.
    let hash = stable_hash(stable_identity(desc));
    let tier_index = desc
        .meta
        .get("tier")
        .or_else(|| desc.meta.get("rank"))
        .and_then(|v| v.parse::<usize>().ok())
        .map_or((hash % HOSTILE_TIERS.len() as u64) as usize, |t| {
            t % HOSTILE_TIERS.len()
        });

    Species {
        kind: EntityKind::Monster,
        color: HOSTILE_TIERS[tier_index],
        allied: false,
    }
}

/// Map the lane's `work` state onto a discrete lifecycle pose (box 5.2).
///
/// The pose is selected from the `work` value ALONE — never inferred from the
/// glow overlay or any other continuous channel. The `_anim_clock` argument is
/// the explicit animation tick the caller advances; it advances the animation
/// *within* a pose but never selects it, so republishing the same `work` state
/// returns the same pose (the clock is accepted to document that the caller, not
/// this function, owns the within-pose animation phase).
///
/// Pure: depends only on its arguments.
#[must_use]
pub fn pose_for(work: &str, _anim_clock: u64) -> LifecyclePose {
    match work {
        "spawning" => LifecyclePose::Spawning,
        "working" => LifecyclePose::Working,
        "idle" => LifecyclePose::Idle,
        "blocked" => LifecyclePose::Blocked,
        "finished" => LifecyclePose::Finished,
        _ => LifecyclePose::Unknown,
    }
}

/// The lifecycle pose for an [`EntityDesc`], reading its `work` meta axis.
///
/// Convenience over [`pose_for`]: a missing `work` axis falls through to
/// [`LifecyclePose::Unknown`] with no error.
#[must_use]
pub fn pose_of(desc: &EntityDesc, anim_clock: u64) -> LifecyclePose {
    let work = desc.meta.get("work").map(String::as_str).unwrap_or("");
    pose_for(work, anim_clock)
}

/// Map an optional attention hint onto the body's orientation/lean (box 5.3).
///
/// Orientation is carried by motion (face-free): a present attention hint
/// (a target yaw in turns) leans the body toward that target; an ABSENT hint
/// (`None`) holds the stable [`Facing::DEFAULT`] with no error. The target yaw
/// is normalized into [0.0, 1.0).
///
/// Pure: depends only on its argument.
#[must_use]
pub fn facing_for(attention_yaw: Option<f32>) -> Facing {
    match attention_yaw {
        Some(target) => Facing {
            yaw: target.rem_euclid(1.0),
            from_attention: true,
        },
        None => Facing::DEFAULT,
    }
}

/// The orientation for an [`EntityDesc`], reading its optional `attention` axis.
///
/// Convenience over [`facing_for`]: reads `meta["attention"]` as a target yaw in
/// turns; a missing or unparseable hint holds the stable default facing without
/// error (the attention channel is OPTIONAL).
#[must_use]
pub fn facing_of(desc: &EntityDesc) -> Facing {
    let hint = desc
        .meta
        .get("attention")
        .and_then(|v| v.parse::<f32>().ok());
    facing_for(hint)
}

/// Map an optional resolved progress phase onto the glow channel (box 5.8).
///
/// When `phase` is `None` (the producer N1 classifier has not resolved a value)
/// the glow ships [`Glow::Dark`] — the graceful-degradation default — and every
/// OTHER embodiment channel renders unaffected. When a phase is present, each of
/// the five [`ProgressPhase`] values maps to a DISTINCT `(saturation,
/// flicker_hz)` pair so no two phases collapse to the same appearance.
///
/// Pure: depends only on its argument.
#[must_use]
pub fn glow_for(phase: Option<ProgressPhase>) -> Glow {
    let Some(phase) = phase else {
        return Glow::Dark;
    };
    // Each phase gets a distinct (saturation, flicker) signature; no collisions.
    let (saturation, flicker_hz) = match phase {
        ProgressPhase::Productive => (1.0, 0.0),
        ProgressPhase::Plateau => (0.5, 0.0),
        ProgressPhase::RegressionSuspected => (0.8, 2.0),
        ProgressPhase::NoiseAmplification => (0.9, 6.0),
        ProgressPhase::Exhausted => (0.2, 0.5),
    };
    Glow::Phase {
        phase,
        saturation,
        flicker_hz,
    }
}

/// The glow for an [`EntityDesc`], reading its optional `progressPhase` axis.
///
/// Convenience over [`glow_for`]: a missing or unrecognized `progressPhase`
/// resolves to `None` → [`Glow::Dark`], so the glow degrades gracefully with no
/// error and no effect on the other channels.
#[must_use]
pub fn glow_of(desc: &EntityDesc) -> Glow {
    let phase = desc
        .meta
        .get("progressPhase")
        .and_then(|v| ProgressPhase::parse(v));
    glow_for(phase)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EntityState;
    use std::collections::HashMap;

    /// Build a minimal desired-set element for `lane_id`.
    fn desc(lane_id: &str) -> EntityDesc {
        EntityDesc {
            lane_id: lane_id.to_string(),
            kind: EntityKind::Agent,
            label: "label".to_string(),
            state: EntityState::Active,
            meta: HashMap::new(),
        }
    }

    /// Like [`desc`] but with a single `meta` entry set.
    fn desc_meta(lane_id: &str, key: &str, value: &str) -> EntityDesc {
        let mut d = desc(lane_id);
        d.meta.insert(key.to_string(), value.to_string());
        d
    }

    // ---- Box 5.1: Persona-Driven Stable Species ----

    // Scenario: Same persona renders the same species across ticks.
    #[test]
    fn same_persona_renders_same_species_across_ticks() {
        // Two separate "ticks" build the same lane with the same persona; the
        // species must be byte-identical (the skin never flaps).
        let mut tick0 = desc_meta("lane-x", "persona", "durandal");
        let mut tick1 = desc_meta("lane-x", "persona", "durandal");
        // The label/state may differ tick to tick without changing identity.
        tick0.label = "old".to_string();
        tick1.label = "new".to_string();
        tick1.state = EntityState::Idle;

        assert_eq!(
            species_for(&tick0),
            species_for(&tick1),
            "the same laneId/persona yields the same species across ticks"
        );
    }

    #[test]
    fn species_hash_uses_stable_identifier_not_mutable_persona_label() {
        // The same stable identity with two DIFFERENT mutable persona labels
        // still hashes to the same species — the hash keys off laneId, not the
        // mutable label, so a persona relabel does not flap the skin.
        let a = desc_meta("lane-x", "persona", "alpha");
        let b = desc_meta("lane-x", "persona", "omega");
        assert_eq!(species_for(&a), species_for(&b));

        // Prefers an explicit session_id when present, still stable.
        let mut s0 = desc_meta("lane-x", "session_id", "sess-1");
        let mut s1 = desc_meta("lane-y", "session_id", "sess-1");
        s0.label = "p".to_string();
        s1.label = "q".to_string();
        assert_eq!(
            species_for(&s0),
            species_for(&s1),
            "the same session_id yields the same species regardless of laneId/label"
        );
    }

    #[test]
    fn tier_is_encoded_by_color_rank() {
        // color = rank: an explicit tier hint selects the corresponding tint,
        // and distinct tiers produce distinct colors.
        let t0 = desc_meta("lane-a", "tier", "0");
        let t1 = desc_meta("lane-a", "tier", "1");
        let t2 = desc_meta("lane-a", "tier", "2");
        let t3 = desc_meta("lane-a", "tier", "3");
        assert_eq!(species_for(&t0).color, SpeciesColor::Tan);
        assert_eq!(species_for(&t1).color, SpeciesColor::Gold);
        assert_eq!(species_for(&t2).color, SpeciesColor::Red);
        assert_eq!(species_for(&t3).color, SpeciesColor::Purple);
    }

    // Scenario: A merged PR adopts an allied skin.
    #[test]
    fn merged_pr_adopts_allied_skin() {
        let merged = desc_meta("lane-a", "pr", "merged");
        let sp = species_for(&merged);
        assert!(
            sp.allied,
            "a merged pr axis must adopt the allied/healthy skin"
        );
        assert_eq!(
            sp.color,
            SpeciesColor::Green,
            "allied skin is green/healthy"
        );
        assert_ne!(
            sp.kind,
            EntityKind::Monster,
            "the merged body is not drawn as a hostile species"
        );

        // An open/non-merged pr stays a hostile species.
        let open = desc_meta("lane-a", "pr", "open");
        assert!(!species_for(&open).allied);
    }

    // ---- Box 5.2: Discrete Lifecycle Pose From Work State ----

    // Scenario: Work-state change drives a discrete pose change.
    #[test]
    fn work_state_change_drives_discrete_pose_change() {
        let working = pose_for("working", 0);
        let blocked = pose_for("blocked", 0);
        assert_eq!(working, LifecyclePose::Working);
        assert_eq!(blocked, LifecyclePose::Blocked);
        assert_ne!(
            working, blocked,
            "a working -> blocked transition is a distinct discrete pose"
        );
    }

    #[test]
    fn each_work_state_maps_to_a_distinct_pose() {
        let poses = [
            pose_for("spawning", 0),
            pose_for("working", 0),
            pose_for("idle", 0),
            pose_for("blocked", 0),
            pose_for("finished", 0),
        ];
        // All five lifecycle poses are pairwise distinct.
        for i in 0..poses.len() {
            for j in (i + 1)..poses.len() {
                assert_ne!(poses[i], poses[j], "poses {i} and {j} must differ");
            }
        }
    }

    // Scenario: Unchanged work state holds the pose.
    #[test]
    fn unchanged_work_state_holds_the_pose() {
        // Same work state, advancing animation clock: the pose is identical
        // (only the within-pose animation phase, owned by the caller, advances).
        let p0 = pose_for("working", 0);
        let p1 = pose_for("working", 1);
        let p99 = pose_for("working", 99);
        assert_eq!(p0, p1);
        assert_eq!(p0, p99);
    }

    #[test]
    fn pose_is_not_inferred_from_glow() {
        // The pose depends only on work-state: two descs with the SAME work but
        // wildly different glow inputs yield the same pose.
        let mut dark = desc_meta("lane-a", "work", "working");
        let mut lit = desc_meta("lane-a", "work", "working");
        // glow inputs differ, pose must not.
        dark.meta
            .insert("progressPhase".to_string(), "exhausted".to_string());
        lit.meta
            .insert("progressPhase".to_string(), "productive".to_string());
        assert_eq!(pose_of(&dark, 0), pose_of(&lit, 0));
        assert_eq!(pose_of(&dark, 0), LifecyclePose::Working);
    }

    #[test]
    fn missing_work_axis_is_unknown_pose_no_error() {
        let d = desc("lane-a");
        assert_eq!(pose_of(&d, 0), LifecyclePose::Unknown);
    }

    // ---- Box 5.3: Attention Drives Orientation ----

    // Scenario: Attention target reorients the monster.
    #[test]
    fn attention_target_reorients_the_monster() {
        let facing = facing_for(Some(0.25));
        assert!(
            facing.from_attention,
            "facing is driven by the attention hint"
        );
        assert!(
            (facing.yaw - 0.25).abs() < f32::EPSILON,
            "the body leans toward the attention target"
        );
        // A different target yields a different facing.
        assert_ne!(facing_for(Some(0.25)).yaw, facing_for(Some(0.75)).yaw);
    }

    #[test]
    fn attention_yaw_is_normalized_into_unit_turns() {
        // Out-of-range yaws wrap into [0.0, 1.0).
        let f = facing_for(Some(1.25));
        assert!((f.yaw - 0.25).abs() < 1e-6);
        let neg = facing_for(Some(-0.25));
        assert!((neg.yaw - 0.75).abs() < 1e-6);
    }

    // Scenario: Absent attention hint holds a stable facing.
    #[test]
    fn absent_attention_hint_holds_stable_default_facing() {
        // No hint -> the stable default, no error, not flagged as from-attention.
        let none = facing_for(None);
        assert_eq!(none, Facing::DEFAULT);
        assert!(!none.from_attention);

        // The default is stable across repeated absent ticks.
        assert_eq!(facing_for(None), facing_for(None));

        // And via the EntityDesc convenience path: no `attention` meta -> default.
        let d = desc("lane-a");
        assert_eq!(facing_of(&d), Facing::DEFAULT);
    }

    #[test]
    fn entity_desc_attention_axis_drives_facing() {
        let d = desc_meta("lane-a", "attention", "0.5");
        let f = facing_of(&d);
        assert!(f.from_attention);
        assert!((f.yaw - 0.5).abs() < 1e-6);
    }

    // ---- Box 5.8: Glow Channel Graceful Degradation ----

    // Scenario: Glow ships dark without a progress value.
    #[test]
    fn glow_ships_dark_without_a_progress_value() {
        assert_eq!(glow_for(None), Glow::Dark, "no progress -> dark, no error");

        // Via the EntityDesc path: a desc carrying no progressPhase resolves dark.
        let d = desc("lane-a");
        assert_eq!(glow_of(&d), Glow::Dark);

        // An unrecognized progressPhase string also degrades to dark.
        let bogus = desc_meta("lane-a", "progressPhase", "not-a-phase");
        assert_eq!(glow_of(&bogus), Glow::Dark);
    }

    // Scenario: Other channels render fully while glow is dark.
    #[test]
    fn other_channels_render_fully_while_glow_is_dark() {
        // A desc with no progress but with persona/work/pr/test/attention/lease:
        // every other channel renders correctly, unaffected by the dark glow.
        let mut d = desc_meta("lane-a", "persona", "durandal");
        d.meta.insert("work".to_string(), "blocked".to_string());
        d.meta.insert("pr".to_string(), "open".to_string());
        d.meta.insert("test".to_string(), "failed".to_string());
        d.meta.insert("attention".to_string(), "0.5".to_string());
        // No progressPhase set -> glow dark.
        assert_eq!(glow_of(&d), Glow::Dark);

        // Species renders (hostile, since pr != merged).
        let sp = species_for(&d);
        assert_eq!(sp.kind, EntityKind::Monster);
        assert!(!sp.allied);
        // Pose renders from work-state.
        assert_eq!(pose_of(&d, 0), LifecyclePose::Blocked);
        // Orientation renders from attention.
        let f = facing_of(&d);
        assert!(f.from_attention);
        assert!((f.yaw - 0.5).abs() < 1e-6);
    }

    // Scenario: All five progress phases render distinctly.
    #[test]
    fn all_five_progress_phases_render_distinctly() {
        let phases = [
            ProgressPhase::Productive,
            ProgressPhase::Plateau,
            ProgressPhase::RegressionSuspected,
            ProgressPhase::NoiseAmplification,
            ProgressPhase::Exhausted,
        ];
        let glows: Vec<Glow> = phases.iter().map(|p| glow_for(Some(*p))).collect();

        // None collapse to Dark, and no two collapse to the same appearance.
        for g in &glows {
            assert_ne!(*g, Glow::Dark, "a present phase never renders dark");
        }
        for i in 0..glows.len() {
            for j in (i + 1)..glows.len() {
                assert_ne!(
                    glows[i], glows[j],
                    "phases {i} and {j} must render distinctly (no collapse)"
                );
            }
        }
    }

    #[test]
    fn progress_phase_parses_all_five_canonical_spellings() {
        assert_eq!(
            ProgressPhase::parse("productive"),
            Some(ProgressPhase::Productive)
        );
        assert_eq!(
            ProgressPhase::parse("plateau"),
            Some(ProgressPhase::Plateau)
        );
        assert_eq!(
            ProgressPhase::parse("regression-suspected"),
            Some(ProgressPhase::RegressionSuspected)
        );
        assert_eq!(
            ProgressPhase::parse("noise-amplification"),
            Some(ProgressPhase::NoiseAmplification)
        );
        assert_eq!(
            ProgressPhase::parse("exhausted"),
            Some(ProgressPhase::Exhausted)
        );
        assert_eq!(ProgressPhase::parse("garbage"), None);
    }
}
