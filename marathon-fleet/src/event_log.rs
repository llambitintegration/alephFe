//! Event-log stage: the append-only NDJSON event log (box 2.7).
//!
//! Every captured signal is normalized into a single append-only NDJSON file —
//! one JSON value per line — which is the source of truth for both live play and
//! replay (design Decision 2, data-flow `events.jsonl`). Two properties make the
//! log a resumable checkpointable stream:
//!
//! - **Byte offset = resumable cursor.** Each appended line occupies a known byte
//!   range; the byte offset just *past* a line is a [`Cursor`] from which a
//!   reader can resume and read the very next line. Because lines are
//!   `\n`-terminated and never rewritten, an offset stays valid for the life of
//!   the file.
//! - **Checkpoint AFTER apply.** A consumer reads a line, applies it, and only
//!   then advances its checkpoint to the cursor past that line. If it crashes
//!   between reading and checkpointing, it re-reads the line on restart — i.e.
//!   delivery is at-least-once, which is exactly why apply must be idempotent
//!   (the dedupe-by-`id` of [`crate::transport`] / [`crate::projection`]).
//!
//! This module is pure and synchronous: it operates over an in-memory byte
//! buffer (`Vec<u8>`) so it tests headlessly. Persisting that buffer to a real
//! file on disk is a thin, untyped write that a later box wires in; the cursor
//! arithmetic here is byte-for-byte identical to seeking in the on-disk file.

use crate::event::EventEnvelope;

/// A resumable position in the log: a byte offset into the NDJSON buffer.
///
/// A `Cursor` always points at a line boundary — either the start of the buffer
/// (`0`) or the byte immediately after a `\n`. Reading from a cursor yields the
/// line starting there (if any).
pub type Cursor = usize;

/// An append-only NDJSON event log over an in-memory byte buffer.
///
/// Lines are serialized [`EventEnvelope`]s, each terminated by a single `\n`.
/// The buffer is only ever appended to, so previously issued [`Cursor`]s remain
/// valid.
#[derive(Debug, Clone, Default)]
pub struct EventLog {
    bytes: Vec<u8>,
}

/// One line read back from the log: the parsed event and the [`Cursor`] *after*
/// it. A consumer checkpoints to `next` only after it has applied `event`.
#[derive(Debug, Clone, PartialEq)]
pub struct LogLine {
    /// The event parsed from this NDJSON line.
    pub event: EventEnvelope,
    /// The byte offset immediately past this line's terminating `\n` — the
    /// cursor to checkpoint *after* applying `event`.
    pub next: Cursor,
}

impl EventLog {
    /// A fresh, empty log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one normalized event as a single NDJSON line, returning the
    /// [`Cursor`] just past it (the cursor a consumer would checkpoint to after
    /// applying it).
    ///
    /// Serialization is one compact JSON object with no embedded newlines,
    /// followed by exactly one `\n`, so each event occupies exactly one line.
    pub fn append(&mut self, event: &EventEnvelope) -> Cursor {
        let line = serde_json::to_string(event).expect("EventEnvelope serializes");
        debug_assert!(!line.contains('\n'), "NDJSON line must not embed a newline");
        self.bytes.extend_from_slice(line.as_bytes());
        self.bytes.push(b'\n');
        self.bytes.len()
    }

    /// The cursor at the current end of the log (one past the last byte).
    #[must_use]
    pub fn end(&self) -> Cursor {
        self.bytes.len()
    }

    /// The raw NDJSON bytes (e.g. to flush to a real file).
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Read the single line starting at `cursor`, returning the parsed event and
    /// the cursor past it, or `None` if `cursor` is at the end of the log.
    ///
    /// `cursor` MUST be a line boundary (`0` or a value previously returned by
    /// [`append`](Self::append) / [`LogLine::next`]).
    #[must_use]
    pub fn read_from(&self, cursor: Cursor) -> Option<LogLine> {
        if cursor >= self.bytes.len() {
            return None;
        }
        let rest = &self.bytes[cursor..];
        // Find this line's terminating newline.
        let nl = rest
            .iter()
            .position(|&b| b == b'\n')
            .expect("appended lines are always \\n-terminated");
        let line = &rest[..nl];
        let event: EventEnvelope =
            serde_json::from_slice(line).expect("each NDJSON line is a valid EventEnvelope");
        Some(LogLine {
            event,
            next: cursor + nl + 1,
        })
    }

    /// Read every line from `cursor` to the end, in append order.
    #[must_use]
    pub fn read_all_from(&self, mut cursor: Cursor) -> Vec<LogLine> {
        let mut out = Vec::new();
        while let Some(line) = self.read_from(cursor) {
            cursor = line.next;
            out.push(line);
        }
        out
    }

    /// Number of lines (events) in the log.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.iter().filter(|&&b| b == b'\n').count()
    }

    /// Whether the log is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

/// A consumer's resumable checkpoint into an [`EventLog`].
///
/// The cursor advances *after* apply (the at-least-once contract): call
/// [`Checkpoint::drain`] to read undelivered lines, apply each, and advance.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Checkpoint {
    /// The byte offset up to which events have been applied.
    pub cursor: Cursor,
}

impl Checkpoint {
    /// A checkpoint at the start of the log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply every not-yet-applied line via `apply`, advancing the checkpoint to
    /// the cursor *past each line only after* `apply` returns for it.
    ///
    /// Returns the number of lines applied this call. Calling again with no new
    /// appends applies nothing (the cursor is at the end).
    pub fn drain<F: FnMut(&EventEnvelope)>(&mut self, log: &EventLog, mut apply: F) -> usize {
        let mut count = 0;
        while let Some(line) = log.read_from(self.cursor) {
            apply(&line.event); // apply happens first ...
            self.cursor = line.next; // ... then the cursor advances (checkpoint AFTER apply)
            count += 1;
        }
        count
    }
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
            // Include nested data to prove compact one-line serialization.
            data: serde_json::json!({ "seq": seq, "nested": { "k": "v" } }),
            correlation_id: "corr".to_string(),
            causation_id: "cause".to_string(),
        }
    }

    #[test]
    fn log_is_one_json_value_per_line() {
        let mut log = EventLog::new();
        log.append(&evt("a", 1));
        log.append(&evt("b", 2));

        let text = std::str::from_utf8(log.as_bytes()).unwrap();
        // Trailing newline => split_terminator gives exactly the lines.
        let lines: Vec<&str> = text.split_terminator('\n').collect();
        assert_eq!(lines.len(), 2, "expected one line per event");
        for line in &lines {
            assert!(!line.is_empty());
            // Each line is itself a complete, parseable JSON value.
            let v: serde_json::Value = serde_json::from_str(line).expect("line is valid JSON");
            assert!(v.is_object());
        }
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn byte_offset_is_a_resumable_cursor() {
        let mut log = EventLog::new();
        let after_a = log.append(&evt("a", 1));
        let after_b = log.append(&evt("b", 2));

        // Resume from the cursor just past the first line: read exactly the rest.
        let resumed = log.read_all_from(after_a);
        assert_eq!(resumed.len(), 1);
        assert_eq!(resumed[0].event.id, "b");
        assert_eq!(resumed[0].next, after_b);

        // Reading from end yields nothing.
        assert!(log.read_from(after_b).is_none());

        // From the start we read everything back in append order.
        let all = log.read_all_from(0);
        let ids: Vec<&str> = all.iter().map(|l| l.event.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b"]);
        // The cursor returned by append matches the line boundary on read-back.
        assert_eq!(all[0].next, after_a);
    }

    #[test]
    fn checkpoint_advances_after_apply_and_is_resumable() {
        let mut log = EventLog::new();
        log.append(&evt("a", 1));
        log.append(&evt("b", 2));

        let mut applied: Vec<String> = Vec::new();
        let mut cp = Checkpoint::new();

        // Drain: apply both, cursor advances to end.
        let n = cp.drain(&log, |e| applied.push(e.id.clone()));
        assert_eq!(n, 2);
        assert_eq!(applied, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(cp.cursor, log.end(), "checkpoint should sit at the end");

        // Re-draining with no new appends applies nothing (cursor past all).
        assert_eq!(cp.drain(&log, |e| applied.push(e.id.clone())), 0);
        assert_eq!(applied.len(), 2);

        // Append more after the checkpoint, then drain only the new tail.
        log.append(&evt("c", 3));
        let n2 = cp.drain(&log, |e| applied.push(e.id.clone()));
        assert_eq!(n2, 1);
        assert_eq!(
            applied,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
        assert_eq!(cp.cursor, log.end());
    }

    #[test]
    fn checkpoint_is_after_apply_so_a_pre_checkpoint_crash_redelivers() {
        // Model a crash *during* apply, before the checkpoint advances: apply
        // observes the event but the cursor is NOT advanced past it, so on
        // resume the same event is delivered again (at-least-once).
        let mut log = EventLog::new();
        log.append(&evt("a", 1));
        let mut cp = Checkpoint::new();

        // First (interrupted) attempt: panic inside apply after observing it.
        let observed = std::cell::RefCell::new(Vec::<String>::new());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            cp.drain(&log, |e| {
                observed.borrow_mut().push(e.id.clone());
                panic!("crash after observing, before checkpoint");
            });
        }));
        assert!(result.is_err(), "apply was supposed to panic");
        assert_eq!(observed.borrow().as_slice(), ["a".to_string()]);
        // Crucially the checkpoint did NOT advance (apply never returned).
        assert_eq!(
            cp.cursor, 0,
            "checkpoint must not advance before apply completes"
        );

        // Resume: the same event is redelivered (caller must dedupe by id).
        observed.borrow_mut().clear();
        let n = cp.drain(&log, |e| observed.borrow_mut().push(e.id.clone()));
        assert_eq!(n, 1);
        assert_eq!(observed.borrow().as_slice(), ["a".to_string()]);
        assert_eq!(cp.cursor, log.end());
    }
}
