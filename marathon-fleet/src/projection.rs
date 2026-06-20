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
//! [`reconstruct_as_of`] reconstructs state-as-of-T from the nearest
//! file-resident [`Anchor`] whose `as_of <= T`, then replays the tail
//! (`seq > anchor.last_seq`, `time <= T`) onto it (box 3.5); the result equals a
//! full prefix fold up to T. [`recover_from_anchor`] rebuilds the live
//! `WorldState` after restart by replaying the log from the latest anchor (box
//! 3.6) — anchors are producer-owned, so this module only ever *consumes*
//! anchors, never authors one. [`validate_anchor`] treats an anchor as a
//! deletable cache: a corrupt anchor (whose embedded state disagrees with the
//! log up to its `last_seq`) is discarded in favor of the log-derived state, and
//! deleting all anchors and re-folding yields an identical state (box 3.7).
//!
//! Time comparison (`time <= T`, `as_of <= T`) is lexicographic on the
//! RFC 3339 / ISO 8601 string fields the envelopes already carry (e.g.
//! `"2026-06-17T00:00:00Z"`). For fixed-offset, equal-precision timestamps —
//! which the producer emits — byte order equals chronological order, so no
//! parsing is required.

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

/// A producer-owned snapshot anchor: a cached fold of the log up to a point in
/// time, used only to bound the cost of a later reconstruction.
///
/// An anchor is never authored by this consumer (box 3.6) — it is read from a
/// file produced upstream. It records the timestamp it was taken at (`as_of`),
/// the highest `seq` folded into its `state` (`last_seq`), and the folded
/// `state` itself. Because the append-only log is the single source of truth
/// (box 3.7), an anchor whose `state` disagrees with the log is discarded.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Anchor {
    /// Producer event-time this anchor was taken at (RFC 3339 string). Compared
    /// lexicographically against event `time` and the requested `T`.
    pub as_of: String,
    /// Highest event `seq` folded into `state`. The reconstruction tail is every
    /// event whose `seq` exceeds this.
    pub last_seq: u64,
    /// The folded [`WorldState`] as of `as_of` / `last_seq`.
    pub state: WorldState,
}

/// Fold only the events whose `time <= t` (lexicographic on the RFC 3339 string)
/// — i.e. a full fold of the log prefix up to and including T. This is the
/// authoritative state-as-of-T against which any anchor-bounded reconstruction
/// must agree (box 3.5) and the fallback when no anchor is usable.
#[must_use]
pub fn fold_prefix_to(events: &[EventEnvelope], t: &str) -> WorldState {
    let prefix: Vec<EventEnvelope> = events
        .iter()
        .filter(|e| e.time.as_str() <= t)
        .cloned()
        .collect();
    fold(&prefix)
}

/// Select the anchor with the GREATEST `as_of` that is still `<= t` (box 3.5).
///
/// Returns `None` when no anchor is at or before T, in which case the caller
/// folds from the beginning. `as_of` is compared lexicographically on the
/// RFC 3339 string.
#[must_use]
fn nearest_anchor_at_or_before<'a>(anchors: &'a [Anchor], t: &str) -> Option<&'a Anchor> {
    anchors
        .iter()
        .filter(|a| a.as_of.as_str() <= t)
        .max_by(|a, b| a.as_of.cmp(&b.as_of))
}

/// Reconstruct state-as-of-T from the nearest file-resident anchor plus the tail
/// of the log (box 3.5).
///
/// Selects the anchor with the greatest `as_of <= t` as the fold base — or the
/// empty [`WorldState`] when no anchor qualifies — then folds the tail events
/// whose `seq` exceeds the anchor's `last_seq` AND whose `time <= t` onto it. The
/// result is byte-identical to [`fold_prefix_to`] for the same `t`, so the anchor
/// is purely a cost optimization and never authority (box 3.7).
#[must_use]
pub fn reconstruct_as_of(events: &[EventEnvelope], anchors: &[Anchor], t: &str) -> WorldState {
    match nearest_anchor_at_or_before(anchors, t) {
        // No anchor at or before T: fall back to a full prefix fold from the
        // beginning of the log (box 3.5, third scenario).
        None => fold_prefix_to(events, t),
        Some(anchor) => {
            let tail: Vec<EventEnvelope> = events
                .iter()
                .filter(|e| e.seq > anchor.last_seq && e.time.as_str() <= t)
                .cloned()
                .collect();
            // Order-independent: `apply` dedupes by id and the tail is folded by
            // ascending seq, so a shuffled tail yields the same state.
            let mut ordered: Vec<&EventEnvelope> = tail.iter().collect();
            ordered.sort_by(|a, b| a.seq.cmp(&b.seq).then_with(|| a.id.cmp(&b.id)));
            let mut state = anchor.state.clone();
            for event in ordered {
                state = apply(state, event);
            }
            state
        }
    }
}

/// Recover the live `WorldState` after a process restart by replaying the
/// append-only log from the latest anchor (box 3.6).
///
/// The latest anchor (greatest `last_seq`) bounds the replay; every event whose
/// `seq` exceeds it is folded back on top, reproducing the exact pre-restart
/// state. With no anchor, the whole log is re-folded. This consumer authors NO
/// anchor — it only reads producer-owned ones — so there is no anchor-creation
/// path to call here (box 3.6, "Consumer synthesizes no anchors").
#[must_use]
pub fn recover_from_anchor(events: &[EventEnvelope], anchors: &[Anchor]) -> WorldState {
    match anchors.iter().max_by_key(|a| a.last_seq) {
        None => fold(events),
        Some(anchor) => {
            let tail: Vec<EventEnvelope> = events
                .iter()
                .filter(|e| e.seq > anchor.last_seq)
                .cloned()
                .collect();
            let mut ordered: Vec<&EventEnvelope> = tail.iter().collect();
            ordered.sort_by(|a, b| a.seq.cmp(&b.seq).then_with(|| a.id.cmp(&b.id)));
            let mut state = anchor.state.clone();
            for event in ordered {
                state = apply(state, event);
            }
            state
        }
    }
}

/// Validate an anchor against the single source of truth, the log (box 3.7).
///
/// An anchor is trustworthy only if its embedded `state` equals the result of
/// folding the log up to its own `last_seq`. A corrupt anchor (whose `state`
/// disagrees) returns `false` and MUST be discarded in favor of the log-derived
/// state — the anchor is a deletable cache, never authority.
#[must_use]
pub fn validate_anchor(events: &[EventEnvelope], anchor: &Anchor) -> bool {
    let log_derived: Vec<EventEnvelope> = events
        .iter()
        .filter(|e| e.seq <= anchor.last_seq)
        .cloned()
        .collect();
    fold(&log_derived) == anchor.state
}

// ============================================================================
// Box 2.15: Graceful degradation with no session binding
// ============================================================================
//
// A lane that has never received a `fleet.lane.session_bound` SHALL still render
// its identity, place, and task from the domain feed alone, with the body-motion
// layer dormant; a never-bound spawn->finish is a NORMAL completion, never an
// error. The functions below are PURE projections over a folded
// [`EntityProjection`] — no clock, no RNG, no I/O, no JSONL tail. They read only
// the merged domain `data` and the lane's lifecycle, so they degrade gracefully
// the instant the producer's late `sessionId` binding (producer dep N2) is
// absent, and become faithful with no code change once it lands.

/// The body-motion layer state for a lane (box 2.15).
///
/// Body-motion (`tool_use`, idle-gap, token deltas) is sourced ONLY from the
/// JSONL tail joined on `sessionId` (box 2.10); until a `fleet.lane.session_bound`
/// is observed there is no tail, so the layer is [`BodyMotion::Dormant`]. This is
/// graceful degradation, NOT an error: every other channel renders from the
/// domain feed regardless.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BodyMotion {
    /// No `session_bound` observed: the body-motion layer is dormant. The lane
    /// still renders fully from the domain feed.
    Dormant,
    /// A `session_bound` bound this `session_id`: the body-motion tail is
    /// (or can be) attached and routed to this lane.
    Bound {
        /// The bound `sessionId` the JSONL tail keys on.
        session_id: String,
    },
}

/// The coarse domain lifecycle status a lane reached on the domain feed alone
/// (box 2.15).
///
/// Derived purely from the lane's `last_event_type`. A never-bound lane that
/// reaches [`LaneLifecycle::Finished`] is a normal completion — the absence of a
/// session binding never turns it into an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneLifecycle {
    /// `fleet.lane.spawned` (or any pre-finish domain event) — live on the feed.
    Spawned,
    /// `fleet.lane.finished` — completed on the domain feed.
    Finished,
}

/// A lane's render descriptor derived from the domain feed alone (box 2.15).
///
/// Carries the identity / place / task the daemon surfaces with no session
/// binding, the domain [`lifecycle`](LaneRender::lifecycle) the lane reached, and
/// the [`body_motion`](LaneRender::body_motion) layer state (dormant until a
/// `session_bound`). Building a [`LaneRender`] NEVER fails: a missing field reads
/// as an empty string and an unbound lane reads [`BodyMotion::Dormant`], so the
/// pipeline is never blocked or stalled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneRender {
    /// The lane's identity (the `identity` domain field, empty if absent).
    pub identity: String,
    /// The lane's place (the `place` domain field, empty if absent).
    pub place: String,
    /// The lane's current task (the `task` domain field, empty if absent).
    pub task: String,
    /// The domain lifecycle status reached on the feed alone.
    pub lifecycle: LaneLifecycle,
    /// The body-motion layer state: dormant until a `session_bound` is observed.
    pub body_motion: BodyMotion,
}

/// Read a merged-`data` string field, defaulting to the empty string (no error).
fn data_str(projection: &EntityProjection, key: &str) -> String {
    projection
        .data
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// Render a lane from its folded domain projection alone (box 2.15).
///
/// Surfaces identity/place/task from the merged domain `data`, classifies the
/// domain lifecycle from `last_event_type` (a `fleet.lane.finished` is a normal
/// completion), and reports the body-motion layer as [`BodyMotion::Dormant`]
/// unless a `fleet.lane.session_bound` bound a `sessionId` into the fold — in
/// which case body-motion is [`BodyMotion::Bound`]. Pure: depends only on the
/// projection.
///
/// This honours the graceful-degradation guarantee: a lane that spawned and even
/// finished before any `session_bound` arrived renders entirely on the domain
/// layer, with body-motion dormant, and never errors.
#[must_use]
pub fn lane_render(projection: &EntityProjection) -> LaneRender {
    let lifecycle = if projection.last_event_type == "fleet.lane.finished" {
        LaneLifecycle::Finished
    } else {
        LaneLifecycle::Spawned
    };
    // A `session_bound` carries the `sessionId` into the merged data; without it
    // the body-motion layer is dormant (graceful degradation, not an error). Both
    // the camelCase wire spelling and the snake_case fallback are accepted.
    let session_id = projection
        .data
        .get("sessionId")
        .or_else(|| projection.data.get("session_id"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let body_motion = match session_id {
        Some(session_id) => BodyMotion::Bound { session_id },
        None => BodyMotion::Dormant,
    };
    LaneRender {
        identity: data_str(projection, "identity"),
        place: data_str(projection, "place"),
        task: data_str(projection, "task"),
        lifecycle,
        body_motion,
    }
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

    /// Like [`evt`] but with an explicit producer `time`, for the time-bounded
    /// reconstruction tests (boxes 3.5-3.7).
    fn evt_at(id: &str, seq: u64, subject: &str, time: &str, data: Value) -> EventEnvelope {
        let mut e = evt(id, seq, subject, "fleet.delta", data);
        e.time = time.to_string();
        e
    }

    /// A monotonic log whose `seq` and `time` advance together, so the prefix up
    /// to a given T is unambiguous.
    fn timed_log() -> Vec<EventEnvelope> {
        vec![
            evt_at(
                "e1",
                1,
                "lane-a",
                "2026-06-17T00:00:01Z",
                serde_json::json!({ "x": 1 }),
            ),
            evt_at(
                "e2",
                2,
                "lane-b",
                "2026-06-17T00:00:02Z",
                serde_json::json!({ "x": 9 }),
            ),
            evt_at(
                "e3",
                3,
                "lane-a",
                "2026-06-17T00:00:03Z",
                serde_json::json!({ "x": 2 }),
            ),
            evt_at(
                "e4",
                4,
                "lane-b",
                "2026-06-17T00:00:04Z",
                serde_json::json!({ "y": 7 }),
            ),
            evt_at(
                "e5",
                5,
                "lane-a",
                "2026-06-17T00:00:05Z",
                serde_json::json!({ "z": 3 }),
            ),
        ]
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

    /// A producer-owned anchor taken at event `seq`, with its `state` folded
    /// faithfully from the prefix of `log` up to that `seq`.
    fn anchor_at(log: &[EventEnvelope], as_of: &str, last_seq: u64) -> Anchor {
        let prefix: Vec<EventEnvelope> =
            log.iter().filter(|e| e.seq <= last_seq).cloned().collect();
        Anchor {
            as_of: as_of.to_string(),
            last_seq,
            state: fold(&prefix),
        }
    }

    // ---- Box 3.5: state-as-of-T reconstruction from a file-resident anchor ----

    #[test]
    fn scrub_reconstructs_from_nearest_anchor_plus_tail() {
        let log = timed_log();
        let t = "2026-06-17T00:00:04Z";
        // One anchor at seq 2 (as_of 00:00:02), comfortably before T.
        let anchors = vec![anchor_at(&log, "2026-06-17T00:00:02Z", 2)];

        let reconstructed = reconstruct_as_of(&log, &anchors, t);
        let full = fold_prefix_to(&log, t);
        assert_eq!(
            serde_json::to_vec(&reconstructed).unwrap(),
            serde_json::to_vec(&full).unwrap(),
            "anchor+tail reconstruction must equal the full prefix fold up to T",
        );
        // e5 (seq 5, time 00:00:05) is past T and must NOT be folded in.
        assert_eq!(reconstructed.entities["lane-a"].last_seq, 3);
        assert!(!reconstructed.entities["lane-a"].data.contains_key("z"));
    }

    #[test]
    fn nearest_anchor_at_or_before_t_is_chosen() {
        let log = timed_log();
        let t = "2026-06-17T00:00:04Z";
        // Anchors both before and after T; the greatest as_of <= T (seq 3) wins.
        let anchors = vec![
            anchor_at(&log, "2026-06-17T00:00:01Z", 1),
            anchor_at(&log, "2026-06-17T00:00:03Z", 3),
            // This one is AFTER T and must be ignored.
            anchor_at(&log, "2026-06-17T00:00:05Z", 5),
        ];
        assert_eq!(
            nearest_anchor_at_or_before(&anchors, t).map(|a| a.last_seq),
            Some(3),
            "the greatest as_of still <= T must be selected as the base",
        );
        // And the reconstruction still equals the full prefix fold.
        assert_eq!(
            serde_json::to_vec(&reconstruct_as_of(&log, &anchors, t)).unwrap(),
            serde_json::to_vec(&fold_prefix_to(&log, t)).unwrap(),
        );
    }

    #[test]
    fn no_anchor_before_t_falls_back_to_full_fold() {
        let log = timed_log();
        let t = "2026-06-17T00:00:03Z";
        // The only anchor is AFTER T, so there is no usable base.
        let anchors = vec![anchor_at(&log, "2026-06-17T00:00:05Z", 5)];
        assert!(nearest_anchor_at_or_before(&anchors, t).is_none());

        let reconstructed = reconstruct_as_of(&log, &anchors, t);
        let full = fold_prefix_to(&log, t);
        assert_eq!(
            serde_json::to_vec(&reconstructed).unwrap(),
            serde_json::to_vec(&full).unwrap(),
            "with no anchor <= T, reconstruction folds the prefix from the start",
        );
    }

    // ---- Box 3.6: restart recovery by log replay ----

    #[test]
    fn restart_replays_to_pre_restart_state() {
        let log = timed_log();
        // The state held immediately before the restart is a full fold.
        let pre_restart = fold(&log);
        // The producer has anchored partway through; recovery replays the tail.
        let anchors = vec![anchor_at(&log, "2026-06-17T00:00:03Z", 3)];

        let recovered = recover_from_anchor(&log, &anchors);
        assert_eq!(
            serde_json::to_vec(&recovered).unwrap(),
            serde_json::to_vec(&pre_restart).unwrap(),
            "recovery from the latest anchor must reproduce the pre-restart state",
        );
    }

    #[test]
    fn recovery_with_no_anchor_refolds_whole_log() {
        let log = timed_log();
        assert_eq!(recover_from_anchor(&log, &[]), fold(&log));
    }

    #[test]
    fn consumer_synthesizes_no_anchors() {
        // The recovery API only ever *consumes* anchors; there is no path by
        // which this module authors one. We assert the contract structurally:
        // recovery takes anchors as input and returns a WorldState, never an
        // Anchor, so no consumer-authored anchor can leak out.
        let log = timed_log();
        let recovered: WorldState = recover_from_anchor(&log, &[]);
        // The returned value is a WorldState, not an Anchor — there is no anchor
        // to inspect because none was created. (If recover_from_anchor ever
        // returned/emitted an Anchor, this binding's type would fail to compile.)
        assert_eq!(recovered, fold(&log));
    }

    // ---- Box 3.7: snapshots are a deletable cache, not source of truth ----

    #[test]
    fn delete_and_rebuild_yields_the_same_state() {
        let log = timed_log();
        let t = "2026-06-17T00:00:05Z";
        let anchors = vec![anchor_at(&log, "2026-06-17T00:00:03Z", 3)];

        let with_anchor = reconstruct_as_of(&log, &anchors, t);
        // Delete all anchors and rebuild by a full re-fold of the log.
        let without_anchor = reconstruct_as_of(&log, &[], t);
        assert_eq!(
            serde_json::to_vec(&with_anchor).unwrap(),
            serde_json::to_vec(&without_anchor).unwrap(),
            "delete-and-rebuild must match the anchor-bounded reconstruction",
        );
    }

    #[test]
    fn corrupt_snapshot_is_discarded_in_favor_of_the_log() {
        let log = timed_log();
        // A faithful anchor validates; a tampered one does not.
        let good = anchor_at(&log, "2026-06-17T00:00:03Z", 3);
        assert!(validate_anchor(&log, &good), "honest anchor must validate");

        let mut corrupt = good.clone();
        corrupt
            .state
            .entities
            .get_mut("lane-a")
            .unwrap()
            .data
            .insert("x".to_string(), serde_json::json!(999));
        assert!(
            !validate_anchor(&log, &corrupt),
            "anchor disagreeing with the log up to its last_seq must be rejected",
        );

        // Discarding the corrupt anchor (treating anchors as []) and folding the
        // log yields the trustworthy, log-derived state.
        let log_derived = recover_from_anchor(&log, &[]);
        assert_eq!(log_derived, fold(&log));
    }

    // ---- Box 2.15: Graceful degradation with no session binding ----

    /// A `fleet.lane.*` lifecycle event carrying identity/place/task in `data`.
    fn lane_evt(id: &str, seq: u64, lane: &str, event_type: &str, data: Value) -> EventEnvelope {
        let mut e = evt(id, seq, lane, event_type, data);
        e.event_type = event_type.to_string();
        e
    }

    // Scenario: Lane renders from domain feed before session binding.
    #[test]
    fn lane_renders_from_domain_feed_before_session_binding() {
        // Only a `fleet.lane.spawned` on the domain feed — no session_bound yet.
        let log = vec![lane_evt(
            "e1",
            1,
            "lane-a",
            "fleet.lane.spawned",
            serde_json::json!({
                "identity": "durandal",
                "place": "room-7",
                "task": "build the platform mechanics",
            }),
        )];
        let state = fold(&log);
        let render = lane_render(&state.entities["lane-a"]);

        // Identity/place/task surface from the domain feed alone.
        assert_eq!(render.identity, "durandal");
        assert_eq!(render.place, "room-7");
        assert_eq!(render.task, "build the platform mechanics");
        // The lane is live (spawned), not finished.
        assert_eq!(render.lifecycle, LaneLifecycle::Spawned);
        // Body-motion is dormant with no session binding — and crucially this is
        // a value, not an error or panic: the pipeline is never blocked.
        assert_eq!(render.body_motion, BodyMotion::Dormant);
    }

    #[test]
    fn session_bound_lights_up_the_body_motion_layer() {
        // The same lane later receives a session_bound; body-motion binds.
        let log = vec![
            lane_evt(
                "e1",
                1,
                "lane-a",
                "fleet.lane.spawned",
                serde_json::json!({ "identity": "durandal", "place": "room-7" }),
            ),
            lane_evt(
                "e2",
                2,
                "lane-a",
                "fleet.lane.session_bound",
                serde_json::json!({ "sessionId": "sess-xyz" }),
            ),
        ];
        let state = fold(&log);
        let render = lane_render(&state.entities["lane-a"]);
        assert_eq!(
            render.body_motion,
            BodyMotion::Bound {
                session_id: "sess-xyz".to_string()
            },
            "an observed session_bound binds the body-motion tail"
        );
        // The domain channels are unaffected by the binding.
        assert_eq!(render.identity, "durandal");
        assert_eq!(render.place, "room-7");
    }

    // Scenario: Short-lived lane finishes with no session binding.
    #[test]
    fn never_bound_spawn_to_finish_is_a_normal_completion() {
        // A lane spawns then finishes, with NO session_bound ever received.
        let log = vec![
            lane_evt(
                "e1",
                1,
                "lane-a",
                "fleet.lane.spawned",
                serde_json::json!({ "identity": "tycho", "task": "quick fix" }),
            ),
            lane_evt(
                "e2",
                2,
                "lane-a",
                "fleet.lane.finished",
                serde_json::json!({}),
            ),
        ];
        let state = fold(&log);
        let render = lane_render(&state.entities["lane-a"]);

        // The never-bound lane is a NORMAL completion, not an error.
        assert_eq!(
            render.lifecycle,
            LaneLifecycle::Finished,
            "a never-bound spawn->finish completes normally on the domain layer"
        );
        // Body-motion stayed dormant the whole life of the lane.
        assert_eq!(render.body_motion, BodyMotion::Dormant);
        // Identity/task still rendered from the domain feed.
        assert_eq!(render.identity, "tycho");
        assert_eq!(render.task, "quick fix");
    }

    #[test]
    fn lane_render_tolerates_missing_domain_fields() {
        // A bare spawn with no identity/place/task fields renders empty strings,
        // never panicking — the pipeline is never blocked.
        let log = vec![lane_evt(
            "e1",
            1,
            "lane-a",
            "fleet.lane.spawned",
            serde_json::json!({}),
        )];
        let state = fold(&log);
        let render = lane_render(&state.entities["lane-a"]);
        assert!(render.identity.is_empty());
        assert!(render.place.is_empty());
        assert!(render.task.is_empty());
        assert_eq!(render.body_motion, BodyMotion::Dormant);
        assert_eq!(render.lifecycle, LaneLifecycle::Spawned);
    }
}
