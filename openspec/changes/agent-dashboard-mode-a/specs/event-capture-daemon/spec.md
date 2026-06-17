## ADDED Requirements

### Requirement: Abstract live transport source (SSE or MQTT)

The daemon SHALL subscribe to the producer fleet feed through a single abstract "live event source" whose concrete transport is selected by configuration, per CONTRACT §10.1. Two transports SHALL be supported behind one interface: an SSE source that connects to `GET /fleet/snapshot` for the full retained `fleet.snapshot` and then `GET /fleet/sse` for live `fleet.delta` events on port `:9091` (loopback-bound with a `?token=` read credential, per CONTRACT §7), and an MQTT source that subscribes to the retained `fleet/v1/…` snapshot topics plus the live subtree (per CONTRACT §10.3). Every stage of the daemon downstream of the source SHALL be transport-agnostic: selecting MQTT instead of SSE SHALL be a configuration change only and SHALL NOT alter the normalized event stream, the ordering rules, or the join behavior.

#### Scenario: SSE source selected by configuration

- **WHEN** the daemon starts with the live source configured as SSE
- **THEN** it SHALL first fetch the full `fleet.snapshot` from `GET /fleet/snapshot` on `:9091` using the configured loopback `?token=` credential
- **AND** it SHALL then follow `GET /fleet/sse` for live `fleet.delta` events
- **AND** it SHALL emit the same normalized internal event stream that the rest of the pipeline consumes

#### Scenario: MQTT source selected by configuration

- **WHEN** the daemon starts with the live source configured as MQTT
- **THEN** it SHALL receive the retained snapshot from the `fleet/v1/…` topics on connect and then live events from the same subtree
- **AND** the normalized internal event stream presented to downstream stages SHALL be identical in shape to the SSE case (CONTRACT §10.1, §10.2)

#### Scenario: Transport swap leaves the pipeline unchanged

- **WHEN** the configured transport is switched from SSE to MQTT (or back) with no other change
- **THEN** the projection, reconciler, and join stages SHALL require no code change and SHALL produce equivalent downstream behavior for the same canonical events

### Requirement: Producer `seq` is the ordering authority; events are deduped by `id`

The daemon SHALL order every received event strictly by the producer-owned monotonic `seq`, and SHALL NOT rely on arrival order, delivery order, or broker cross-topic order, per CONTRACT §10.2. Event application SHALL be idempotent: the daemon SHALL dedupe by event `id` so that a redelivered or duplicated event (expected under at-least-once / QoS 1 delivery) produces no additional state change.

#### Scenario: Out-of-order arrival is reordered by seq

- **WHEN** two events for the same stream arrive in an order that does not match their `seq` values
- **THEN** the daemon SHALL apply them in ascending `seq` order regardless of which arrived first

#### Scenario: Duplicate event id is applied once

- **WHEN** an event whose `id` has already been applied is received again (redelivery / at-least-once)
- **THEN** the daemon SHALL recognize the duplicate `id` and SHALL NOT apply it a second time, leaving resulting state unchanged

### Requirement: Independent JSONL tail joined to the domain feed on `sessionId`

The daemon SHALL tail raw Claude Code JSONL transcripts independently of the domain feed and SHALL JOIN that body-motion stream to the domain feed on `sessionId` (the keystone join, per CONTRACT §2 and decision D2). Sub-second body-motion (`tool_use`, idle-gap, token deltas) SHALL be sourced only from the tailed JSONL and SHALL NEVER be expected to arrive on the domain feed. The join SHALL be keyed on `sessionId` delivered late via `fleet.lane.session_bound`, SHALL be idempotent on `laneId`, and a refire SHALL re-point the tail.

#### Scenario: session_bound attaches the body-motion tail

- **WHEN** a `fleet.lane.session_bound { laneId, sessionId }` event is received for a lane whose monster already exists
- **THEN** the daemon SHALL attach the raw `<sessionId>.jsonl` tail for that lane and route its body-motion events to the matching entity keyed on `laneId`

#### Scenario: Body-motion never sourced from the domain feed

- **WHEN** the domain feed delivers lane events
- **THEN** the daemon SHALL NOT expect `tool_use`, idle-gap, or token-delta body-motion on that feed and SHALL derive body-motion exclusively from the joined JSONL tail

#### Scenario: session_bound refire re-points the tail

- **WHEN** a `session_bound` for an existing `laneId` arrives again carrying a different `sessionId`
- **THEN** the daemon SHALL idempotently re-point the tail to the new `<sessionId>.jsonl` without creating a duplicate entity

### Requirement: Daemon-held HMAC key broker-signs outbound care verbs

The daemon SHALL hold the instance HMAC key at `.agentic/secrets/instance-hmac.key` with file mode `0600`, host-resident, per CONTRACT §5. The daemon SHALL broker-sign every outbound care verb that requires a signature: it SHALL receive the unsigned `GameAction` emitted from the browser, wrap it 1:1 into an `OperatorHint{field:"fleet.careAction", …}`, and compute an HMAC-SHA256 signature over `miniCanonicalize({field,value,origin,nonce,issuedAt})`. The browser SHALL never hold the secret; the signing boundary SHALL be the local daemon.

#### Scenario: Signed care verb leaves the daemon, not the browser

- **WHEN** the browser emits an unsigned `GameAction` care verb that requires signing (e.g. `offer_help`, `send_home`, `retire`)
- **THEN** the daemon SHALL wrap it into an `OperatorHint` and attach an HMAC-SHA256 signature computed with the host-resident key
- **AND** no signing key material SHALL ever be transmitted to or held by the browser

#### Scenario: HMAC key file permissions enforced

- **WHEN** the daemon loads `.agentic/secrets/instance-hmac.key`
- **THEN** the key file SHALL be required to be mode `0600` and host-resident

#### Scenario: Ungated check-in passes through unsigned

- **WHEN** the browser emits a `check_in` (inspect) care verb, which CONTRACT §5 classifies as ungated/unsigned
- **THEN** the daemon SHALL forward it without requiring a signature, while still signing the gated/signed verbs

### Requirement: Short-lived browser read tokens for the WASM client

The daemon SHALL provision short-lived read tokens that the WASM browser client uses to authenticate its read-only subscription to the live source, per CONTRACT §7. These tokens SHALL grant read access only (never signing capability) and SHALL be loopback-scoped by default consistent with the beachhead transport posture.

#### Scenario: WASM client receives a read-only token

- **WHEN** the WASM browser client requests access to the live feed through the daemon
- **THEN** the daemon SHALL issue a short-lived read token that authorizes read subscription only
- **AND** the token SHALL NOT confer any ability to sign or emit care verbs directly

#### Scenario: Expired read token is refused

- **WHEN** the WASM client presents a read token whose short lifetime has elapsed
- **THEN** the daemon SHALL refuse the stale token and require a freshly provisioned one

### Requirement: Graceful degradation with no session binding

With no `fleet.lane.session_bound` yet received for a lane, the daemon SHALL render that lane's identity, place, and task from the domain feed alone and SHALL keep the body-motion layer dormant, per the CONTRACT §9 graceful-degradation guarantee. The absence of a session binding SHALL NEVER block, stall, or error the pipeline: a lane that spawns and even finishes before any `session_bound` arrives SHALL live entirely on the domain layer as a normal, non-error case (CONTRACT §2).

#### Scenario: Lane renders from domain feed before session binding

- **WHEN** a `fleet.lane.spawned` event is received but no `session_bound` has yet arrived for that `laneId`
- **THEN** the daemon SHALL surface the lane's identity, place, and task from the domain feed alone
- **AND** it SHALL keep the body-motion layer dormant for that lane without emitting an error or blocking subsequent events

#### Scenario: Short-lived lane finishes with no session binding

- **WHEN** a lane spawns and then emits `fleet.lane.finished` before any `session_bound` is received
- **THEN** the daemon SHALL treat the never-bound lane as a normal completion (graceful degradation, not an error) and SHALL process its lifecycle entirely on the domain layer
