//! Transport stage: abstract live event sources + capture-side ordering/dedupe.
//!
//! Hosts the [`LiveEventSource`] abstraction (box 2.1): a single internal
//! interface that yields a normalized stream of [`EventEnvelope`]s, so every
//! stage downstream of the source is transport-agnostic. The concrete SSE / MQTT
//! network sources (boxes 2.2–2.4) plug in *behind* this trait and are out of
//! scope here — `LiveEventSource` deliberately models a source as something that
//! yields a batch of already-normalized envelopes (`Vec<EventEnvelope>`), with no
//! mention of HTTP, sockets, brokers, or async, so the capture pipeline that
//! consumes it depends only on the trait and never on the concrete transport.
//!
//! On top of the source, this module provides the capture-layer ordering and
//! dedupe that CONTRACT §10.2 places *before* projection apply:
//! - [`CaptureBuffer`] orders every received event strictly by the
//!   producer-owned monotonic `seq`, reordering out-of-order arrivals before
//!   they are handed downstream (box 2.5).
//! - The same buffer dedupes by event `id`, so a redelivered (at-least-once /
//!   QoS 1) event is admitted exactly once (box 2.6).
//!
//! This is a distinct concern from the projection reducer's own dedupe/order
//! (which guards the fold): here we guarantee the *captured*, *logged* stream is
//! already canonical (ascending `seq`, no duplicate `id`) so the append-only
//! NDJSON log in [`crate::event_log`] is itself the source of truth.

use std::collections::BTreeSet;

use crate::event::EventEnvelope;

/// An abstract live event source (box 2.1).
///
/// A source yields the producer fleet feed as a normalized internal stream of
/// [`EventEnvelope`]s. The concrete transport (SSE, MQTT, or a test fixture) is
/// selected by configuration and lives *behind* this trait; the rest of the
/// capture pipeline is written against `LiveEventSource` alone and so does not
/// depend on which transport produced the events.
///
/// The interface is intentionally pull-based and synchronous: each call to
/// [`LiveEventSource::poll_batch`] returns the events available since the prior
/// call (possibly empty, possibly out-of-order, possibly containing duplicates —
/// the capture layer canonicalizes them). This keeps the trait free of any
/// transport- or runtime-specific machinery (no async, no I/O types in the
/// signature), which is exactly what makes a transport swap a no-op for
/// downstream code.
pub trait LiveEventSource {
    /// A stable identifier for the concrete transport (e.g. `"sse"`, `"mqtt"`),
    /// used for diagnostics only. Downstream behavior MUST NOT branch on it.
    fn transport_name(&self) -> &str;

    /// Yield the batch of normalized events newly available from the source.
    ///
    /// May be empty. The returned events are *not* assumed to be ordered or
    /// deduplicated — those guarantees are established by [`CaptureBuffer`].
    fn poll_batch(&mut self) -> Vec<EventEnvelope>;
}

/// An in-memory [`LiveEventSource`] backed by a fixed list of pre-normalized
/// batches, draining one batch per [`poll_batch`](LiveEventSource::poll_batch)
/// call. Used to exercise the transport-agnostic capture pipeline headlessly and
/// to prove the "transport swap leaves the pipeline unchanged" property without
/// any network I/O.
pub struct ReplaySource {
    name: String,
    batches: std::collections::VecDeque<Vec<EventEnvelope>>,
}

impl ReplaySource {
    /// Build a source that will yield each batch in `batches` in turn.
    #[must_use]
    pub fn new(name: impl Into<String>, batches: Vec<Vec<EventEnvelope>>) -> Self {
        Self {
            name: name.into(),
            batches: batches.into_iter().collect(),
        }
    }
}

impl LiveEventSource for ReplaySource {
    fn transport_name(&self) -> &str {
        &self.name
    }

    fn poll_batch(&mut self) -> Vec<EventEnvelope> {
        self.batches.pop_front().unwrap_or_default()
    }
}

/// Capture-layer ordering + dedupe buffer (boxes 2.5, 2.6).
///
/// Events are admitted via [`CaptureBuffer::admit`] in whatever order they
/// arrive from a [`LiveEventSource`]. The buffer drops any event whose `id` was
/// already admitted (box 2.6) and keeps the survivors sorted strictly ascending
/// by `(seq, id)` (box 2.5). [`CaptureBuffer::canonical`] returns the ordered,
/// deduplicated stream ready to be logged and applied.
#[derive(Debug, Default)]
pub struct CaptureBuffer {
    /// Survivors, kept sorted ascending by `(seq, id)`.
    ordered: Vec<EventEnvelope>,
    /// Ids already admitted — the dedupe ledger.
    seen_ids: BTreeSet<String>,
}

impl CaptureBuffer {
    /// A fresh, empty buffer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Admit one event. Returns `true` if it was newly admitted, `false` if it
    /// was a duplicate `id` and therefore ignored (box 2.6).
    ///
    /// Survivors are kept ordered ascending by `(seq, id)` (box 2.5), so the
    /// arrival order of `admit` calls does not affect [`canonical`].
    pub fn admit(&mut self, event: EventEnvelope) -> bool {
        if self.seen_ids.contains(&event.id) {
            return false;
        }
        self.seen_ids.insert(event.id.clone());
        // Insert at the position that keeps `ordered` sorted by (seq, id).
        let key = (event.seq, event.id.clone());
        let pos = self
            .ordered
            .partition_point(|e| (e.seq, e.id.clone()) < key);
        self.ordered.insert(pos, event);
        true
    }

    /// Admit a whole batch (e.g. one [`LiveEventSource::poll_batch`] result),
    /// returning the number of newly admitted (non-duplicate) events.
    pub fn admit_batch(&mut self, batch: impl IntoIterator<Item = EventEnvelope>) -> usize {
        batch.into_iter().filter(|e| self.admit_ref(e)).count()
    }

    /// Internal helper so `admit_batch` can count without moving twice.
    fn admit_ref(&mut self, event: &EventEnvelope) -> bool {
        self.admit(event.clone())
    }

    /// The ordered, deduplicated stream: ascending by `(seq, id)`, each `id`
    /// appearing exactly once.
    #[must_use]
    pub fn canonical(&self) -> &[EventEnvelope] {
        &self.ordered
    }

    /// Number of distinct events admitted so far.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ordered.len()
    }

    /// Whether any event has been admitted.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ordered.is_empty()
    }
}

/// Drain a [`LiveEventSource`] for `polls` poll cycles into a [`CaptureBuffer`],
/// returning the canonical (ordered, deduped) stream.
///
/// This is the transport-agnostic capture entry point: it consumes only the
/// trait, so swapping the concrete source for another implementation produces an
/// identical canonical stream for the same canonical events ("transport swap
/// leaves the pipeline unchanged").
pub fn capture<S: LiveEventSource>(source: &mut S, polls: usize) -> Vec<EventEnvelope> {
    let mut buffer = CaptureBuffer::new();
    for _ in 0..polls {
        buffer.admit_batch(source.poll_batch());
    }
    buffer.canonical().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evt(id: &str, seq: u64) -> EventEnvelope {
        EventEnvelope {
            id: id.to_string(),
            seq,
            time: format!("2026-06-17T00:00:{seq:02}Z"),
            ingest_time: "2026-06-17T00:00:00Z".to_string(),
            subject: "lane-abc".to_string(),
            event_type: "fleet.delta".to_string(),
            data: serde_json::json!({ "seq": seq }),
            correlation_id: "corr".to_string(),
            causation_id: "cause".to_string(),
        }
    }

    // --- box 2.1: abstract source / transport-agnostic pipeline -------------

    /// A second concrete source with a different internal representation, to
    /// prove the pipeline consumes only the trait.
    struct OtherSource {
        remaining: Vec<EventEnvelope>,
    }
    impl LiveEventSource for OtherSource {
        fn transport_name(&self) -> &str {
            "other"
        }
        fn poll_batch(&mut self) -> Vec<EventEnvelope> {
            // Yields everything at once, unlike ReplaySource's per-batch drain.
            std::mem::take(&mut self.remaining)
        }
    }

    #[test]
    fn transport_swap_leaves_pipeline_unchanged() {
        // Same canonical events delivered through two *different* concrete
        // transports must yield an identical canonical stream — the consumer
        // (`capture`) depends only on the `LiveEventSource` trait.
        let canonical_events = vec![evt("a", 1), evt("b", 2), evt("c", 3)];

        let mut sse = ReplaySource::new(
            "sse",
            vec![
                vec![canonical_events[0].clone()],
                vec![canonical_events[1].clone(), canonical_events[2].clone()],
            ],
        );
        let mut mqtt = OtherSource {
            remaining: canonical_events.clone(),
        };

        let via_sse = capture(&mut sse, 4);
        let via_mqtt = capture(&mut mqtt, 4);

        assert_eq!(via_sse, via_mqtt, "transport swap changed the stream");
        assert_eq!(via_sse, canonical_events);
        // The two sources are genuinely distinct transports.
        assert_ne!(
            ReplaySource::new("sse", vec![]).transport_name(),
            OtherSource { remaining: vec![] }.transport_name()
        );
    }

    // --- box 2.5: order by producer-owned monotonic seq ---------------------

    #[test]
    fn out_of_order_arrival_is_reordered_by_seq() {
        let mut buf = CaptureBuffer::new();
        // Arrive 3, 1, 2 — the reverse/scrambled of seq order.
        buf.admit(evt("c", 3));
        buf.admit(evt("a", 1));
        buf.admit(evt("b", 2));

        let seqs: Vec<u64> = buf.canonical().iter().map(|e| e.seq).collect();
        assert_eq!(seqs, vec![1, 2, 3], "events were not reordered by seq");
    }

    #[test]
    fn capture_orders_across_batches_regardless_of_arrival() {
        // A source whose batches arrive out of seq order across polls.
        let mut src = ReplaySource::new(
            "sse",
            vec![vec![evt("c", 3), evt("a", 1)], vec![evt("b", 2)]],
        );
        let stream = capture(&mut src, 3);
        let ids: Vec<&str> = stream.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b", "c"]);
    }

    // --- box 2.6: dedupe by id (at-least-once) ------------------------------

    #[test]
    fn duplicate_event_id_is_admitted_once() {
        let mut buf = CaptureBuffer::new();
        assert!(buf.admit(evt("a", 1)), "first admit should succeed");
        assert!(!buf.admit(evt("a", 1)), "redelivered id must be rejected");
        // A redelivery carrying a (spuriously) different seq is still a dup by id.
        assert!(!buf.admit(evt("a", 99)));

        assert_eq!(buf.len(), 1, "duplicate id changed the stream");
        assert_eq!(buf.canonical().len(), 1);
        assert_eq!(buf.canonical()[0].seq, 1, "first-write-wins on dup id");
    }

    #[test]
    fn batch_redelivery_is_a_no_op() {
        let mut buf = CaptureBuffer::new();
        let batch = vec![evt("a", 1), evt("b", 2)];
        assert_eq!(buf.admit_batch(batch.clone()), 2);
        // Redeliver the entire batch (at-least-once).
        assert_eq!(buf.admit_batch(batch), 0, "whole-batch redelivery applied");
        assert_eq!(buf.len(), 2);
    }
}
