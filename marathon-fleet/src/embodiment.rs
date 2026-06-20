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

// ============================================================================
// Box 5.4: Discrete Event And Status Channels
// ============================================================================
//
// Four INDEPENDENT channels, each a pure mapping that renders without consulting
// any of the others (the spec's "Each of these channels MUST render
// independently of the others"):
//   - `box.advanced` -> a discrete completion BEAT; an `append:true` round for an
//     already-beaten box renders a REPEATED beat, not a new box.
//   - `test` result   -> a ONE-SHOT damage flash (a momentary event, not a state).
//   - `pr` axis        -> a floating-label QUEST status (the quest-label channel).
//   - `hitl.required`  -> a RAISE-A-HAND pose plus a BEACON that doubles as the
//     ack/resurrect surface.

/// A discrete completion beat fired by a `box.advanced` event (box 5.4).
///
/// A *new* box advance plays a single [`CompletionBeat::New`] beat; an
/// `append:true` round for a box that already beat plays a
/// [`CompletionBeat::Repeat`] beat (a repeated emphasis on the same box, NOT a
/// new completion). When no `box.advanced` event is present the channel is
/// [`CompletionBeat::None`] — discrete and momentary, never a sustained state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompletionBeat {
    /// No completion event this tick.
    None,
    /// A fresh box advance: a single new-completion beat.
    New,
    /// An `append:true` round on an already-beaten box: a repeated beat.
    Repeat,
}

/// Map a `box.advanced` signal onto a discrete completion beat (box 5.4).
///
/// `advanced` is whether a `box.advanced` event fired this tick; `append` is its
/// `append:true` round flag. An append round for an already-beaten box renders a
/// [`CompletionBeat::Repeat`] beat rather than a fresh box. The beat is always a
/// momentary one-shot — there is no sustained "completed" state here.
///
/// Pure: depends only on its arguments.
#[must_use]
pub fn completion_beat_for(advanced: bool, append: bool) -> CompletionBeat {
    match (advanced, append) {
        (false, _) => CompletionBeat::None,
        (true, false) => CompletionBeat::New,
        (true, true) => CompletionBeat::Repeat,
    }
}

/// The completion beat for an [`EntityDesc`], reading its `box.advanced` axes.
///
/// Reads `meta["box.advanced"]` (a truthy `"true"`/`"1"`) and `meta["append"]`;
/// a missing event resolves to [`CompletionBeat::None`] with no error.
#[must_use]
pub fn completion_beat_of(desc: &EntityDesc) -> CompletionBeat {
    let advanced = meta_truthy(desc, "box.advanced");
    let append = meta_truthy(desc, "append");
    completion_beat_for(advanced, append)
}

/// A one-shot damage flash fired by a `test` result (box 5.4).
///
/// A `failed` test plays a single [`DamageFlash::Flash`] — a momentary flash, NOT
/// a sustained damaged state. A `passed` test or an absent/unrecognized result is
/// [`DamageFlash::None`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DamageFlash {
    /// No flash this tick (test passed, absent, or unrecognized).
    None,
    /// A single one-shot damage flash (the test failed).
    Flash,
}

/// Map a `test` result string onto a one-shot damage flash (box 5.4).
///
/// `failed` -> a single [`DamageFlash::Flash`]; anything else (incl. `passed`,
/// empty, or unrecognized) -> [`DamageFlash::None`]. The flash is momentary by
/// construction: this channel carries no sustained state.
///
/// Pure: depends only on its argument.
#[must_use]
pub fn damage_flash_for(test: &str) -> DamageFlash {
    match test {
        "failed" => DamageFlash::Flash,
        _ => DamageFlash::None,
    }
}

/// The damage flash for an [`EntityDesc`], reading its `test` axis.
#[must_use]
pub fn damage_flash_of(desc: &EntityDesc) -> DamageFlash {
    let test = desc.meta.get("test").map(String::as_str).unwrap_or("");
    damage_flash_for(test)
}

/// The `pr` axis rendered as a floating-label quest status (box 5.4).
///
/// The `pr` axis is surfaced as a quest-status label floating at the monster; the
/// `merged` value pairs with the allied species (box 5.1) but the quest label is
/// an independent channel. An absent/unrecognized `pr` renders no quest status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuestStatus {
    /// No `pr` axis present — no quest label.
    None,
    /// A PR is open (work in flight).
    Open,
    /// A PR is in review.
    InReview,
    /// A PR is merged (paired with the allied skin).
    Merged,
    /// A PR is closed without merge.
    Closed,
}

/// Map the `pr` axis onto a floating-label quest status (box 5.4).
///
/// Recognizes the canonical `pr` spellings; an absent/unrecognized value renders
/// [`QuestStatus::None`] (no quest label) with no error. This is the quest-label
/// channel; it renders independently of the species/allied-skin channel.
///
/// Pure: depends only on its argument.
#[must_use]
pub fn quest_status_for(pr: &str) -> QuestStatus {
    match pr {
        "open" => QuestStatus::Open,
        "in-review" | "in_review" | "review" => QuestStatus::InReview,
        "merged" => QuestStatus::Merged,
        "closed" => QuestStatus::Closed,
        _ => QuestStatus::None,
    }
}

/// The quest status for an [`EntityDesc`], reading its optional `pr` axis.
#[must_use]
pub fn quest_status_of(desc: &EntityDesc) -> QuestStatus {
    let pr = desc.meta.get("pr").map(String::as_str).unwrap_or("");
    quest_status_for(pr)
}

/// The `hitl.required` channel: a raise-a-hand pose plus an ack/resurrect beacon
/// (box 5.4).
///
/// When a HITL gate is required the monster adopts the raise-a-hand pose AND
/// surfaces a beacon; the SAME beacon is the surface an operator clicks to
/// ack/resurrect, so `beacon.is_some()` iff `raise_hand` (they co-fire). When no
/// gate is required there is no raised hand and no beacon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HitlSignal {
    /// `true` when the monster adopts the raise-a-hand gate pose.
    pub raise_hand: bool,
    /// The beacon, present iff a gate is required; it doubles as the
    /// ack/resurrect interaction surface.
    pub beacon: Option<HitlBeacon>,
}

/// The HITL beacon — surfaced on a required gate, doubling as ack/resurrect
/// surface (box 5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HitlBeacon {
    /// `true`: clicking the beacon acks/resurrects the gated monster.
    pub is_ack_resurrect_surface: bool,
}

impl HitlSignal {
    /// The default signal when no gate is required: no raised hand, no beacon.
    pub const NONE: HitlSignal = HitlSignal {
        raise_hand: false,
        beacon: None,
    };
}

/// Map the `hitl.required` flag onto the raise-a-hand pose + beacon (box 5.4).
///
/// A set gate raises the hand AND surfaces a beacon that is the ack/resurrect
/// surface; an unset gate holds [`HitlSignal::NONE`] (no hand, no beacon).
///
/// Pure: depends only on its argument.
#[must_use]
pub fn hitl_signal_for(required: bool) -> HitlSignal {
    if required {
        HitlSignal {
            raise_hand: true,
            beacon: Some(HitlBeacon {
                is_ack_resurrect_surface: true,
            }),
        }
    } else {
        HitlSignal::NONE
    }
}

/// The HITL signal for an [`EntityDesc`], reading its `hitl.required` axis.
#[must_use]
pub fn hitl_signal_of(desc: &EntityDesc) -> HitlSignal {
    hitl_signal_for(meta_truthy(desc, "hitl.required"))
}

/// Read a `meta` axis as a boolean flag (`"true"`/`"1"` are truthy).
///
/// A missing key or any other value is falsey, with no error.
#[must_use]
fn meta_truthy(desc: &EntityDesc, key: &str) -> bool {
    matches!(
        desc.meta.get(key).map(String::as_str),
        Some("true" | "1" | "yes")
    )
}

// ============================================================================
// Box 5.6: Floating-label / annotation overlay
// ============================================================================

/// The floating-label / annotation overlay drawn at the monster (box 5.6).
///
/// A small, render-facing annotation carrying the agent's opaque identity
/// (`laneId`) and its current human-facing task (`label`). The `id` is the
/// stable, opaque `laneId` (never a mutable label); `task` is the human-facing
/// `label` and may change tick-to-tick without changing identity. An empty task
/// renders the id alone (no error).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelOverlay {
    /// The opaque, stable agent identity (the `laneId`).
    pub id: String,
    /// The human-facing current task (the `label`); may be empty.
    pub task: String,
}

/// Render the floating-label overlay for an [`EntityDesc`] (box 5.6).
///
/// The overlay carries the opaque `laneId` as `id` and the human-facing `label`
/// as `task`. It is keyed on the stable `laneId`, so a relabel changes only the
/// `task` text, never the `id`.
///
/// Pure: depends only on `desc`.
#[must_use]
pub fn label_overlay_for(desc: &EntityDesc) -> LabelOverlay {
    LabelOverlay {
        id: desc.lane_id.clone(),
        task: desc.label.clone(),
    }
}

// ============================================================================
// Box 5.7: Monster Is A Faithful Debugger View
// ============================================================================

/// The complete debugger-view body composed from the per-channel mappings
/// (box 5.7).
///
/// Bundles the channels an observer reads to infer the agent's state from the
/// body ALONE: the discrete lifecycle [`pose`](BodyView::pose) and the
/// [`facing`](BodyView::facing) orientation are BOTH present so a simultaneous
/// lifecycle + attention shift is legible at once. There is deliberately NO
/// weapon/kill affordance field; any equipment is decorative and indicates the
/// active tool only ([`tool`](BodyView::tool)).
#[derive(Debug, Clone, PartialEq)]
pub struct BodyView {
    /// The discrete lifecycle pose (driven by `work` state).
    pub pose: LifecyclePose,
    /// The orientation/lean (driven by the attention target).
    pub facing: Facing,
    /// Decorative-only equipment indicating the active tool, if any.
    ///
    /// This is the ONLY equipment channel and it is purely decorative: it can
    /// never name or render a kill-weapon affordance.
    pub tool: Option<DecorativeTool>,
}

/// Decorative equipment indicating the agent's active tool only (box 5.7).
///
/// This is NOT a weapon: it carries no damage/kill semantics. It exists solely to
/// indicate which tool the agent is actively using, by construction unable to
/// represent a combat affordance.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DecorativeTool {
    /// A human-facing tool name (e.g. `"editor"`, `"shell"`); decorative only.
    pub active_tool: String,
}

impl BodyView {
    /// Whether this body renders any combat/kill-weapon affordance.
    ///
    /// Always `false`: the [`BodyView`] type has no weapon channel, so a faithful
    /// debugger view can never render a kill affordance. The decorative
    /// [`tool`](BodyView::tool), when present, indicates the active tool only.
    #[must_use]
    pub const fn renders_combat_affordance(&self) -> bool {
        false
    }
}

/// Compose the faithful debugger-view body from an [`EntityDesc`] + clock
/// (box 5.7).
///
/// Drives BOTH the discrete lifecycle pose (from `work`) and the orientation
/// (from `attention`) from the same `desc`, so a tick that shifts lifecycle state
/// AND attention together updates both channels at once — the body stays legible
/// as a debugger view. The optional `tool` axis is decorative (active-tool
/// indicator) only; there is no combat affordance anywhere in the result.
///
/// Pure: depends only on its arguments.
#[must_use]
pub fn body_view_of(desc: &EntityDesc, anim_clock: u64) -> BodyView {
    BodyView {
        pose: pose_of(desc, anim_clock),
        facing: facing_of(desc),
        tool: desc.meta.get("tool").map(|t| DecorativeTool {
            active_tool: t.clone(),
        }),
    }
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

    // ---- Box 5.4: Discrete Event And Status Channels ----

    // Scenario: A failed test produces a one-shot damage flash.
    #[test]
    fn failed_test_produces_a_one_shot_damage_flash() {
        assert_eq!(
            damage_flash_for("failed"),
            DamageFlash::Flash,
            "a failed test plays a single damage flash"
        );
        // A passed/absent test plays no flash (the flash is momentary, not a state).
        assert_eq!(damage_flash_for("passed"), DamageFlash::None);
        assert_eq!(damage_flash_for(""), DamageFlash::None);

        let failed = desc_meta("lane-a", "test", "failed");
        assert_eq!(damage_flash_of(&failed), DamageFlash::Flash);
        let passed = desc_meta("lane-a", "test", "passed");
        assert_eq!(damage_flash_of(&passed), DamageFlash::None);
    }

    // Scenario: A required HITL gate raises a hand and a beacon.
    #[test]
    fn required_hitl_gate_raises_a_hand_and_a_beacon() {
        let sig = hitl_signal_for(true);
        assert!(
            sig.raise_hand,
            "a required gate adopts the raise-a-hand pose"
        );
        let beacon = sig.beacon.expect("a required gate surfaces a beacon");
        assert!(
            beacon.is_ack_resurrect_surface,
            "the beacon doubles as the ack/resurrect surface"
        );

        // No gate -> no hand, no beacon.
        let none = hitl_signal_for(false);
        assert_eq!(none, HitlSignal::NONE);
        assert!(!none.raise_hand);
        assert!(none.beacon.is_none());

        // Via the EntityDesc path.
        let gated = desc_meta("lane-a", "hitl.required", "true");
        assert!(hitl_signal_of(&gated).raise_hand);
        let ungated = desc("lane-a");
        assert_eq!(hitl_signal_of(&ungated), HitlSignal::NONE);
    }

    // Scenario: An append round repeats the completion beat.
    #[test]
    fn append_round_repeats_the_completion_beat() {
        // A fresh box advance is a NEW beat.
        assert_eq!(completion_beat_for(true, false), CompletionBeat::New);
        // An append:true round on an already-beaten box is a REPEATED beat,
        // distinct from a new box.
        assert_eq!(completion_beat_for(true, true), CompletionBeat::Repeat);
        assert_ne!(
            completion_beat_for(true, false),
            completion_beat_for(true, true),
            "an append round is a repeat, not a new box"
        );
        // No event -> no beat (discrete/momentary).
        assert_eq!(completion_beat_for(false, false), CompletionBeat::None);

        // Via the EntityDesc path.
        let mut appended = desc_meta("lane-a", "box.advanced", "true");
        appended
            .meta
            .insert("append".to_string(), "true".to_string());
        assert_eq!(completion_beat_of(&appended), CompletionBeat::Repeat);
        let fresh = desc_meta("lane-a", "box.advanced", "1");
        assert_eq!(completion_beat_of(&fresh), CompletionBeat::New);
        let quiet = desc("lane-a");
        assert_eq!(completion_beat_of(&quiet), CompletionBeat::None);
    }

    #[test]
    fn pr_axis_renders_a_floating_label_quest_status() {
        assert_eq!(quest_status_for("open"), QuestStatus::Open);
        assert_eq!(quest_status_for("in-review"), QuestStatus::InReview);
        assert_eq!(quest_status_for("merged"), QuestStatus::Merged);
        assert_eq!(quest_status_for("closed"), QuestStatus::Closed);
        assert_eq!(quest_status_for(""), QuestStatus::None);
        assert_eq!(quest_status_for("garbage"), QuestStatus::None);

        let merged = desc_meta("lane-a", "pr", "merged");
        assert_eq!(quest_status_of(&merged), QuestStatus::Merged);
        let none = desc("lane-a");
        assert_eq!(quest_status_of(&none), QuestStatus::None);
    }

    #[test]
    fn discrete_event_status_channels_render_independently() {
        // One desc carries ALL four event/status axes; each channel resolves its
        // own value with no cross-talk.
        let mut d = desc_meta("lane-a", "test", "failed");
        d.meta.insert("pr".to_string(), "open".to_string());
        d.meta
            .insert("hitl.required".to_string(), "true".to_string());
        d.meta
            .insert("box.advanced".to_string(), "true".to_string());

        assert_eq!(damage_flash_of(&d), DamageFlash::Flash);
        assert_eq!(quest_status_of(&d), QuestStatus::Open);
        assert!(hitl_signal_of(&d).raise_hand);
        assert_eq!(completion_beat_of(&d), CompletionBeat::New);

        // Flipping one channel (the test) does not disturb the others.
        let mut d2 = d.clone();
        d2.meta.insert("test".to_string(), "passed".to_string());
        assert_eq!(
            damage_flash_of(&d2),
            DamageFlash::None,
            "test channel changed"
        );
        assert_eq!(quest_status_of(&d2), QuestStatus::Open, "pr unaffected");
        assert!(hitl_signal_of(&d2).raise_hand, "hitl unaffected");
        assert_eq!(
            completion_beat_of(&d2),
            CompletionBeat::New,
            "completion beat unaffected"
        );
    }

    // ---- Box 5.6: Floating-label / annotation overlay ----

    #[test]
    fn label_overlay_carries_opaque_id_and_human_task() {
        let mut d = desc("lane-xyz");
        d.label = "build the platform mechanics".to_string();
        let overlay = label_overlay_for(&d);
        assert_eq!(overlay.id, "lane-xyz", "id is the opaque laneId");
        assert_eq!(overlay.task, "build the platform mechanics");
    }

    #[test]
    fn label_overlay_id_is_stable_across_relabel() {
        // A relabel changes only the task text, never the opaque id.
        let mut t0 = desc("lane-xyz");
        t0.label = "old task".to_string();
        let mut t1 = desc("lane-xyz");
        t1.label = "new task".to_string();
        let o0 = label_overlay_for(&t0);
        let o1 = label_overlay_for(&t1);
        assert_eq!(o0.id, o1.id, "the id is stable across a relabel");
        assert_ne!(o0.task, o1.task, "only the task text changes");
    }

    #[test]
    fn label_overlay_empty_task_renders_id_alone_no_error() {
        let mut d = desc("lane-xyz");
        d.label = String::new();
        let overlay = label_overlay_for(&d);
        assert_eq!(overlay.id, "lane-xyz");
        assert!(overlay.task.is_empty());
    }

    // ---- Box 5.7: Monster Is A Faithful Debugger View ----

    // Scenario: Lifecycle and attention are both legible from the body.
    #[test]
    fn simultaneous_lifecycle_and_attention_shift_drives_both_channels() {
        // Tick 0: working, attention at 0.0.
        let mut t0 = desc_meta("lane-a", "work", "working");
        t0.meta.insert("attention".to_string(), "0.0".to_string());
        let v0 = body_view_of(&t0, 0);
        assert_eq!(v0.pose, LifecyclePose::Working);

        // Tick 1: SIMULTANEOUSLY shift lifecycle (-> blocked) AND attention (-> 0.5).
        let mut t1 = desc_meta("lane-a", "work", "blocked");
        t1.meta.insert("attention".to_string(), "0.5".to_string());
        let v1 = body_view_of(&t1, 1);

        // BOTH channels updated on the same tick.
        assert_eq!(
            v1.pose,
            LifecyclePose::Blocked,
            "the discrete pose changed for the new lifecycle state"
        );
        assert_ne!(v0.pose, v1.pose, "pose changed");
        assert!(v1.facing.from_attention, "facing is attention-driven");
        assert!(
            (v1.facing.yaw - 0.5).abs() < 1e-6,
            "the body reoriented toward the new attention target"
        );
        assert_ne!(v0.facing.yaw, v1.facing.yaw, "orientation changed");
    }

    // Scenario: No combat affordance is rendered.
    #[test]
    fn no_combat_affordance_is_ever_rendered() {
        // Across a spread of states, the body never renders a kill-weapon.
        for work in ["spawning", "working", "idle", "blocked", "finished", "???"] {
            let d = desc_meta("lane-a", "work", work);
            let view = body_view_of(&d, 0);
            assert!(
                !view.renders_combat_affordance(),
                "no kill-weapon affordance is rendered in any state"
            );
        }
    }

    #[test]
    fn decorative_tool_indicates_active_tool_only() {
        // The only equipment channel is the decorative active-tool indicator.
        let d = desc_meta("lane-a", "tool", "editor");
        let view = body_view_of(&d, 0);
        let tool = view
            .tool
            .as_ref()
            .expect("an active tool is surfaced decoratively");
        assert_eq!(tool.active_tool, "editor");
        // It is still not a combat affordance.
        assert!(!view.renders_combat_affordance());

        // No tool axis -> no equipment shown, still no combat affordance.
        let bare = desc("lane-a");
        let bare_view = body_view_of(&bare, 0);
        assert!(bare_view.tool.is_none());
        assert!(!bare_view.renders_combat_affordance());
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
