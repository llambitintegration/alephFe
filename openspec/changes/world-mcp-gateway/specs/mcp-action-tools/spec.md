## ADDED Requirements

### Requirement: Curated High-Level Action Tool Surface

The gateway SHALL expose exactly the high-level, server-validated sim actions as MCP tools: `move`, `turn`, `use`, and `say`. `move` and `turn` SHALL submit movement/orientation intent; `use` SHALL submit the action-key intent; `say` SHALL submit a communication intent. The gateway SHALL NOT expose any tool outside this set (per CONTRACT §8).

#### Scenario: Tool surface advertises exactly move/turn/use/say

- **WHEN** an MCP client lists the tools advertised by the gateway
- **THEN** the returned set contains exactly `move`, `turn`, `use`, and `say`
- **AND** no additional action tool is advertised

#### Scenario: move submits a movement intent

- **WHEN** an authorized agent invokes the `move` tool with a valid movement argument
- **THEN** a movement intent for that agent is enqueued into the tick pipeline

#### Scenario: say submits a communication intent

- **WHEN** an authorized agent invokes the `say` tool with a valid message argument
- **THEN** a communication intent for that agent is enqueued into the tick pipeline

### Requirement: No Weapon or Fire Tool

By constitution (CONTRACT §8, collaborative/deliberative actions only), the gateway SHALL NOT expose any `fire` tool or any other weapon/combat action tool, and SHALL reject any request to invoke such a tool.

#### Scenario: No fire tool is advertised

- **WHEN** an MCP client lists the tools advertised by the gateway
- **THEN** no tool named `fire` and no weapon or combat action tool appears in the set

#### Scenario: Invoking a fire tool is rejected

- **WHEN** an MCP client attempts to invoke a tool named `fire`
- **THEN** the gateway rejects the request as an unknown/unsupported tool
- **AND** no weapon or combat intent is enqueued

### Requirement: Non-Blocking Intent Enqueue

Each action tool SHALL enqueue an intent into the existing tick pipeline and return to the caller WITHOUT blocking on a tick. The tool invocation SHALL NOT wait for the intent to be applied by the sim before returning.

#### Scenario: Tool returns without waiting for a tick

- **WHEN** an authorized agent invokes any action tool while the sim is running
- **THEN** the tool returns to the caller after the intent is enqueued
- **AND** the tool does not block until the next tick is processed

#### Scenario: Tool return does not stall the tick loop

- **WHEN** an authorized agent invokes an action tool
- **THEN** the sim's tick loop continues to advance independently of the tool call

### Requirement: Server-Side Validation Within Agent AOI Scope

Every action SHALL be validated server-side before it is enqueued. An intent SHALL be authorized only within the requesting agent's observation/AOI scope (obs-scope == action-auth scope, CONTRACT §8). An intent that fails validation or falls outside the agent's authorized scope SHALL be rejected and SHALL NOT be enqueued.

#### Scenario: Malformed action argument is rejected

- **WHEN** an agent invokes an action tool with an argument that fails server-side validation
- **THEN** the gateway rejects the invocation
- **AND** no intent is enqueued for that agent

#### Scenario: Action outside the agent's AOI scope is rejected

- **WHEN** an agent invokes an action tool that targets or affects a region outside that agent's observation/AOI scope
- **THEN** the gateway rejects the invocation as out of scope
- **AND** no intent is enqueued

#### Scenario: Valid in-scope action is enqueued

- **WHEN** an agent invokes an action tool with a valid argument fully within its own observation/AOI scope
- **THEN** server-side validation passes
- **AND** the corresponding intent is enqueued into the tick pipeline

### Requirement: Explicit Action Effect Timing

Action effect timing SHALL be explicit: an intent submitted via a tool SHALL be applied on the sim's own schedule, no earlier than the next tick (`t+1`). An intent submitted during the processing of tick `t` SHALL NOT take effect during tick `t`.

#### Scenario: Intent applies no earlier than the next tick

- **WHEN** an agent enqueues an intent during tick `t`
- **THEN** the intent's effect appears in the world no earlier than tick `t+1`
- **AND** the intent has no effect on the world state of tick `t`
