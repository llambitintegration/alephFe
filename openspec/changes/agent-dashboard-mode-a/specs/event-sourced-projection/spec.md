## ADDED Requirements

### Requirement: Pure Deterministic Reducer

The projection SHALL fold the ordered event log into a per-entity `WorldState` via a pure, deterministic reducer of the form `apply(state, event) -> state`. The reducer MUST have no access to a clock, no source of randomness, and no I/O; its output MUST be a function of the prior state and the applied event alone. Folding the same ordered log MUST yield byte-identical `WorldState` regardless of when, where, or how many times the fold runs.

#### Scenario: Same log yields byte-identical state

- **WHEN** the same ordered event log is folded twice, in two independent fold runs
- **THEN** both runs produce a byte-identical `WorldState`

#### Scenario: Reducer consults no external source

- **WHEN** the reducer is invoked on a `(state, event)` pair with the system clock advanced and the process RNG re-seeded between invocations
- **THEN** the resulting `WorldState` is identical to a run performed without those external changes

#### Scenario: Per-entity fold keyed by subject

- **WHEN** the log contains events for two distinct `subject` (entity/stream) ids interleaved
- **THEN** each entity's `WorldState` reflects only the events whose `subject` is its own id, and neither entity's state is altered by the other's events

### Requirement: Producer-Owned Seq Ordering Authority

The projection SHALL order events strictly by the producer-owned monotonic per-feed `seq` (1-based), never by arrival, delivery, or wall-clock time. The fold MUST apply events in ascending `seq` order; out-of-order delivery MUST NOT change the resulting `WorldState` (per CONTRACT §10.2 invariant 1 and §7).

#### Scenario: Out-of-order delivery does not change state

- **WHEN** events that arrive in a shuffled delivery order are folded
- **THEN** the resulting `WorldState` is identical to the state produced by folding the same events in ascending `seq` order

#### Scenario: Arrival time is not the ordering key

- **WHEN** two events with later `seq` arrive before an event with earlier `seq`
- **THEN** the projection applies them in ascending `seq` order, not in arrival order

### Requirement: Idempotent Apply With Dedupe By Id

The projection SHALL make `apply` idempotent by deduplicating events by their event `id`, so that at-least-once delivery is safe (per CONTRACT §10.2 invariant 3, pairing dedupe-by-`id` with QoS 1). Re-applying an event whose `id` has already been folded MUST leave the `WorldState` unchanged.

#### Scenario: Duplicate event id is a no-op

- **WHEN** an event is folded, and then a second event carrying the same `id` is folded
- **THEN** the `WorldState` after the second fold is identical to the `WorldState` after the first

#### Scenario: At-least-once redelivery is safe

- **WHEN** an entire batch of events is folded, then the same batch is redelivered and folded again
- **THEN** the resulting `WorldState` is identical to the single-delivery fold

### Requirement: State-As-Of-T Reconstruction From File-Resident Anchor

The projection SHALL reconstruct state-as-of-T by folding from the nearest file-resident snapshot anchor whose `asOf` is less than or equal to T, then replaying the tail of events whose `seq` exceeds that anchor's `lastSeq` and whose `time` is less than or equal to T (per CONTRACT §7). The reconstructed state MUST equal a full fold of the prefix of the log up to T.

#### Scenario: Scrub reconstructs from nearest anchor plus tail

- **WHEN** state-as-of-T is requested and the log contains at least one anchor with `asOf` less than or equal to T
- **THEN** the projection folds the events with `seq` greater than the chosen anchor's `lastSeq` and `time` less than or equal to T onto that anchor, and the result equals a full fold of the log prefix up to T

#### Scenario: Nearest anchor at or before T is chosen

- **WHEN** the log contains multiple anchors with `asOf` values both before and after T
- **THEN** the projection selects the anchor with the greatest `asOf` that is still less than or equal to T as the fold base

#### Scenario: No anchor before T falls back to full fold

- **WHEN** state-as-of-T is requested and no anchor has `asOf` less than or equal to T
- **THEN** the projection reconstructs state by folding the log prefix from the beginning up to T

### Requirement: Restart Recovery By Log Replay

The projection SHALL survive process restart by replaying the append-only event log from the latest anchor, producing the same `WorldState` that existed before the restart. The consumer MUST NOT synthesize anchors of its own; anchors are producer-owned (per CONTRACT §7).

#### Scenario: Restart replays to pre-restart state

- **WHEN** the projection is rebuilt after a restart by replaying the log from the latest anchor
- **THEN** the recovered `WorldState` is identical to the `WorldState` held immediately before the restart

#### Scenario: Consumer synthesizes no anchors

- **WHEN** the projection processes a log and is then restarted
- **THEN** the log contains no anchors authored by the consumer; only producer-emitted anchors are present

### Requirement: Snapshots Are A Deletable Cache, Not Source Of Truth

The projection SHALL treat snapshot anchors strictly as an optimization that bounds seek cost, never as the source of truth. Deleting all snapshots and rebuilding by a full re-fold of the append-only log MUST yield the same `WorldState`. The append-only event log is the single source of truth (per CONTRACT §10.2 invariant 2 and Decision 3).

#### Scenario: Delete-and-rebuild yields the same state

- **WHEN** all snapshot anchors are removed from consideration and the `WorldState` is rebuilt by a full re-fold of the log
- **THEN** the rebuilt `WorldState` is identical to the state produced when snapshots were used to bound the fold

#### Scenario: Corrupt snapshot is discarded in favor of the log

- **WHEN** a snapshot anchor disagrees with the result of folding the log up to the same `seq`
- **THEN** the projection relies on the log-derived state, treating the snapshot as a deletable cache rather than authority
