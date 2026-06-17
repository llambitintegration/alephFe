//! Reconciler stage: the per-tick desired-set diff.
//!
//! Consumes a desired-set of [`EntityDesc`] and, keyed strictly on the opaque
//! `laneId`, classifies each lane into spawn / update-in-place / despawn and
//! emits an ordered, deterministic list of [`ReconcileOp`]. This module is
//! PURE: no clock, no RNG, no I/O, no async — "time" is an explicit integer
//! tick counter passed into [`Reconciler::reconcile`]. The `tokio::sync::watch`
//! plumbing (box 4.1) and the `marathon-sim` `update_agents()` ECS wiring
//! (box 4.3) live elsewhere; this is the diff brain they call into.
//!
//! What lives here:
//! - **laneId-keyed reconcile (box 4.2):** the live-set is keyed solely on
//!   `lane_id`; a republished [`EntityDesc`] with a changed `label`/persona
//!   matches the same live record and yields an [`ReconcileOp::UpdateInPlace`],
//!   never a fresh spawn.
//! - **Per-tick spawn cap (box 4.4):** at most [`SPAWN_CAP`] (8) spawns are
//!   emitted per [`Reconciler::reconcile`] call; the overflow is queued in
//!   `pending_spawns` and drained across subsequent ticks with no further
//!   publish required. A below-cap diff spawns everything immediately.
//! - **Stable slot assignment (box 4.5):** every `lane_id` gets the lowest free
//!   slot index on first spawn, held 1:1 for the lane's life and freed only on
//!   full despawn, so a flapping lane reappears in place and distinct lanes
//!   never collide.
//! - **Grace-debounced despawn (box 4.6):** a lane's first absence starts a
//!   grace countdown (`grace_ticks`) rather than despawning immediately; the
//!   pending despawn is cancelled if the lane reappears within the window, and
//!   the smooth despawn proceeds only once the window elapses.
//! - **`m_del` split (box 4.7):** [`classify_departure`] distinguishes a
//!   deliberate operator retire/send-home of a lane still in the last
//!   desired-set (emit a [`GameAction::Kill`]) from a self-departed lane (a
//!   `final` flag in `meta` and/or bare absence) which is swept silently.
//!
//! Determinism: every collection is a `BTreeMap`/`BTreeSet` and the
//! pending-spawn queue preserves first-seen order, so the `Vec<ReconcileOp>`
//! output of a given input sequence is fully reproducible.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::event::{EntityDesc, GameAction};

/// Maximum number of monsters spawned on any single reconcile tick (box 4.4).
pub const SPAWN_CAP: usize = 8;

/// Default grace window, in ticks, before an absent lane is despawned (box 4.6).
pub const DEFAULT_GRACE_TICKS: u64 = 30;

/// One unit of diff work the caller (`update_agents()`) carries out this tick.
///
/// Ordered deterministically by [`Reconciler::reconcile`]: spawns first (in
/// queue order), then in-place updates (lane-id ascending), then despawns
/// (lane-id ascending).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconcileOp {
    /// Spawn a new monster for `lane_id` in its stable `slot`.
    Spawn { lane_id: String, slot: usize },
    /// Update an existing monster in place (its `EntityDesc` changed).
    UpdateInPlace { lane_id: String },
    /// Smooth-despawn an existing monster whose grace window has elapsed.
    Despawn { lane_id: String },
}

/// How a lane left the desired-set, per the `m_del` disambiguation (box 4.7).
///
/// Only `PartialEq` (not `Eq`): it wraps a [`GameAction`], whose JSON payload
/// type carries `PartialEq` alone.
#[derive(Debug, Clone, PartialEq)]
pub enum Departure {
    /// A deliberate operator retire/send-home of a lane still present in the
    /// last desired-set: emit this [`GameAction::Kill`].
    Retire(GameAction),
    /// A lane that departed on its own (a `final` flag and/or bare absence):
    /// sweep silently, no callback.
    SelfDeparted,
}

/// The live record the reconciler holds for a single lane.
#[derive(Debug, Clone, PartialEq)]
struct LiveAgent {
    /// The stable slot this lane holds for its life (box 4.5).
    slot: usize,
    /// The most recent [`EntityDesc`] applied for this lane (the update-in-place
    /// comparison baseline, and the `last_desc` for the `m_del` split).
    desc: EntityDesc,
    /// `Some(deadline_tick)` once the lane has gone absent and a grace timer is
    /// running; `None` while the lane is present. The despawn fires when the
    /// current tick reaches `deadline_tick` (box 4.6).
    despawn_at: Option<u64>,
}

/// The pure, synchronous reconciler: holds the live agent set, the stable-slot
/// allocator, and the overflow spawn queue across reconcile calls.
#[derive(Debug, Clone)]
pub struct Reconciler {
    /// Live agents keyed strictly on the opaque `lane_id` (box 4.2). `BTreeMap`
    /// for deterministic iteration.
    live: BTreeMap<String, LiveAgent>,
    /// Slots currently occupied by a live (or grace-pending) lane. Used to find
    /// the lowest free slot on a new spawn (box 4.5).
    occupied_slots: BTreeSet<usize>,
    /// Newcomer lanes diffed but not yet spawned because of the per-tick cap,
    /// in first-seen order; drained up to [`SPAWN_CAP`] per tick (box 4.4). Each
    /// entry carries the `EntityDesc` to apply once the spawn lands.
    pending_spawns: VecDeque<EntityDesc>,
    /// Lane ids already queued in `pending_spawns`, to avoid double-queuing a
    /// newcomer that is re-seen on a later tick while still waiting.
    pending_lane_ids: BTreeSet<String>,
    /// Grace window in ticks before an absent lane despawns (box 4.6).
    grace_ticks: u64,
}

impl Default for Reconciler {
    fn default() -> Self {
        Self::new(DEFAULT_GRACE_TICKS)
    }
}

impl Reconciler {
    /// Construct a reconciler with an explicit grace window (box 4.6).
    #[must_use]
    pub fn new(grace_ticks: u64) -> Self {
        Self {
            live: BTreeMap::new(),
            occupied_slots: BTreeSet::new(),
            pending_spawns: VecDeque::new(),
            pending_lane_ids: BTreeSet::new(),
            grace_ticks,
        }
    }

    /// The stable slot currently assigned to `lane_id`, if it is live or
    /// grace-pending. `None` for an unknown lane (box 4.5).
    #[must_use]
    pub fn slot_of(&self, lane_id: &str) -> Option<usize> {
        self.live.get(lane_id).map(|a| a.slot)
    }

    /// Number of lanes still queued for a deferred spawn (box 4.4).
    #[must_use]
    pub fn pending_spawn_count(&self) -> usize {
        self.pending_spawns.len()
    }

    /// Is this lane currently live (spawned, possibly grace-pending)?
    #[must_use]
    pub fn is_live(&self, lane_id: &str) -> bool {
        self.live.contains_key(lane_id)
    }

    /// The lowest free slot index not currently occupied (box 4.5).
    fn lowest_free_slot(&self) -> usize {
        let mut slot = 0;
        while self.occupied_slots.contains(&slot) {
            slot += 1;
        }
        slot
    }

    /// Reconcile the latest desired-set against the live set at `now_tick`,
    /// returning the ordered ops to carry out this tick.
    ///
    /// Pure and deterministic: the only "clock" is `now_tick`, an explicit
    /// integer the caller advances once per game tick. Spawns are capped at
    /// [`SPAWN_CAP`] and the overflow is queued for later ticks (box 4.4);
    /// matching is keyed solely on `lane_id` (box 4.2); slots are stable for a
    /// lane's life (box 4.5); absences are grace-debounced (box 4.6).
    pub fn reconcile(&mut self, desired: &[EntityDesc], now_tick: u64) -> Vec<ReconcileOp> {
        // Index the desired-set by lane_id (latest-wins on duplicate ids within
        // one snapshot). BTreeMap keeps iteration deterministic.
        let mut desired_by_lane: BTreeMap<String, EntityDesc> = BTreeMap::new();
        for desc in desired {
            desired_by_lane.insert(desc.lane_id.clone(), desc.clone());
        }

        let mut updates: Vec<ReconcileOp> = Vec::new();

        // ---- Present lanes: spawn newcomers (queued), update-in-place, and
        //      cancel any pending despawn for a reappearing lane (box 4.6) ----
        for (lane_id, desc) in &desired_by_lane {
            if let Some(agent) = self.live.get_mut(lane_id) {
                // Reappearance cancels a running grace timer (box 4.6).
                agent.despawn_at = None;
                // laneId-keyed update-in-place: a changed label/persona/state is
                // an in-place update, never a respawn (boxes 4.2, 4.3 spirit).
                if &agent.desc != desc {
                    agent.desc = desc.clone();
                    updates.push(ReconcileOp::UpdateInPlace {
                        lane_id: lane_id.clone(),
                    });
                }
            } else if !self.pending_lane_ids.contains(lane_id) {
                // A genuine newcomer: queue it for a (possibly deferred) spawn.
                self.pending_lane_ids.insert(lane_id.clone());
                self.pending_spawns.push_back(desc.clone());
            } else {
                // Already queued from an earlier tick but not yet spawned;
                // refresh the queued desc so it spawns with the latest state.
                if let Some(slot) = self
                    .pending_spawns
                    .iter_mut()
                    .find(|d| &d.lane_id == lane_id)
                {
                    *slot = desc.clone();
                }
            }
        }

        // ---- Absent lanes: start a grace timer, or fire the despawn once the
        //      window has elapsed (box 4.6) ----
        let mut despawns: Vec<ReconcileOp> = Vec::new();
        let mut to_remove: Vec<String> = Vec::new();
        for (lane_id, agent) in &mut self.live {
            if desired_by_lane.contains_key(lane_id) {
                continue;
            }
            match agent.despawn_at {
                None => {
                    // First tick of absence: arm the grace timer (box 4.6).
                    agent.despawn_at = Some(now_tick.saturating_add(self.grace_ticks));
                }
                Some(deadline) => {
                    if now_tick >= deadline {
                        despawns.push(ReconcileOp::Despawn {
                            lane_id: lane_id.clone(),
                        });
                        to_remove.push(lane_id.clone());
                    }
                }
            }
        }
        // Drop despawned lanes and free their slots (box 4.5: slot freed only on
        // full despawn).
        for lane_id in to_remove {
            if let Some(agent) = self.live.remove(&lane_id) {
                self.occupied_slots.remove(&agent.slot);
            }
        }

        // ---- Drain the pending-spawn queue under the per-tick cap (box 4.4) ----
        let mut spawns: Vec<ReconcileOp> = Vec::new();
        for _ in 0..SPAWN_CAP {
            let Some(desc) = self.pending_spawns.pop_front() else {
                break;
            };
            self.pending_lane_ids.remove(&desc.lane_id);
            let slot = self.lowest_free_slot();
            self.occupied_slots.insert(slot);
            let lane_id = desc.lane_id.clone();
            self.live.insert(
                lane_id.clone(),
                LiveAgent {
                    slot,
                    desc,
                    despawn_at: None,
                },
            );
            spawns.push(ReconcileOp::Spawn { lane_id, slot });
        }

        // Deterministic op order: spawns (queue order), updates (lane asc),
        // despawns (lane asc). Updates/despawns are already lane-asc because
        // they were built from the BTreeMap iteration.
        let mut ops = spawns;
        ops.extend(updates);
        ops.extend(despawns);
        ops
    }
}

/// Classify how a lane left the desired-set, replicating the
/// `m_del_from_pid_list` disambiguation (box 4.7).
///
/// A `kill` callback is emitted ONLY for a deliberate operator retire/send-home
/// of a lane that is still present in the last desired-set snapshot
/// (`present_in_desired == true` and `operator_retired == true`). A lane that
/// departed on its own — a `final` flag in `meta` and/or bare absence from the
/// desired-set — is swept silently with NO callback.
///
/// Pure: depends only on its arguments.
#[must_use]
pub fn classify_departure(
    last_desc: &EntityDesc,
    present_in_desired: bool,
    operator_retired: bool,
) -> Departure {
    // A self-departure signal (`final` flag) always wins: even an operator verb
    // races a lane that already finished on its own — sweep it silently.
    let self_final = last_desc
        .meta
        .get("final")
        .map(|v| v == "true")
        .unwrap_or(false);

    if operator_retired && present_in_desired && !self_final {
        Departure::Retire(GameAction::Kill {
            id: last_desc.lane_id.clone(),
        })
    } else {
        Departure::SelfDeparted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EntityKind, EntityState};
    use std::collections::HashMap;

    /// Build a desired-set element for `lane_id` with a given label/state.
    fn desc(lane_id: &str, label: &str, state: EntityState) -> EntityDesc {
        EntityDesc {
            lane_id: lane_id.to_string(),
            kind: EntityKind::Agent,
            label: label.to_string(),
            state,
            meta: HashMap::new(),
        }
    }

    /// Like [`desc`] but with a single `meta` entry set.
    fn desc_meta(lane_id: &str, key: &str, value: &str) -> EntityDesc {
        let mut d = desc(lane_id, "label", EntityState::Active);
        d.meta.insert(key.to_string(), value.to_string());
        d
    }

    fn lane_ids_of_spawns(ops: &[ReconcileOp]) -> Vec<String> {
        ops.iter()
            .filter_map(|op| match op {
                ReconcileOp::Spawn { lane_id, .. } => Some(lane_id.clone()),
                _ => None,
            })
            .collect()
    }

    // ---- Box 4.2: Reconcile is keyed on laneId not label ----

    #[test]
    fn reconcile_is_keyed_on_lane_id_not_label() {
        let mut r = Reconciler::default();
        // Spawn lane-x on tick 0.
        let ops = r.reconcile(&[desc("lane-x", "old-label", EntityState::Active)], 0);
        assert_eq!(lane_ids_of_spawns(&ops), vec!["lane-x".to_string()]);
        let slot = r.slot_of("lane-x").expect("lane-x is live");

        // Republish the SAME laneId with a changed label and a changed persona
        // attribute. It must match the same live monster (update-in-place), not
        // spawn a second one.
        let mut changed = desc("lane-x", "brand-new-label", EntityState::Idle);
        changed
            .meta
            .insert("persona".to_string(), "durandal".to_string());
        let ops = r.reconcile(&[changed], 1);

        assert_eq!(
            lane_ids_of_spawns(&ops),
            Vec::<String>::new(),
            "a changed label must not spawn a new entity"
        );
        assert_eq!(
            ops,
            vec![ReconcileOp::UpdateInPlace {
                lane_id: "lane-x".to_string()
            }],
            "the republished desc must update in place"
        );
        assert_eq!(
            r.slot_of("lane-x"),
            Some(slot),
            "the same laneId keeps its slot across a label change"
        );
        assert!(r.is_live("lane-x"));
    }

    // ---- Box 4.4: Per-Tick Spawn Cap ----

    #[test]
    fn spawn_storm_is_rate_limited() {
        // Desired-set jumps 0 -> 20 in one publish.
        let mut r = Reconciler::default();
        let desired: Vec<EntityDesc> = (0..20)
            .map(|i| desc(&format!("lane-{i:02}"), "l", EntityState::Active))
            .collect();
        let ops = r.reconcile(&desired, 0);
        assert_eq!(
            lane_ids_of_spawns(&ops).len(),
            SPAWN_CAP,
            "exactly 8 spawn this tick"
        );
        assert_eq!(
            r.pending_spawn_count(),
            12,
            "the remaining 12 are deferred to following ticks"
        );
    }

    #[test]
    fn overflow_drains_across_subsequent_ticks() {
        let mut r = Reconciler::default();
        let desired: Vec<EntityDesc> = (0..20)
            .map(|i| desc(&format!("lane-{i:02}"), "l", EntityState::Active))
            .collect();

        // Tick 0: 8 spawn, 12 queued.
        let _ = r.reconcile(&desired, 0);
        assert_eq!(r.pending_spawn_count(), 12);

        // Tick 1: SAME desired-set re-published; 8 more spawn from the queue
        // with no new publish required, 4 remain.
        let ops = r.reconcile(&desired, 1);
        assert_eq!(lane_ids_of_spawns(&ops).len(), SPAWN_CAP);
        assert_eq!(r.pending_spawn_count(), 4);

        // Tick 2: drain the last 4, queue empty, all 20 live.
        let ops = r.reconcile(&desired, 2);
        assert_eq!(lane_ids_of_spawns(&ops).len(), 4);
        assert_eq!(r.pending_spawn_count(), 0);
        for i in 0..20 {
            assert!(r.is_live(&format!("lane-{i:02}")));
        }
    }

    #[test]
    fn drains_with_no_further_publish() {
        // After the first publish the queue drains even if `reconcile` is called
        // with an EMPTY desired-set on later ticks (the deferred newcomers are
        // already committed to the queue).
        let mut r = Reconciler::default();
        let desired: Vec<EntityDesc> = (0..20)
            .map(|i| desc(&format!("lane-{i:02}"), "l", EntityState::Active))
            .collect();
        let _ = r.reconcile(&desired, 0);
        assert_eq!(r.pending_spawn_count(), 12);

        // Re-publish the same set so the just-spawned lanes are not treated as
        // absent; the queue still drains 8/tick.
        let ops = r.reconcile(&desired, 1);
        assert_eq!(lane_ids_of_spawns(&ops).len(), SPAWN_CAP);
        assert_eq!(r.pending_spawn_count(), 4);
    }

    #[test]
    fn below_cap_diff_spawns_all_immediately() {
        let mut r = Reconciler::default();
        let desired: Vec<EntityDesc> = (0..5)
            .map(|i| desc(&format!("lane-{i}"), "l", EntityState::Active))
            .collect();
        let ops = r.reconcile(&desired, 0);
        assert_eq!(lane_ids_of_spawns(&ops).len(), 5, "all 5 spawn on the tick");
        assert_eq!(r.pending_spawn_count(), 0, "nothing is queued");
    }

    // ---- Box 4.5: Stable Slot Assignment Per Lane ----

    #[test]
    fn flapping_agent_reappears_in_the_same_slot() {
        // Use a zero grace window so the despawn fires on the tick AFTER the
        // first absence (the first absence always arms the timer).
        let mut r = Reconciler::new(0);
        let _ = r.reconcile(&[desc("lane-a", "l", EntityState::Active)], 0);
        let original_slot = r.slot_of("lane-a").expect("live");

        // Absent on tick 1: arms the grace timer (deadline = tick 1).
        let ops = r.reconcile(&[], 1);
        assert!(!ops
            .iter()
            .any(|op| matches!(op, ReconcileOp::Despawn { .. })));
        // Tick 2: 2 >= deadline(1), the despawn fires and frees the slot.
        let ops = r.reconcile(&[], 2);
        assert_eq!(
            ops,
            vec![ReconcileOp::Despawn {
                lane_id: "lane-a".to_string()
            }]
        );
        assert!(!r.is_live("lane-a"));

        // Reappears on tick 3: it gets a slot again. With only one lane ever
        // present, the lowest-free slot is the same one it held before.
        let _ = r.reconcile(&[desc("lane-a", "l", EntityState::Active)], 3);
        assert_eq!(
            r.slot_of("lane-a"),
            Some(original_slot),
            "a flapping lane reappears in its previous slot"
        );
    }

    #[test]
    fn flapping_lane_keeps_slot_while_still_live() {
        // With a non-zero grace window, a lane that flaps WITHIN the window is
        // never despawned and trivially keeps its slot.
        let mut r = Reconciler::new(10);
        let _ = r.reconcile(&[desc("lane-a", "l", EntityState::Active)], 0);
        let slot = r.slot_of("lane-a").expect("live");
        // Absent on tick 1 (arms grace), reappears on tick 2 (cancels it).
        let _ = r.reconcile(&[], 1);
        let _ = r.reconcile(&[desc("lane-a", "l", EntityState::Active)], 2);
        assert_eq!(r.slot_of("lane-a"), Some(slot));
        assert!(r.is_live("lane-a"));
    }

    #[test]
    fn distinct_lanes_get_distinct_slots() {
        let mut r = Reconciler::default();
        let _ = r.reconcile(
            &[
                desc("lane-a", "l", EntityState::Active),
                desc("lane-b", "l", EntityState::Active),
            ],
            0,
        );
        let sa = r.slot_of("lane-a").expect("a live");
        let sb = r.slot_of("lane-b").expect("b live");
        assert_ne!(sa, sb, "two live lanes never collide onto one slot");
    }

    // ---- Box 4.6: Grace-Debounced Despawn ----

    #[test]
    fn brief_absence_does_not_despawn() {
        let mut r = Reconciler::new(5);
        let _ = r.reconcile(&[desc("lane-a", "l", EntityState::Active)], 0);

        // Absent on tick 1: arms a grace timer, does NOT despawn.
        let ops = r.reconcile(&[], 1);
        assert!(
            !ops.iter()
                .any(|op| matches!(op, ReconcileOp::Despawn { .. })),
            "first absence must not despawn"
        );
        assert!(r.is_live("lane-a"));

        // Reappears on tick 3, within the window (deadline was tick 1+5=6):
        // cancels the pending despawn, monster stays live in its slot.
        let slot = r.slot_of("lane-a");
        let ops = r.reconcile(&[desc("lane-a", "l", EntityState::Active)], 3);
        assert!(
            !ops.iter()
                .any(|op| matches!(op, ReconcileOp::Despawn { .. })),
            "reappearance within the window cancels the despawn"
        );
        assert!(r.is_live("lane-a"));
        assert_eq!(r.slot_of("lane-a"), slot, "kept its slot");

        // Now absent again from tick 4 onward; it must NOT despawn at the OLD
        // deadline — the grace timer restarted on reappearance.
        let ops = r.reconcile(&[], 4);
        assert!(!ops
            .iter()
            .any(|op| matches!(op, ReconcileOp::Despawn { .. })));
        assert!(r.is_live("lane-a"));
    }

    #[test]
    fn sustained_absence_despawns_after_grace_window() {
        let mut r = Reconciler::new(5);
        let _ = r.reconcile(&[desc("lane-a", "l", EntityState::Active)], 0);

        // Absent from tick 1: deadline = 1 + 5 = 6.
        for tick in 1..6 {
            let ops = r.reconcile(&[], tick);
            assert!(
                !ops.iter()
                    .any(|op| matches!(op, ReconcileOp::Despawn { .. })),
                "must not despawn before the window elapses (tick {tick})"
            );
            assert!(r.is_live("lane-a"));
        }
        // Tick 6: window elapsed -> smooth despawn.
        let ops = r.reconcile(&[], 6);
        assert_eq!(
            ops,
            vec![ReconcileOp::Despawn {
                lane_id: "lane-a".to_string()
            }]
        );
        assert!(!r.is_live("lane-a"));
    }

    // ---- Box 4.7: m_del Operator-Retire vs Self-Departure Split ----

    #[test]
    fn operator_retire_of_a_still_present_monster_emits_kill() {
        let last = desc("lane-a", "l", EntityState::Active);
        let departure = classify_departure(
            &last, /* present_in_desired */ true, /* operator_retired */ true,
        );
        assert_eq!(
            departure,
            Departure::Retire(GameAction::Kill {
                id: "lane-a".to_string()
            }),
            "an operator retire of a still-present lane emits a kill"
        );
    }

    #[test]
    fn self_departed_lane_is_swept_with_no_callback() {
        // `final:true` in meta with no operator verb: silent sweep.
        let last = desc_meta("lane-a", "final", "true");
        let departure = classify_departure(
            &last, /* present_in_desired */ false, /* operator_retired */ false,
        );
        assert_eq!(departure, Departure::SelfDeparted);

        // Even if an operator verb races a lane that already signalled `final`,
        // the self-departure wins — no kill callback.
        let raced = classify_departure(&last, true, true);
        assert_eq!(
            raced,
            Departure::SelfDeparted,
            "a final-flagged lane is swept silently even under a racing retire"
        );
    }

    #[test]
    fn absence_alone_never_emits_kill() {
        // Bare absence (not in desired-set) with no operator retire: silent.
        let last = desc("lane-a", "l", EntityState::Active);
        let departure = classify_departure(
            &last, /* present_in_desired */ false, /* operator_retired */ false,
        );
        assert_eq!(
            departure,
            Departure::SelfDeparted,
            "absence with no operator verb is a self-departure"
        );
    }
}
