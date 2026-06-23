## ADDED Requirements

### Requirement: Self-state observation resource (`world://self`)

The gateway SHALL expose an MCP resource `world://self` that returns the requesting agent's own avatar state — read from the most recent `WorldSnapshot` only — including at minimum its position-derived self-readout, facing/orientation, health/condition, and current weapon/loadout state as projected by the snapshot. The resource SHALL describe only the requesting agent's own avatar and SHALL NOT include any other agent's private state.

#### Scenario: Agent reads its own state

- **WHEN** an agent reads `world://self`
- **THEN** the gateway SHALL return that agent's avatar self-state derived from the latest `WorldSnapshot`
- **AND** the payload SHALL describe only the requesting agent's avatar, not any other agent's private state

#### Scenario: Self-state reflects the latest snapshot

- **WHEN** the sim has advanced and produced newer snapshots since the agent's previous read
- **THEN** a subsequent read of `world://self` SHALL reflect the most recent snapshot's self-state and SHALL NOT return a stale earlier snapshot

### Requirement: Nearby-entities observation resource (`world://nearby`)

The gateway SHALL expose an MCP resource `world://nearby` that returns the entities within the requesting agent's area of interest, each described by its entity type, its bearing relative to the requesting avatar, and a NUMERIC relative distance from the requesting avatar. The list SHALL be derived from the most recent `WorldSnapshot` and SHALL contain only entities the requesting avatar can legitimately perceive (LOS/AOI), per CONTRACT §8.

#### Scenario: Nearby list carries type, bearing, and numeric distance

- **WHEN** an agent reads `world://nearby` and perceptible entities exist in its area of interest
- **THEN** each returned entity SHALL include its entity type, its bearing relative to the requesting avatar, and a numeric relative distance
- **AND** every returned entity SHALL be one the requesting avatar can legitimately perceive

#### Scenario: Entities outside the area of interest are excluded

- **WHEN** an entity exists in the snapshot but lies outside the requesting avatar's LOS/AOI
- **THEN** that entity SHALL NOT appear in the `world://nearby` payload

### Requirement: Optional natural-language summary resource (`world://summary`)

The gateway SHALL expose an MCP resource `world://summary` that returns an optional natural-language summary rendered over the SAME LOS/AOI-scoped snapshot data that backs `world://self` and `world://nearby`. The summary SHALL NOT reveal any state that falls outside the requesting agent's area of interest, and SHALL be derived from the most recent `WorldSnapshot` only.

#### Scenario: Summary describes only perceptible state

- **WHEN** an agent reads `world://summary`
- **THEN** the gateway SHALL return a natural-language description rendered over the agent's LOS/AOI-scoped snapshot data
- **AND** the summary SHALL NOT mention or imply any entity or state outside the requesting agent's area of interest

#### Scenario: Summary is consistent with the structured resources

- **WHEN** `world://summary` is read for the same snapshot that backs a read of `world://self` and `world://nearby`
- **THEN** the summary SHALL be consistent with that structured data and SHALL NOT introduce entities absent from the scoped `world://nearby` set

### Requirement: Low-frequency event doorbell resource (`world://events`)

The gateway SHALL expose an MCP resource `world://events` that delivers low-frequency event-subscription notifications ("doorbells") for significant events the requesting avatar can perceive — such as taking damage, a perceptible geometry change, or a target being spotted. A doorbell SHALL signal only that the agent should come read the latest observation resources; it SHALL NOT carry per-frame world state and SHALL NOT act as a per-frame push channel. Every event signalled SHALL be one within the requesting avatar's LOS/AOI.

#### Scenario: Doorbell signals the agent to come read

- **WHEN** a significant perceptible event occurs for the requesting avatar
- **THEN** the gateway SHALL deliver a low-frequency doorbell notification on `world://events`
- **AND** the notification SHALL prompt the agent to read the observation resources rather than embedding per-frame world state

#### Scenario: Events resource is not a per-frame push channel

- **WHEN** the sim advances many ticks without a significant perceptible event for the requesting avatar
- **THEN** `world://events` SHALL NOT emit a notification per tick or per frame

#### Scenario: Doorbells respect the area of interest

- **WHEN** a significant event occurs outside the requesting avatar's LOS/AOI
- **THEN** no doorbell for that event SHALL be delivered to the requesting agent on `world://events`

### Requirement: Observation payload is a trust boundary scoped to the avatar's area of interest

Every observation resource (`world://self`, `world://nearby`, `world://summary`, `world://events`) SHALL be LOS/AOI-scoped to what the requesting agent's avatar can legitimately perceive, and the observation payload SHALL be treated as a TRUST BOUNDARY: an agent MUST NOT be able to read any world state outside its area of interest through any observation resource, per CONTRACT §8. Scoping SHALL be enforced server-side and SHALL NOT depend on client cooperation.

#### Scenario: No resource leaks out-of-AOI state

- **WHEN** an agent reads any observation resource
- **THEN** the returned payload SHALL contain only state within that agent's avatar's LOS/AOI
- **AND** state outside the area of interest SHALL NOT be derivable from any observation resource's payload

#### Scenario: Scoping is enforced server-side

- **WHEN** a request is crafted to attempt to widen the returned scope beyond the avatar's area of interest
- **THEN** the gateway SHALL still scope the payload to the avatar's LOS/AOI and SHALL NOT honor any client-supplied widening of scope

### Requirement: Observation scope equals action-authorization scope

The scope of what an agent can observe through these resources SHALL equal the scope of what that agent is authorized to act upon — what an agent can see bounds what it can act on, per CONTRACT §8. An agent SHALL NOT be able to observe any entity or region through an observation resource that lies outside the LOS/AOI scope used to authorize its actions.

#### Scenario: Observation and action share one scope

- **WHEN** an agent's action-authorization scope is determined for a given snapshot
- **THEN** the set of entities and regions observable through `world://self`, `world://nearby`, `world://summary`, and `world://events` SHALL be bounded by that same LOS/AOI scope

#### Scenario: Unobservable targets are unactionable

- **WHEN** an entity does not appear in any observation resource because it is outside the agent's area of interest
- **THEN** that entity SHALL also be outside the agent's action-authorization scope

### Requirement: Reads come from the most recent snapshot without blocking the tick

Every observation resource read SHALL be served from the most recent `WorldSnapshot` only and SHALL NOT block waiting for a live tick to complete. The sim SHALL own the clock and continue ticking independently; an observation read SHALL return the latest available snapshot immediately rather than synchronizing with the tick loop.

#### Scenario: Read returns latest snapshot without waiting for a tick

- **WHEN** an agent reads an observation resource while the sim is mid-tick or between ticks
- **THEN** the gateway SHALL return the most recent completed `WorldSnapshot` immediately
- **AND** the read SHALL NOT block on the next tick boundary and SHALL NOT stall the tick loop

#### Scenario: Concurrent reads never block each other or the sim

- **WHEN** multiple agents read observation resources concurrently
- **THEN** each read SHALL be served from the most recent snapshot without blocking another read or the sim's tick loop
