//! Projection stage: the pure deterministic event-sourced reducer.
//!
//! Hosts `apply(state, event) -> state` with no clock, no RNG, and no I/O. The
//! same ordered event log MUST fold to a byte-identical [`WorldState`].
//!
//! The fold is per-entity, keyed by the event's `subject`, so interleaved
//! events for distinct subjects never cross-contaminate (box 3.2). [`fold`]
//! applies events strictly in ascending `seq` order (box 3.3) and deduplicates
//! by event `id` so at-least-once redelivery is a no-op (box 3.4). Dedupe is
//! intrinsic to [`apply`] — the set of already-applied ids lives inside the
//! `WorldState` — so a direct re-`apply` of the same id is also a no-op.
//!
//! Snapshot-anchor reconstruction and restart-by-replay recovery (boxes
//! 3.5-3.7) are not yet implemented.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;

use crate::event::EventEnvelope;

/// The folded projection of an event log: one [`EntityProjection`] per
/// `subject`, plus the set of event ids already applied (the dedupe ledger).
///
/// All collections are ordered (`BTreeMap`/`BTreeSet`) so that serialization is
/// byte-stable — folding the same ordered log always yields byte-identical
/// bytes, independent of insertion order.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct WorldState {
    /// Per-entity folded state, keyed by `subject` (deterministic iteration).
    pub entities: BTreeMap<String, EntityProjection>,
    /// Ids of every event already folded — the dedupe ledger. Re-applying an
    /// id already present here is a no-op (box 3.4).
    pub applied_ids: BTreeSet<String>,
}

/// The per-entity folded state. A minimal but non-trivial fold: it counts the
/// events seen, remembers the highest `seq` and last `event_type` applied, and
/// shallow-merges each event's `data` object so the latest field values win.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct EntityProjection {
    /// Number of distinct events folded into this entity.
    pub event_count: u64,
    /// The highest `seq` applied to this entity so far.
    pub last_seq: u64,
    /// The `event_type` of the most recently applied event.
    pub last_event_type: String,
    /// Shallow merge of every applied event's `data` object (last write wins
    /// per key). Object keys are stored in a `BTreeMap` for byte-stability.
    pub data: BTreeMap<String, Value>,
}

impl EntityProjection {
    /// Fold one event into this entity's projection (pure).
    fn ingest(&mut self, event: &EventEnvelope) {
        self.event_count += 1;
        self.last_seq = event.seq;
        self.last_event_type = event.event_type.clone();
        if let Value::Object(map) = &event.data {
            for (k, v) in map {
                self.data.insert(k.clone(), v.clone());
            }
        }
    }
}

/// The pure deterministic reducer: fold one event into the state and return the
/// new state (box 3.1).
///
/// This function has no access to a clock, no source of randomness, and no
/// I/O. Its output is a function of the prior `state` and the applied `event`
/// alone. The fold is keyed on `event.subject` (box 3.2). Applying an event
/// whose `id` is already in `state.applied_ids` is a no-op (box 3.4), so a
/// direct re-`apply` of the same id leaves the state unchanged.
#[must_use]
pub fn apply(mut state: WorldState, event: &EventEnvelope) -> WorldState {
    // Dedupe by id: at-least-once redelivery applies exactly once (box 3.4).
    if state.applied_ids.contains(&event.id) {
        return state;
    }
    state.applied_ids.insert(event.id.clone());
    // Per-entity fold keyed by subject (box 3.2).
    state
        .entities
        .entry(event.subject.clone())
        .or_default()
        .ingest(event);
    state
}

/// Fold a slice of events into a [`WorldState`].
///
/// Applies events strictly in ascending `seq` order regardless of arrival order
/// (box 3.3) and deduplicates by event `id`, first-wins (box 3.4). A copy of
/// the input is sorted internally, so the caller's slice is never mutated and
/// shuffled delivery yields an identical result.
#[must_use]
pub fn fold(events: &[EventEnvelope]) -> WorldState {
    let mut ordered: Vec<&EventEnvelope> = events.iter().collect();
    // Ascending seq is the ordering authority (box 3.3); `id` breaks ties so
    // the order is total and deterministic even for equal seqs.
    ordered.sort_by(|a, b| a.seq.cmp(&b.seq).then_with(|| a.id.cmp(&b.id)));

    let mut state = WorldState::default();
    for event in ordered {
        state = apply(state, event);
    }
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evt(id: &str, seq: u64, subject: &str, event_type: &str, data: Value) -> EventEnvelope {
        EventEnvelope {
            id: id.to_string(),
            seq,
            time: "2026-06-17T00:00:00Z".to_string(),
            ingest_time: "2026-06-17T00:00:01Z".to_string(),
            subject: subject.to_string(),
            event_type: event_type.to_string(),
            data,
            correlation_id: "corr".to_string(),
            causation_id: "cause".to_string(),
        }
    }

    fn sample_log() -> Vec<EventEnvelope> {
        vec![
            evt(
                "e1",
                1,
                "lane-a",
                "fleet.spawn",
                serde_json::json!({ "x": 1 }),
            ),
            evt(
                "e2",
                2,
                "lane-b",
                "fleet.spawn",
                serde_json::json!({ "x": 9 }),
            ),
            evt(
                "e3",
                3,
                "lane-a",
                "fleet.delta",
                serde_json::json!({ "x": 2, "y": 5 }),
            ),
            evt(
                "e4",
                4,
                "lane-b",
                "fleet.delta",
                serde_json::json!({ "y": 7 }),
            ),
        ]
    }

    // ---- Box 3.1: pure deterministic reducer ----

    #[test]
    fn same_log_yields_byte_identical_state() {
        let log = sample_log();
        let a = fold(&log);
        let b = fold(&log);
        let bytes_a = serde_json::to_vec(&a).expect("serialize a");
        let bytes_b = serde_json::to_vec(&b).expect("serialize b");
        assert_eq!(
            bytes_a, bytes_b,
            "two independent folds must be byte-identical"
        );
    }

    #[test]
    fn reducer_consults_no_external_source() {
        // Two single-event applies of the same (state, event) pair must be
        // identical. There is no clock/RNG input to perturb, by construction;
        // this asserts the output depends only on (state, event).
        let event = evt(
            "e1",
            1,
            "lane-a",
            "fleet.spawn",
            serde_json::json!({ "x": 1 }),
        );
        let first = apply(WorldState::default(), &event);
        let second = apply(WorldState::default(), &event);
        assert_eq!(first, second);
    }

    // ---- Box 3.2: per-entity fold keyed by subject ----

    #[test]
    fn per_entity_fold_keyed_by_subject() {
        let state = fold(&sample_log());
        assert_eq!(state.entities.len(), 2);

        let a = &state.entities["lane-a"];
        assert_eq!(a.event_count, 2);
        assert_eq!(a.last_seq, 3);
        assert_eq!(a.last_event_type, "fleet.delta");
        assert_eq!(a.data["x"], serde_json::json!(2));
        assert_eq!(a.data["y"], serde_json::json!(5));

        let b = &state.entities["lane-b"];
        assert_eq!(b.event_count, 2);
        assert_eq!(b.last_seq, 4);
        // lane-b's x must still be 9 — lane-a's events never touched it.
        assert_eq!(b.data["x"], serde_json::json!(9));
        assert_eq!(b.data["y"], serde_json::json!(7));
    }

    #[test]
    fn interleaved_subjects_do_not_cross_contaminate() {
        // Folding lane-a alone must produce the same lane-a state as folding
        // the interleaved log.
        let interleaved = fold(&sample_log());
        let a_only = fold(&[
            evt(
                "e1",
                1,
                "lane-a",
                "fleet.spawn",
                serde_json::json!({ "x": 1 }),
            ),
            evt(
                "e3",
                3,
                "lane-a",
                "fleet.delta",
                serde_json::json!({ "x": 2, "y": 5 }),
            ),
        ]);
        assert_eq!(interleaved.entities["lane-a"], a_only.entities["lane-a"]);
    }

    // ---- Box 3.3: ascending seq order ----

    #[test]
    fn out_of_order_delivery_does_not_change_state() {
        let ordered = sample_log();
        let mut shuffled = sample_log();
        shuffled.reverse();
        shuffled.swap(0, 2);
        let from_ordered = fold(&ordered);
        let from_shuffled = fold(&shuffled);
        assert_eq!(
            serde_json::to_vec(&from_ordered).unwrap(),
            serde_json::to_vec(&from_shuffled).unwrap(),
        );
    }

    #[test]
    fn arrival_time_is_not_the_ordering_key() {
        // Two later-seq events arrive before an earlier-seq event for the same
        // subject; the fold must apply them in ascending seq order, so the
        // last_event_type reflects the highest-seq event, not the last-arrived.
        let shuffled = vec![
            evt("e3", 3, "lane-a", "type-3", serde_json::json!({ "v": 3 })),
            evt("e2", 2, "lane-a", "type-2", serde_json::json!({ "v": 2 })),
            evt("e1", 1, "lane-a", "type-1", serde_json::json!({ "v": 1 })),
        ];
        let state = fold(&shuffled);
        let a = &state.entities["lane-a"];
        assert_eq!(a.last_seq, 3);
        assert_eq!(a.last_event_type, "type-3");
        assert_eq!(a.data["v"], serde_json::json!(3));
    }

    // ---- Box 3.4: idempotent apply with dedupe by id ----

    #[test]
    fn duplicate_event_id_is_a_no_op() {
        let event = evt(
            "dup",
            1,
            "lane-a",
            "fleet.spawn",
            serde_json::json!({ "x": 1 }),
        );
        let once = apply(WorldState::default(), &event);
        let twice = apply(once.clone(), &event);
        assert_eq!(once, twice, "re-applying the same id must be a no-op");
        assert_eq!(twice.entities["lane-a"].event_count, 1);
    }

    #[test]
    fn at_least_once_redelivery_is_safe() {
        let log = sample_log();
        let single = fold(&log);

        // Redeliver the whole batch appended to itself.
        let mut redelivered = log.clone();
        redelivered.extend(log.clone());
        let doubled = fold(&redelivered);

        assert_eq!(
            serde_json::to_vec(&single).unwrap(),
            serde_json::to_vec(&doubled).unwrap(),
            "redelivering the batch must equal the single-delivery fold",
        );
    }

    #[test]
    fn fold_does_not_mutate_caller_slice() {
        let log = sample_log();
        let before: Vec<u64> = log.iter().map(|e| e.seq).collect();
        let _ = fold(&log);
        let after: Vec<u64> = log.iter().map(|e| e.seq).collect();
        assert_eq!(before, after, "fold must sort a copy, never the input");
    }
}
