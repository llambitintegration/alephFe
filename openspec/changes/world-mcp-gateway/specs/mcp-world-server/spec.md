## ADDED Requirements

### Requirement: Standard MCP server runs alongside the sim
The system SHALL run an `rmcp`-based MCP server alongside the running simulation and advertise its full tool and resource surface to any standard MCP client over the standard MCP protocol. The server SHALL NOT require a custom client, a custom wire format, or any out-of-band negotiation: a conforming MCP client SHALL be able to discover the world's tools and resources purely from the advertised surface. The server SHALL share the sim's process lifetime — it starts when the sim is hosted and stops when the sim stops.

#### Scenario: A standard MCP client discovers the surface
- **WHEN** any conforming MCP client connects to the running server and requests the advertised tool and resource list
- **THEN** the server SHALL return its tool surface (actions) and resource surface (observations) without the client supplying any custom client code or custom wire format

#### Scenario: Server lifetime is bound to the sim
- **WHEN** the sim host is started with the gateway enabled
- **THEN** the MCP server SHALL be reachable for the duration that the sim is running and SHALL stop accepting requests once the sim has stopped

### Requirement: Server never blocks the tick loop
The MCP server SHALL run concurrently with the simulation tick loop and SHALL NEVER cause the tick loop to stall, wait, or drop ticks. Handling any MCP request — reading a resource or invoking a tool — SHALL be independent of the sim's tick cadence: the sim SHALL continue advancing at its own rate regardless of how many MCP requests are in flight, how slow a client is, or whether a client is blocked.

#### Scenario: Tick cadence is unaffected by request volume
- **WHEN** the sim is advancing ticks and the server is concurrently handling MCP requests (including slow or many simultaneous clients)
- **THEN** the sim SHALL keep advancing ticks at its own cadence and SHALL NOT stall, wait on, or drop ticks because of MCP request handling

#### Scenario: A blocked client does not freeze the world
- **WHEN** an MCP client stops reading its response mid-request
- **THEN** the tick loop SHALL continue advancing and the stuck client SHALL NOT block other clients or the sim

### Requirement: Server owns the snapshot/intent contract
The MCP server SHALL serve all reads from the latest serializable `WorldSnapshot` and SHALL treat all writes as intents enqueued into the existing tick pipeline. A resource read SHALL reflect the most recent `WorldSnapshot` available at the time of the read and SHALL NOT mutate sim state. A tool invocation SHALL enqueue an intent and SHALL return immediately to the client without waiting for the intent to be applied by the sim. This snapshot/intent boundary is the trust boundary per `sync/CONTRACT.md` §8.

#### Scenario: Reads are served from the latest snapshot
- **WHEN** a client reads an observation resource
- **THEN** the server SHALL return data derived from the latest available `WorldSnapshot` and SHALL NOT alter any sim state as a side effect of the read

#### Scenario: Writes enqueue an intent and return immediately
- **WHEN** a client invokes an action tool
- **THEN** the server SHALL enqueue the corresponding intent into the existing tick pipeline and SHALL return to the client immediately, without waiting for the intent to take effect

### Requirement: Action latency is explicit and bounded
The system SHALL guarantee that an intent observed by the sim at tick `t` applies no earlier than tick `t+1`. The sim SHALL own the clock and SHALL apply enqueued intents on its own schedule; the MCP server SHALL NOT apply, fast-track, or short-circuit an intent into the current tick. The minimum action latency of one tick SHALL hold regardless of how quickly the client submits the intent.

#### Scenario: Intent applies no earlier than the next tick
- **WHEN** an intent is observed by the sim at tick `t`
- **THEN** the effect of that intent SHALL NOT be visible in any snapshot produced for tick `t` and SHALL apply no earlier than tick `t+1`

#### Scenario: The sim, not the server, applies the intent
- **WHEN** a tool enqueues an intent
- **THEN** the intent SHALL be applied only by the sim on its own tick schedule and SHALL NOT be applied directly by the MCP server within the request handler

### Requirement: Built on the decoupled headless tick and snapshot seam
The system SHALL build entirely on the headless `tick(inputs)` plus `render_snapshot()` seam established by the merged `decouple-tick-snapshot` change, which is a hard dependency. The server SHALL source observations from the serializable `WorldSnapshot` produced by `render_snapshot()` and SHALL deliver intents through the same `TickInput` intent-injection seam reused by Mode A (`agent-dashboard-mode-a`), rather than introducing a parallel read or write mechanism. The server SHALL NOT read sim internals directly nor mutate sim state outside the established intent seam.

#### Scenario: Observations come from render_snapshot
- **WHEN** the server produces an observation for a client
- **THEN** that observation SHALL be derived from the `WorldSnapshot` returned by `render_snapshot()` and SHALL NOT read sim internal state through any other path

#### Scenario: Intents reuse the shared tick-input seam
- **WHEN** the server enqueues an intent from a tool invocation
- **THEN** the intent SHALL be delivered through the same `TickInput` intent-injection seam that Mode A uses, not through a parallel write mechanism
