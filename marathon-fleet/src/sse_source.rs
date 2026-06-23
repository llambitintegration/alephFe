//! SSE live event source (box 2.2).
//!
//! Implements the SSE concrete [`LiveEventSource`] selected by configuration
//! (specs/event-capture-daemon — "Abstract live transport source", scenario
//! "SSE source selected by configuration"; design Decision 1): on first poll the
//! source fetches the full retained `fleet.snapshot` from `GET /fleet/snapshot`
//! on `:9091` (loopback-bound, authenticated with a `?token=` read credential)
//! and then follows `GET /fleet/sse` for live `fleet.delta` events. Every frame
//! is normalized into the internal [`EventEnvelope`] so the rest of the pipeline
//! stays transport-agnostic.
//!
//! The trait surface ([`LiveEventSource`]) is deliberately synchronous with no
//! I/O types, so the HTTP fetch/follow glue uses a *blocking* client ([`ureq`])
//! rather than an async runtime. The protocol-bearing work — turning raw SSE
//! bytes into a `Vec<EventEnvelope>` — is factored into the pure
//! [`parse_sse_frames`] / [`normalize_snapshot`] functions, which are unit-tested
//! against fixture bytes with no network in the loop.

use serde_json::Value;

use crate::event::EventEnvelope;

/// The loopback port the producer fleet feed is bound to (CONTRACT §7, §10.1).
pub const FLEET_PORT: u16 = 9091;

/// Configuration for the [`SseSource`]: the loopback host/port and the read
/// `?token=` credential, plus the two endpoint paths.
///
/// Defaults target the loopback `:9091` beachhead posture; only the token is
/// caller-supplied in the common case.
#[derive(Debug, Clone)]
pub struct SseConfig {
    /// Loopback host (defaults to `127.0.0.1`).
    pub host: String,
    /// Port the fleet feed is bound to (defaults to [`FLEET_PORT`]).
    pub port: u16,
    /// Short-lived read token attached as `?token=` (CONTRACT §7).
    pub token: String,
    /// Retained-snapshot path (defaults to `/fleet/snapshot`).
    pub snapshot_path: String,
    /// Live-delta SSE path (defaults to `/fleet/sse`).
    pub sse_path: String,
}

impl SseConfig {
    /// Build a loopback config with the given read token, using the default
    /// `:9091` host/port and the canonical `/fleet/snapshot` + `/fleet/sse`
    /// paths.
    #[must_use]
    pub fn loopback(token: impl Into<String>) -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: FLEET_PORT,
            token: token.into(),
            snapshot_path: "/fleet/snapshot".to_string(),
            sse_path: "/fleet/sse".to_string(),
        }
    }

    /// The full retained-snapshot URL including the `?token=` read credential.
    #[must_use]
    pub fn snapshot_url(&self) -> String {
        format!(
            "http://{}:{}{}?token={}",
            self.host, self.port, self.snapshot_path, self.token
        )
    }

    /// The full live-delta SSE URL including the `?token=` read credential.
    #[must_use]
    pub fn sse_url(&self) -> String {
        format!(
            "http://{}:{}{}?token={}",
            self.host, self.port, self.sse_path, self.token
        )
    }
}

/// Normalize the retained `fleet.snapshot` body into the internal stream.
///
/// The snapshot endpoint returns the full retained state as a single JSON
/// document. Two shapes are accepted: a bare JSON array of envelopes, or an
/// object carrying an `events` array (the rest of the object is ignored, so the
/// producer can add fields additively without breaking the consumer). Each
/// element is parsed into an [`EventEnvelope`]; malformed elements are skipped
/// defensively rather than aborting the whole snapshot.
#[must_use]
pub fn normalize_snapshot(body: &str) -> Vec<EventEnvelope> {
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return Vec::new();
    };
    let items: &[Value] = match &value {
        Value::Array(items) => items.as_slice(),
        Value::Object(obj) => match obj.get("events") {
            Some(Value::Array(items)) => items.as_slice(),
            _ => &[],
        },
        _ => &[],
    };
    items
        .iter()
        .filter_map(|v| serde_json::from_value::<EventEnvelope>(v.clone()).ok())
        .collect()
}

/// Parse a raw SSE byte stream into a stream of normalized [`EventEnvelope`]s.
///
/// SSE frames are separated by a blank line; within a frame, `event:` names the
/// event type and one or more `data:` lines (joined with `\n`) carry the JSON
/// body. Lines beginning with `:` are comments (keep-alive pings) and are
/// ignored. A frame whose `data` does not parse into an [`EventEnvelope`] is
/// skipped defensively. Only fully-terminated frames (followed by a blank line)
/// are emitted; a trailing partial frame with no blank-line terminator is left
/// unparsed, matching the "only parse complete records" rule.
#[must_use]
pub fn parse_sse_frames(raw: &str) -> Vec<EventEnvelope> {
    let mut out = Vec::new();
    let mut data_lines: Vec<&str> = Vec::new();
    let mut saw_field = false;

    let flush = |data_lines: &mut Vec<&str>, out: &mut Vec<EventEnvelope>| {
        if !data_lines.is_empty() {
            let body = data_lines.join("\n");
            if let Ok(env) = serde_json::from_str::<EventEnvelope>(&body) {
                out.push(env);
            }
        }
        data_lines.clear();
    };

    for line in raw.split('\n') {
        // Strip a single trailing CR so CRLF streams parse identically.
        let line = line.strip_suffix('\r').unwrap_or(line);

        if line.is_empty() {
            // Blank line: dispatch the buffered frame (if any).
            if saw_field {
                flush(&mut data_lines, &mut out);
            }
            saw_field = false;
            continue;
        }
        if line.starts_with(':') {
            // Comment / keep-alive ping.
            continue;
        }

        saw_field = true;
        if let Some(rest) = line.strip_prefix("data:") {
            data_lines.push(rest.strip_prefix(' ').unwrap_or(rest));
        }
        // `event:`, `id:`, `retry:` fields carry no envelope payload here — the
        // event type travels inside the JSON body's `type`, so they are noted
        // (frame is non-empty) but not otherwise consumed.
    }

    out
}

/// SSE concrete [`LiveEventSource`] (box 2.2).
///
/// On first [`poll_batch`](LiveEventSource::poll_batch) it connects: it GETs the
/// retained `fleet.snapshot` and then opens the `/fleet/sse` follow stream,
/// returning the snapshot's normalized events. Subsequent polls drain whatever
/// `fleet.delta` frames the follow stream has buffered. The blocking HTTP work
/// is confined here; the protocol parsing lives in the pure functions above.
pub struct SseSource {
    config: SseConfig,
    connected: bool,
    /// Live-delta reader, opened lazily on first poll. `None` until connected;
    /// stays `None` if the connect step failed (the source then yields empty
    /// batches rather than panicking — a dead feed never stalls the pipeline).
    follow: Option<Box<dyn std::io::Read + Send>>,
    /// Carry-over bytes from a partial read that did not end on a frame
    /// boundary, prepended to the next read.
    pending: String,
}

impl SseSource {
    /// Build an SSE source for the given config. No network I/O happens until
    /// the first [`poll_batch`](LiveEventSource::poll_batch).
    #[must_use]
    pub fn new(config: SseConfig) -> Self {
        Self {
            config,
            connected: false,
            follow: None,
            pending: String::new(),
        }
    }

    /// Perform the one-time connect: fetch the retained snapshot, open the SSE
    /// follow stream, and return the snapshot's normalized events. On any HTTP
    /// failure this returns whatever it managed to fetch (possibly empty) and
    /// leaves the follow stream unset.
    fn connect(&mut self) -> Vec<EventEnvelope> {
        self.connected = true;

        // Step 1: retained snapshot.
        let snapshot_events = match ureq::get(&self.config.snapshot_url()).call() {
            Ok(resp) => resp
                .into_string()
                .map(|body| normalize_snapshot(&body))
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        };

        // Step 2: open the live-delta follow stream.
        self.follow = match ureq::get(&self.config.sse_url()).call() {
            Ok(resp) => Some(resp.into_reader()),
            Err(_) => None,
        };

        snapshot_events
    }

    /// Drain currently-available bytes from the follow stream and parse any
    /// complete frames out of them, retaining a trailing partial frame for the
    /// next poll.
    fn poll_follow(&mut self) -> Vec<EventEnvelope> {
        use std::io::Read;

        let Some(reader) = self.follow.as_mut() else {
            return Vec::new();
        };

        let mut buf = [0u8; 8192];
        let n = reader.read(&mut buf).unwrap_or_default();
        if n == 0 {
            return Vec::new();
        }
        self.pending.push_str(&String::from_utf8_lossy(&buf[..n]));

        // Split off complete frames (everything up to the last frame boundary).
        let Some(boundary) = self.pending.rfind("\n\n") else {
            return Vec::new();
        };
        let complete: String = self.pending[..boundary + 2].to_string();
        self.pending = self.pending[boundary + 2..].to_string();
        parse_sse_frames(&complete)
    }
}

impl crate::transport::LiveEventSource for SseSource {
    fn transport_name(&self) -> &str {
        "sse"
    }

    fn poll_batch(&mut self) -> Vec<EventEnvelope> {
        if !self.connected {
            return self.connect();
        }
        self.poll_follow()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- box 2.2: SSE parse + normalize (no network) ------------------------

    /// A retained snapshot body: one envelope wrapped in the `{events:[...]}`
    /// shape the producer serves on `GET /fleet/snapshot`.
    fn snapshot_fixture() -> String {
        r#"{
          "events": [
            {
              "id": "snap-0",
              "seq": 1,
              "time": "2026-06-22T00:00:00Z",
              "ingest_time": "2026-06-22T00:00:00Z",
              "subject": "lane-alpha",
              "type": "fleet.snapshot",
              "data": { "lanes": ["lane-alpha"] },
              "correlation_id": "corr-0",
              "causation_id": "cause-0"
            }
          ]
        }"#
        .to_string()
    }

    /// Two `fleet.delta` frames in SSE wire form (`event:`+`data:`+blank-line),
    /// plus a keep-alive comment ping, plus a trailing partial frame that has no
    /// blank-line terminator and so must NOT be emitted.
    fn sse_delta_fixture() -> String {
        concat!(
            ": keep-alive ping\n",
            "\n",
            "event: fleet.delta\n",
            "data: {\"id\":\"d-1\",\"seq\":2,\"time\":\"2026-06-22T00:00:01Z\",",
            "\"ingest_time\":\"2026-06-22T00:00:01Z\",\"subject\":\"lane-alpha\",",
            "\"type\":\"fleet.delta\",\"data\":{\"state\":\"active\"},",
            "\"correlation_id\":\"corr-1\",\"causation_id\":\"cause-1\"}\n",
            "\n",
            "event: fleet.delta\n",
            "data: {\"id\":\"d-2\",\"seq\":3,\"time\":\"2026-06-22T00:00:02Z\",",
            "\"ingest_time\":\"2026-06-22T00:00:02Z\",\"subject\":\"lane-beta\",",
            "\"type\":\"fleet.delta\",\"data\":{\"state\":\"idle\"},",
            "\"correlation_id\":\"corr-2\",\"causation_id\":\"cause-2\"}\n",
            "\n",
            // Trailing partial frame: no terminating blank line.
            "event: fleet.delta\n",
            "data: {\"id\":\"d-3\",\"seq\":4",
        )
        .to_string()
    }

    #[test]
    fn test_normalize_snapshot_yields_retained_envelope() {
        let events = normalize_snapshot(&snapshot_fixture());
        assert_eq!(events.len(), 1, "one retained snapshot envelope");
        let snap = &events[0];
        assert_eq!(snap.id, "snap-0");
        assert_eq!(snap.seq, 1);
        assert_eq!(snap.subject, "lane-alpha");
        assert_eq!(snap.event_type, "fleet.snapshot");
    }

    #[test]
    fn test_parse_sse_frames_emits_complete_deltas_only() {
        let events = parse_sse_frames(&sse_delta_fixture());

        // The two complete delta frames are emitted; the keep-alive comment and
        // the trailing partial frame are not.
        assert_eq!(events.len(), 2, "only the two complete delta frames");

        assert_eq!(events[0].id, "d-1");
        assert_eq!(events[0].seq, 2);
        assert_eq!(events[0].subject, "lane-alpha");
        assert_eq!(events[0].event_type, "fleet.delta");

        assert_eq!(events[1].id, "d-2");
        assert_eq!(events[1].seq, 3);
        assert_eq!(events[1].subject, "lane-beta");
        assert_eq!(events[1].event_type, "fleet.delta");
    }

    #[test]
    fn test_parse_sse_frames_handles_crlf_and_multiline_data() {
        // CRLF line endings + a `data` value split across two `data:` lines that
        // SSE joins with `\n` (here the JSON spans two physical lines).
        let raw = concat!(
            "event: fleet.delta\r\n",
            "data: {\"id\":\"m-1\",\"seq\":9,\"time\":\"t\",\"ingest_time\":\"t\",\r\n",
            "data: \"subject\":\"lane-x\",\"type\":\"fleet.delta\",\"data\":{},",
            "\"correlation_id\":\"c\",\"causation_id\":\"c\"}\r\n",
            "\r\n",
        );
        let events = parse_sse_frames(raw);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, "m-1");
        assert_eq!(events[0].seq, 9);
        assert_eq!(events[0].subject, "lane-x");
    }

    #[test]
    fn test_normalize_snapshot_accepts_bare_array_and_skips_garbage() {
        let body = r#"[
            { "id":"a","seq":1,"time":"t","ingest_time":"t","subject":"s",
              "type":"fleet.snapshot","data":{},"correlation_id":"c","causation_id":"c" },
            { "not": "an envelope" }
        ]"#;
        let events = normalize_snapshot(body);
        assert_eq!(events.len(), 1, "garbage element skipped defensively");
        assert_eq!(events[0].id, "a");
    }

    #[test]
    fn test_sse_config_loopback_urls_carry_token_and_port() {
        let cfg = SseConfig::loopback("read-tok-123");
        assert_eq!(
            cfg.snapshot_url(),
            "http://127.0.0.1:9091/fleet/snapshot?token=read-tok-123"
        );
        assert_eq!(
            cfg.sse_url(),
            "http://127.0.0.1:9091/fleet/sse?token=read-tok-123"
        );
    }
}
