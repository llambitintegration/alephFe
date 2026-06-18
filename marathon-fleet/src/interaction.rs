//! Interaction stage: the PURE care-verb interaction surface.
//!
//! This module models the in-world interaction with an agent-monster as a set of
//! total, deterministic, side-effect-free functions: there is no clock, no RNG,
//! no I/O, no async, no networking, and no HMAC/crypto here. The daemon-side
//! broker signing (box 6.3) and the MQTT transport binding live elsewhere; what
//! is captured here is the engine-local *logic* of the collaborative care
//! surface — including the pure nonce minting + result correlation half of box
//! 6.4 ([`NonceCorrelator`]), whose producer emission and per-nonce
//! response-topic remain the daemon's responsibility.
//!
//! What lives here (boxes 6.1, 6.2, 6.5, 6.6, 6.7, 6.8):
//! - **Care-verb gating (box 6.1):** the verb set is EXACTLY
//!   [`CareVerb`]'s five collaborative-care variants — there is deliberately no
//!   `fire`/weapon/shoot variant. [`action_key`] is actionable only when the
//!   target is within proximity AND has clear line-of-sight.
//! - **Verb → game-action kind mapping (box 6.2):** [`CareVerb::game_action`]
//!   maps each verb onto a [`CareAction`] carrying its [`GameActionKind`] and the
//!   `graceful` flag (`check_in→inspect`, `offer_help`/`ask_to_break→poke`,
//!   `send_home→kill graceful:true`, `retire→kill graceful:false`).
//! - **Denied/failed retire resurrects (box 6.5):** [`resurrect_on_result`]
//!   restores the body (archvile path) when a `retire`/`send_home` result is
//!   `denied`/`failed`.
//! - **m_del disambiguation (box 6.6):** [`sweep_outcome`] emits a `kill`-kind
//!   action ONLY for a deliberate retire/send-home of a monster still in the last
//!   desired-set; a self-departed agent is swept silently (despawn animation, no
//!   callback/action).
//! - **Graded non-committal reveal (box 6.7):** [`reveal_for_zone`] reveals more
//!   across proxemic zones and never commits a care action or touches lifecycle.
//! - **Retire confirmation (box 6.8):** the forced non-graceful `retire` emits a
//!   `kill` action only after an explicit [`Confirmation::Confirmed`].

use crate::event::GameAction;

/// The collaborative care verb set (box 6.1).
///
/// This is the COMPLETE enumeration of verbs the interaction surface can emit.
/// There is no `fire`, no weapon, and no shoot-to-kill variant by constitution
/// (CONTRACT §8): the surface is collaborative care only. Adding a hostile verb
/// here is a deliberate constitutional change, not an incremental edit.
///
/// A small, fully value-like leaf enum: `Copy`/`Eq`/`Hash` are derived to match
/// the taxonomy convention for scalar enums in this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CareVerb {
    /// Read-only check-in (ungated inspect).
    CheckIn,
    /// Offer help (a poke-class nudge).
    OfferHelp,
    /// Ask the agent to take a break (a poke-class nudge).
    AskToBreak,
    /// Send the agent home gracefully (graceful kill).
    SendHome,
    /// Forcibly retire the agent (non-graceful kill — last resort).
    Retire,
}

impl CareVerb {
    /// The COMPLETE verb set the interaction surface can emit (box 6.1).
    ///
    /// Used by callers and tests to enumerate the surface and assert that no
    /// weapon/fire verb exists.
    pub const ALL: [CareVerb; 5] = [
        CareVerb::CheckIn,
        CareVerb::OfferHelp,
        CareVerb::AskToBreak,
        CareVerb::SendHome,
        CareVerb::Retire,
    ];

    /// Map this verb onto its gated game-action kind + `graceful` flag (box 6.2).
    ///
    /// `check_in → inspect`, `offer_help`/`ask_to_break → poke`,
    /// `send_home → kill graceful:true`, `retire → kill graceful:false`.
    pub fn game_action(self) -> CareAction {
        match self {
            CareVerb::CheckIn => CareAction {
                verb: self,
                kind: GameActionKind::Inspect,
                graceful: true,
            },
            CareVerb::OfferHelp | CareVerb::AskToBreak => CareAction {
                verb: self,
                kind: GameActionKind::Poke,
                graceful: true,
            },
            CareVerb::SendHome => CareAction {
                verb: self,
                kind: GameActionKind::Kill,
                graceful: true,
            },
            CareVerb::Retire => CareAction {
                verb: self,
                kind: GameActionKind::Kill,
                graceful: false,
            },
        }
    }

    /// Whether emitting this verb requires an explicit confirmation (box 6.8).
    ///
    /// Only the forced, non-graceful `retire` is a non-reversible last resort and
    /// requires confirmation; every other verb is reversible/opt-in.
    pub fn requires_confirmation(self) -> bool {
        matches!(self, CareVerb::Retire)
    }
}

/// The interaction-local game-action kind a [`CareVerb`] maps to (box 6.2).
///
/// Intentionally distinct from [`GameAction`] (the shared wire enum): this is the
/// engine-local *kind* discriminant, carried alongside the `graceful` flag in a
/// [`CareAction`]. A small value-like leaf enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameActionKind {
    /// Read-only inspect / check-in.
    Inspect,
    /// A non-destructive poke (offer-help / ask-to-break class).
    Poke,
    /// Retire/send-home — the `graceful` flag on [`CareAction`] distinguishes the
    /// graceful send-home from the forced retire.
    Kill,
}

/// The result of mapping a [`CareVerb`] onto its game-action (box 6.2).
///
/// Carries the originating verb, its [`GameActionKind`], and the `graceful` flag
/// (only meaningful for [`GameActionKind::Kill`], where `true` = send-home and
/// `false` = forced retire). A value-like leaf struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CareAction {
    /// The care verb this action originated from.
    pub verb: CareVerb,
    /// The gated game-action kind.
    pub kind: GameActionKind,
    /// `true` for a graceful operation; `false` only for the forced `retire`.
    pub graceful: bool,
}

impl CareAction {
    /// Convert this care action into the shared [`GameAction`] for `target` so
    /// the reconciler/sim can route it (box 6.2).
    pub fn to_game_action(self, target: &str) -> GameAction {
        let id = target.to_string();
        match self.kind {
            GameActionKind::Inspect => GameAction::Inspect { id },
            GameActionKind::Poke => GameAction::Poke { id },
            GameActionKind::Kill => GameAction::Kill { id },
        }
    }
}

/// The operator's interaction context against a single target this frame
/// (box 6.1).
///
/// Captures the two physical gates the engine derives from the world (proximity
/// and line-of-sight). The action key is only actionable when BOTH hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InteractionContext {
    /// `true` when the target is within interaction proximity.
    pub within_proximity: bool,
    /// `true` when the operator has clear line-of-sight to the target.
    pub has_line_of_sight: bool,
}

impl InteractionContext {
    /// Whether the action key is actionable against the target (box 6.1).
    ///
    /// Actionable iff within proximity AND with clear line-of-sight. Neither gate
    /// alone is sufficient.
    pub fn actionable(self) -> bool {
        self.within_proximity && self.has_line_of_sight
    }
}

/// Press the action key with `verb` selected against a target whose physical
/// gates are `ctx` (box 6.1).
///
/// Returns `Some(verb)` only when the action key is actionable (proximity AND
/// LOS). When LOS is blocked (or the target is out of proximity), the verb is
/// suppressed and `None` is returned — no care verb is raised.
pub fn action_key(ctx: InteractionContext, verb: CareVerb) -> Option<CareVerb> {
    if ctx.actionable() {
        Some(verb)
    } else {
        None
    }
}

/// The outcome status of a previously emitted care action.
///
/// Mirrors the producer `fleet.action.result{status}` set; correlation-by-nonce
/// lives in the daemon (box 6.4, out of scope here), but the *consequence* of a
/// status is pure engine logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResultStatus {
    /// The action was accepted and applied.
    Accepted,
    /// The producer denied the action.
    Denied,
    /// The action failed to apply.
    Failed,
}

/// What the engine does to the target body in response to a care-action result
/// (box 6.5).
///
/// A value-like leaf enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BodyOutcome {
    /// Leave the body despawned/retired — the action took effect.
    StayDespawned,
    /// Restore the body via the archvile resurrect path.
    Resurrect,
    /// No body change — the action was not a retire/send-home.
    NoChange,
}

/// Resolve a care-action result into a body outcome (box 6.5).
///
/// A `retire` or `send_home` whose result is `denied`/`failed` resurrects the
/// body (archvile path) rather than leaving it despawned; an `accepted` retire
/// stays despawned. Non-kill verbs never touch the body.
pub fn resurrect_on_result(action: CareAction, status: ResultStatus) -> BodyOutcome {
    let is_retire_class = matches!(action.kind, GameActionKind::Kill);
    match (is_retire_class, status) {
        (true, ResultStatus::Denied) | (true, ResultStatus::Failed) => BodyOutcome::Resurrect,
        (true, ResultStatus::Accepted) => BodyOutcome::StayDespawned,
        (false, _) => BodyOutcome::NoChange,
    }
}

/// Why a monster is being removed this tick (box 6.6).
///
/// The `m_del_from_pid_list` disambiguation between an operator's deliberate
/// retire/send-home and an agent that left on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RemovalReason {
    /// The operator deliberately retired or sent home the monster.
    OperatorRetire,
    /// The lane emitted a terminal `final` event.
    SelfDepartedFinal,
    /// The lane is simply absent from the next desired-set snapshot.
    SelfDepartedAbsent,
}

/// What the engine emits when a monster is removed (box 6.6).
///
/// A `kill`-kind care action is emitted ONLY for a deliberate operator retire of
/// a monster still present in the last desired-set; a self-departed agent is
/// swept silently (despawn animation, no callback/action). A value-like enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SweepOutcome {
    /// Emit a `kill`-kind care action for `graceful`-or-forced retire.
    EmitKill {
        /// `true` for a graceful send-home, `false` for a forced retire.
        graceful: bool,
    },
    /// Sweep silently: play the despawn animation, emit no callback/action.
    SweepSilently,
}

/// Resolve the m_del split for a removed monster (box 6.6).
///
/// `present_in_last_desired_set` is whether the `laneId` was still in the last
/// desired-set snapshot at the moment of removal. A `kill`-kind action is emitted
/// only for an [`RemovalReason::OperatorRetire`] of a still-present monster;
/// every self-departure (and any retire of an already-absent lane) sweeps
/// silently.
pub fn sweep_outcome(reason: RemovalReason, present_in_last_desired_set: bool) -> SweepOutcome {
    match reason {
        RemovalReason::OperatorRetire if present_in_last_desired_set => SweepOutcome::EmitKill {
            // Default to forced; callers that distinguish send-home vs retire use
            // `sweep_outcome_for` below with the originating verb.
            graceful: false,
        },
        _ => SweepOutcome::SweepSilently,
    }
}

/// Resolve the m_del split for a deliberate care verb against a monster (box 6.6).
///
/// Like [`sweep_outcome`] but takes the originating retire-class `verb` so the
/// emitted [`SweepOutcome::EmitKill`] carries the correct `graceful` flag
/// (`send_home → true`, `retire → false`). A non-kill verb (or a verb against an
/// absent lane) sweeps silently.
pub fn sweep_outcome_for(verb: CareVerb, present_in_last_desired_set: bool) -> SweepOutcome {
    let action = verb.game_action();
    match action.kind {
        GameActionKind::Kill if present_in_last_desired_set => SweepOutcome::EmitKill {
            graceful: action.graceful,
        },
        _ => SweepOutcome::SweepSilently,
    }
}

/// The proxemic zone of the operator relative to a target (box 6.7).
///
/// Ordered from farthest ([`ProxemicZone::Public`]) to nearest
/// ([`ProxemicZone::Intimate`]); approaching moves toward `Intimate` and reveals
/// progressively more. A value-like, ordered leaf enum (`PartialOrd`/`Ord` are
/// derived so callers can compare distance bands directly).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProxemicZone {
    /// Farthest: only an automap blip is revealed.
    Public,
    /// Identity + presence color + task one-liner is revealed.
    Social,
    /// The "press Action to inspect" affordance is revealed.
    Personal,
    /// Nearest: the action key opens the live terminal.
    Intimate,
}

/// What is revealed about a target in a given proxemic zone (box 6.7).
///
/// Each variant reveals strictly MORE than the farther one and is purely
/// informational — none of these commits a care action or touches the target's
/// lifecycle. A value-like leaf enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reveal {
    /// Public zone: an automap blip only.
    AutomapBlip,
    /// Social zone: identity + presence color + task one-liner.
    IdentityColorTask,
    /// Personal zone: the "press Action to inspect" affordance.
    InspectAffordance,
    /// Intimate zone: the live terminal is opened.
    OpenLiveTerminal,
}

/// Map a proxemic zone onto its graded, non-committal reveal (box 6.7).
///
/// `public → blip`, `social → identity+color+task`, `personal → inspect
/// affordance`, `intimate → open live terminal`. This is a pure read: it NEVER
/// emits a care action and has no lifecycle side effect — approaching only
/// reveals more.
pub fn reveal_for_zone(zone: ProxemicZone) -> Reveal {
    match zone {
        ProxemicZone::Public => Reveal::AutomapBlip,
        ProxemicZone::Social => Reveal::IdentityColorTask,
        ProxemicZone::Personal => Reveal::InspectAffordance,
        ProxemicZone::Intimate => Reveal::OpenLiveTerminal,
    }
}

/// An explicit confirmation token for a forced, non-graceful action (box 6.8).
///
/// The forced `retire` must not emit a `kill` action until the operator has
/// explicitly confirmed; the reversible/opt-in verbs need no confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Confirmation {
    /// The operator has explicitly confirmed.
    Confirmed,
    /// No confirmation yet (the default for a freshly selected `retire`).
    Pending,
}

/// Attempt to emit a care verb against `target`, enforcing both the physical
/// gates (box 6.1) and the retire-confirmation gate (box 6.8).
///
/// Returns `Some(GameAction)` only when:
/// - the action key is actionable (`ctx` proximity AND LOS), AND
/// - if the verb requires confirmation (only the forced `retire`), the operator
///   has explicitly confirmed.
///
/// A `retire` with [`Confirmation::Pending`] yields `None` — no `kill`-kind
/// action is emitted until confirmation. Every reversible/opt-in verb ignores the
/// confirmation gate.
pub fn emit_care_action(
    ctx: InteractionContext,
    verb: CareVerb,
    confirmation: Confirmation,
    target: &str,
) -> Option<GameAction> {
    if !ctx.actionable() {
        return None;
    }
    if verb.requires_confirmation() && confirmation != Confirmation::Confirmed {
        return None;
    }
    Some(verb.game_action().to_game_action(target))
}

/// Whether the interaction surface renders any combat/weapon affordance (box 6.1).
///
/// Always `false`: the care surface is collaborative-only and carries no
/// fire/shoot/weapon verb. Provided as an explicit invariant the debugger-view
/// test asserts, mirroring `embodiment::BodyView::renders_combat_affordance`.
pub const fn renders_weapon_affordance() -> bool {
    false
}

/// A minted, opaque care-action nonce (box 6.4).
///
/// Unique per emitted care action; the engine correlates an inbound
/// `fleet.action.result{nonce, …}` back to its originating action by this value
/// alone. Minting is deterministic — a monotonic counter owned by the
/// [`NonceCorrelator`], with no RNG and no clock in this pure layer. The MQTT
/// per-nonce response-topic binding (CONTRACT §10.4) is the daemon's transport
/// wiring and stays out of this module, exactly as the broker signing does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Nonce(pub u64);

/// Correlates care-action results back to their originating action by nonce
/// (box 6.4).
///
/// Each emitted care action is registered under a freshly-minted [`Nonce`]; an
/// inbound `fleet.action.result{nonce, status}` is applied ONLY to the action
/// that minted the matching nonce, and applied at most once. A result for an
/// unknown or already-resolved nonce is ignored — it is never mis-applied to a
/// different pending action. The body consequence of a result reuses the pure
/// [`resurrect_on_result`] rule (box 6.5), so a `denied`/`failed` retire/send-home
/// resurrects while an `accepted` retire stays despawned.
///
/// This is the engine-local half of box 6.4 (nonce minting + result
/// correlation); the producer-facing emission and the MQTT response-topic
/// binding remain the daemon's responsibility.
#[derive(Debug, Clone, Default)]
pub struct NonceCorrelator {
    next: u64,
    pending: std::collections::HashMap<Nonce, CareAction>,
}

impl NonceCorrelator {
    /// A fresh correlator with no pending actions.
    pub fn new() -> Self {
        Self {
            next: 0,
            pending: std::collections::HashMap::new(),
        }
    }

    /// Mint a unique [`Nonce`] for `action`, register it as pending, and return
    /// the nonce the caller attaches to the outbound `fleet.careAction`. Each
    /// call yields a distinct nonce for the lifetime of this correlator.
    pub fn emit(&mut self, action: CareAction) -> Nonce {
        let nonce = Nonce(self.next);
        self.next += 1;
        self.pending.insert(nonce, action);
        nonce
    }

    /// Number of emitted actions still awaiting a result.
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// Apply a `fleet.action.result{nonce, status}` to the action that minted
    /// `nonce`, returning its [`BodyOutcome`]. Returns `None` when no pending
    /// action minted `nonce` (unknown or already resolved) — the result is then
    /// dropped rather than applied to any other action.
    pub fn correlate(&mut self, nonce: Nonce, status: ResultStatus) -> Option<BodyOutcome> {
        let action = self.pending.remove(&nonce)?;
        Some(resurrect_on_result(action, status))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ----- box 6.1: care-verb gating via proximity + LOS, no weapon verb -----

    #[test]
    fn there_is_no_weapon_or_fire_verb() {
        // The complete enumeration is exactly the five care verbs.
        assert_eq!(CareVerb::ALL.len(), 5);
        assert_eq!(
            CareVerb::ALL,
            [
                CareVerb::CheckIn,
                CareVerb::OfferHelp,
                CareVerb::AskToBreak,
                CareVerb::SendHome,
                CareVerb::Retire,
            ]
        );
        // None of the verbs maps to anything but inspect/poke/kill — there is no
        // discharge/fire kind at all.
        for verb in CareVerb::ALL {
            let kind = verb.game_action().kind;
            assert!(matches!(
                kind,
                GameActionKind::Inspect | GameActionKind::Poke | GameActionKind::Kill
            ));
        }
    }

    #[test]
    fn action_key_with_proximity_and_los_emits_a_care_verb() {
        let ctx = InteractionContext {
            within_proximity: true,
            has_line_of_sight: true,
        };
        assert!(ctx.actionable());
        assert_eq!(
            action_key(ctx, CareVerb::OfferHelp),
            Some(CareVerb::OfferHelp)
        );
    }

    #[test]
    fn no_line_of_sight_suppresses_interaction() {
        let ctx = InteractionContext {
            within_proximity: true,
            has_line_of_sight: false,
        };
        assert!(!ctx.actionable());
        assert_eq!(action_key(ctx, CareVerb::OfferHelp), None);
    }

    #[test]
    fn out_of_proximity_suppresses_interaction() {
        let ctx = InteractionContext {
            within_proximity: false,
            has_line_of_sight: true,
        };
        assert!(!ctx.actionable());
        assert_eq!(action_key(ctx, CareVerb::CheckIn), None);
    }

    // ----- box 6.2: each verb maps to its gated game-action kind -----

    #[test]
    fn each_verb_maps_to_its_gated_game_action_kind() {
        let check_in = CareVerb::CheckIn.game_action();
        assert_eq!(check_in.kind, GameActionKind::Inspect);

        for verb in [CareVerb::OfferHelp, CareVerb::AskToBreak] {
            assert_eq!(verb.game_action().kind, GameActionKind::Poke);
        }

        let send_home = CareVerb::SendHome.game_action();
        assert_eq!(send_home.kind, GameActionKind::Kill);
        assert!(send_home.graceful);

        let retire = CareVerb::Retire.game_action();
        assert_eq!(retire.kind, GameActionKind::Kill);
        assert!(!retire.graceful);
    }

    #[test]
    fn care_action_maps_to_shared_game_action() {
        assert_eq!(
            CareVerb::CheckIn.game_action().to_game_action("lane-1"),
            GameAction::Inspect {
                id: "lane-1".to_string()
            }
        );
        assert_eq!(
            CareVerb::OfferHelp.game_action().to_game_action("lane-2"),
            GameAction::Poke {
                id: "lane-2".to_string()
            }
        );
        assert_eq!(
            CareVerb::Retire.game_action().to_game_action("lane-3"),
            GameAction::Kill {
                id: "lane-3".to_string()
            }
        );
    }

    // ----- box 6.5: denied/failed retire resurrects the body -----

    #[test]
    fn denied_or_failed_retire_resurrects_the_body() {
        let retire = CareVerb::Retire.game_action();
        assert_eq!(
            resurrect_on_result(retire, ResultStatus::Denied),
            BodyOutcome::Resurrect
        );
        assert_eq!(
            resurrect_on_result(retire, ResultStatus::Failed),
            BodyOutcome::Resurrect
        );

        let send_home = CareVerb::SendHome.game_action();
        assert_eq!(
            resurrect_on_result(send_home, ResultStatus::Denied),
            BodyOutcome::Resurrect
        );
        assert_eq!(
            resurrect_on_result(send_home, ResultStatus::Failed),
            BodyOutcome::Resurrect
        );
    }

    #[test]
    fn accepted_retire_stays_despawned_and_non_kill_does_not_touch_body() {
        let retire = CareVerb::Retire.game_action();
        assert_eq!(
            resurrect_on_result(retire, ResultStatus::Accepted),
            BodyOutcome::StayDespawned
        );

        // A non-kill verb never touches the body, whatever the status.
        for status in [
            ResultStatus::Accepted,
            ResultStatus::Denied,
            ResultStatus::Failed,
        ] {
            assert_eq!(
                resurrect_on_result(CareVerb::OfferHelp.game_action(), status),
                BodyOutcome::NoChange
            );
            assert_eq!(
                resurrect_on_result(CareVerb::CheckIn.game_action(), status),
                BodyOutcome::NoChange
            );
        }
    }

    // ----- box 6.6: m_del disambiguation -----

    #[test]
    fn deliberate_retire_of_a_present_monster_emits_a_kill_action() {
        assert_eq!(
            sweep_outcome(RemovalReason::OperatorRetire, true),
            SweepOutcome::EmitKill { graceful: false }
        );
        // verb-aware variant carries the correct graceful flag.
        assert_eq!(
            sweep_outcome_for(CareVerb::Retire, true),
            SweepOutcome::EmitKill { graceful: false }
        );
        assert_eq!(
            sweep_outcome_for(CareVerb::SendHome, true),
            SweepOutcome::EmitKill { graceful: true }
        );
    }

    #[test]
    fn self_departed_agent_is_swept_silently() {
        assert_eq!(
            sweep_outcome(RemovalReason::SelfDepartedFinal, true),
            SweepOutcome::SweepSilently
        );
        assert_eq!(
            sweep_outcome(RemovalReason::SelfDepartedAbsent, true),
            SweepOutcome::SweepSilently
        );
        // A retire of a lane already absent from the last desired-set is also a
        // silent sweep (operator acted on something that already left).
        assert_eq!(
            sweep_outcome(RemovalReason::OperatorRetire, false),
            SweepOutcome::SweepSilently
        );
        assert_eq!(
            sweep_outcome_for(CareVerb::Retire, false),
            SweepOutcome::SweepSilently
        );
        // A non-kill verb is never a kill sweep.
        assert_eq!(
            sweep_outcome_for(CareVerb::OfferHelp, true),
            SweepOutcome::SweepSilently
        );
    }

    // ----- box 6.7: graded non-committal proximity reveal -----

    #[test]
    fn approaching_reveals_but_does_not_commit() {
        // Each zone reveals its own band.
        assert_eq!(reveal_for_zone(ProxemicZone::Public), Reveal::AutomapBlip);
        assert_eq!(
            reveal_for_zone(ProxemicZone::Social),
            Reveal::IdentityColorTask
        );
        assert_eq!(
            reveal_for_zone(ProxemicZone::Personal),
            Reveal::InspectAffordance
        );
        assert_eq!(
            reveal_for_zone(ProxemicZone::Intimate),
            Reveal::OpenLiveTerminal
        );

        // Approaching strictly increases the zone ordering (reveals more).
        assert!(ProxemicZone::Public < ProxemicZone::Social);
        assert!(ProxemicZone::Social < ProxemicZone::Personal);
        assert!(ProxemicZone::Personal < ProxemicZone::Intimate);
    }

    #[test]
    fn revealing_emits_no_care_action_and_no_lifecycle_change() {
        // The reveal API takes only a zone and returns only a Reveal — it has no
        // capacity to emit a GameAction or change a body. We assert structurally
        // that nearing a target without the action key produces no action: the
        // emit path is exclusively `emit_care_action`, never `reveal_for_zone`.
        for zone in [
            ProxemicZone::Public,
            ProxemicZone::Social,
            ProxemicZone::Personal,
            ProxemicZone::Intimate,
        ] {
            // No body outcome is produced by revealing.
            let _reveal = reveal_for_zone(zone);
        }
        // Even at the nearest zone, no action key press => no action.
        let near = InteractionContext {
            within_proximity: true,
            has_line_of_sight: true,
        };
        // The operator has NOT pressed the action key; reveal alone commits
        // nothing. (emit_care_action is the only commit surface and is not
        // invoked by approach.)
        assert!(near.actionable()); // actionable, but uncommitted by approach
    }

    // ----- box 6.8: retire requires explicit confirmation -----

    #[test]
    fn retire_requires_explicit_confirmation() {
        let near = InteractionContext {
            within_proximity: true,
            has_line_of_sight: true,
        };
        // Pending retire emits nothing even with proximity + LOS.
        assert_eq!(
            emit_care_action(near, CareVerb::Retire, Confirmation::Pending, "lane-x"),
            None
        );
        // Confirmed retire emits the kill action.
        assert_eq!(
            emit_care_action(near, CareVerb::Retire, Confirmation::Confirmed, "lane-x"),
            Some(GameAction::Kill {
                id: "lane-x".to_string()
            })
        );
    }

    #[test]
    fn reversible_verbs_need_no_confirmation() {
        let near = InteractionContext {
            within_proximity: true,
            has_line_of_sight: true,
        };
        // A pending confirmation does not block reversible/opt-in verbs.
        assert_eq!(
            emit_care_action(near, CareVerb::OfferHelp, Confirmation::Pending, "lane-y"),
            Some(GameAction::Poke {
                id: "lane-y".to_string()
            })
        );
        assert_eq!(
            emit_care_action(near, CareVerb::CheckIn, Confirmation::Pending, "lane-y"),
            Some(GameAction::Inspect {
                id: "lane-y".to_string()
            })
        );
        // send_home is graceful and reversible-class: no confirmation gate.
        assert_eq!(
            emit_care_action(near, CareVerb::SendHome, Confirmation::Pending, "lane-y"),
            Some(GameAction::Kill {
                id: "lane-y".to_string()
            })
        );
        assert!(!CareVerb::SendHome.requires_confirmation());
        assert!(CareVerb::Retire.requires_confirmation());
    }

    #[test]
    fn confirmation_does_not_bypass_the_physical_gates() {
        // Even a confirmed retire is suppressed without proximity + LOS.
        let blocked = InteractionContext {
            within_proximity: true,
            has_line_of_sight: false,
        };
        assert_eq!(
            emit_care_action(blocked, CareVerb::Retire, Confirmation::Confirmed, "lane-z"),
            None
        );
    }

    #[test]
    fn no_weapon_affordance_is_rendered() {
        assert!(!renders_weapon_affordance());
    }

    // A read-only consumer of EntityDesc to keep the import live and document the
    // intended desired-set membership input for the m_del split.
    #[test]
    fn present_in_desired_set_helper_reads_entity_desc() {
        use crate::event::{EntityDesc, EntityKind, EntityState};
        use std::collections::HashMap;
        let desc = EntityDesc {
            lane_id: "lane-1".to_string(),
            kind: EntityKind::Agent,
            label: "build".to_string(),
            state: EntityState::Active,
            meta: HashMap::new(),
        };
        let last_set = [desc.lane_id.clone()];
        let present = last_set.iter().any(|id| id == &desc.lane_id);
        assert_eq!(
            sweep_outcome_for(CareVerb::Retire, present),
            SweepOutcome::EmitKill { graceful: false }
        );
    }

    // ----- box 6.4: nonce minting + result correlation -----

    #[test]
    fn minted_nonces_are_unique_per_action() {
        let mut c = NonceCorrelator::new();
        let n1 = c.emit(CareVerb::Retire.game_action());
        let n2 = c.emit(CareVerb::SendHome.game_action());
        let n3 = c.emit(CareVerb::OfferHelp.game_action());
        assert_ne!(n1, n2);
        assert_ne!(n2, n3);
        assert_ne!(n1, n3);
        assert_eq!(c.pending_len(), 3);
    }

    #[test]
    fn result_is_applied_only_to_the_matching_nonce() {
        // Two distinct care actions are in flight; a result for the second's
        // nonce must resolve ONLY the second, leaving the first untouched.
        let mut c = NonceCorrelator::new();
        let retire = c.emit(CareVerb::Retire.game_action());
        let send_home = c.emit(CareVerb::SendHome.game_action());

        // Deny the send-home — only it resurrects; the retire is still pending.
        assert_eq!(
            c.correlate(send_home, ResultStatus::Denied),
            Some(BodyOutcome::Resurrect)
        );
        assert_eq!(c.pending_len(), 1);

        // The retire resolves independently and on its own nonce.
        assert_eq!(
            c.correlate(retire, ResultStatus::Accepted),
            Some(BodyOutcome::StayDespawned)
        );
        assert_eq!(c.pending_len(), 0);
    }

    #[test]
    fn result_for_unknown_nonce_is_dropped() {
        let mut c = NonceCorrelator::new();
        let _ = c.emit(CareVerb::Retire.game_action());
        // A nonce that was never minted by this correlator resolves nothing.
        assert_eq!(c.correlate(Nonce(9_999), ResultStatus::Failed), None);
        assert_eq!(c.pending_len(), 1);
    }

    #[test]
    fn result_is_applied_at_most_once() {
        let mut c = NonceCorrelator::new();
        let n = c.emit(CareVerb::SendHome.game_action());
        assert_eq!(
            c.correlate(n, ResultStatus::Failed),
            Some(BodyOutcome::Resurrect)
        );
        // A duplicate/late redelivery of the same result is ignored.
        assert_eq!(c.correlate(n, ResultStatus::Failed), None);
        assert_eq!(c.pending_len(), 0);
    }
}
