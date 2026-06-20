## ADDED Requirements

### Requirement: Care-verb interaction via proximity, line-of-sight, and the action key

In-world interaction with an agent-monster SHALL be driven by **proximity + line-of-sight (LOS) + the action key** only. The available verbs SHALL be the collaborative care set `check_in / offer_help / ask_to_break / send_home / retire`. There SHALL be **no `fire`, no weapon, and no shoot-to-kill verb** of any kind; the interaction surface is collaborative care only, by constitution (CONTRACT §8). The action key SHALL only be actionable against a target that is within interaction proximity and has clear LOS to the operator's viewpoint.

#### Scenario: Action key with proximity and LOS emits a care verb

- **WHEN** the operator is within interaction proximity of an agent-monster, has clear line-of-sight to it, and presses the action key selecting a care verb
- **THEN** the corresponding care verb (`check_in`, `offer_help`, `ask_to_break`, `send_home`, or `retire`) SHALL be raised against that target

#### Scenario: No line-of-sight suppresses interaction

- **WHEN** the operator is within proximity of an agent-monster but line-of-sight is blocked, and presses the action key
- **THEN** no care verb SHALL be emitted for that target

#### Scenario: There is no weapon or fire verb

- **WHEN** the set of interaction verbs the engine can emit is enumerated
- **THEN** it SHALL contain only `check_in`, `offer_help`, `ask_to_break`, `send_home`, and `retire`, and SHALL NOT contain any `fire`, shoot, or weapon-discharge verb

### Requirement: Care verbs emit a broker-signed OperatorHint

Each emitted care verb SHALL be wrapped 1:1 into an `OperatorHint` with `field:"fleet.careAction"` and `value:{careVerb, targetLaneId, graceful, gameActionKind}`, `origin:"human"`, and delivered to the producer's `fleet.careAction` stage. The signature SHALL be an **HMAC-SHA256 over `miniCanonicalize({field, value, origin, nonce, issuedAt})`**. The browser/WASM client SHALL NOT hold the HMAC key; the **local daemon** SHALL hold the key (`.agentic/secrets/instance-hmac.key`) and perform the signing (CONTRACT §5). The unsigned `check_in` (inspect) verb is the one exception the daemon MAY forward without a signature.

#### Scenario: Daemon signs a care verb the browser raised

- **WHEN** the browser raises an unsigned care action (e.g. `offer_help`) for a target lane and forwards it to the local daemon
- **THEN** the daemon SHALL produce an `OperatorHint{field:"fleet.careAction", value:{careVerb, targetLaneId, graceful, gameActionKind}, origin:"human", nonce, issuedAt, signature}` whose `signature` is the HMAC-SHA256 of `miniCanonicalize({field, value, origin, nonce, issuedAt})` computed with the host-resident key

#### Scenario: Browser never holds the signing key

- **WHEN** a care action originates in the browser/WASM client
- **THEN** the signature SHALL be applied by the local daemon and the browser SHALL never possess or transmit the HMAC key material

#### Scenario: Each verb maps to its gated game-action kind

- **WHEN** a care verb is wrapped into an `OperatorHint`
- **THEN** `check_in` SHALL map to `gameActionKind:inspect`, `offer_help` and `ask_to_break` SHALL map to `gameActionKind:poke`, `send_home` SHALL map to `gameActionKind:kill` with `graceful:true`, and `retire` SHALL map to `gameActionKind:kill` with `graceful:false`

### Requirement: Action results are correlated by nonce

Every emitted care action SHALL carry a unique `nonce`, and its outcome SHALL be correlated back to the originating action **by that `nonce`**. The canonical result is `fleet.action.result{nonce, status:accepted|denied|failed, reason?}`; under the MQTT binding the result SHALL be matched on a per-`nonce` response-topic with `nonce` in the request/response correlation data (CONTRACT §10.4). The engine SHALL apply each result only to the action that minted the matching `nonce`.

#### Scenario: Result matched to its originating action

- **WHEN** the producer emits `fleet.action.result{nonce, status, reason?}` for a previously emitted care action
- **THEN** the engine SHALL correlate the result to the originating action solely by `nonce` and apply the outcome to that action only

#### Scenario: Denied or failed retire resurrects the body

- **WHEN** a `retire` or `send_home` care action receives a `fleet.action.result` with `status:denied` or `status:failed`
- **THEN** the target agent-monster SHALL be restored (archvile resurrect path) rather than left despawned

### Requirement: m_del disambiguation preserves operator intent

A `kill`-kind `GameAction` (from a deliberate `retire` or `send_home`) SHALL be emitted **only** when the operator deliberately acts on a monster that is **still present in the last desired-set snapshot**. An agent that left on its own (terminal `final` event or bare absence from the snapshot) SHALL be swept silently with a despawn animation and **no** outbound care action or callback. This is the `m_del_from_pid_list` disambiguation between "operator acted on it" and "it left on its own" (CONTRACT §5, design Decision 6).

#### Scenario: Deliberate retire of a present monster emits a kill action

- **WHEN** the operator retires (or sends home) a monster that is still present in the last desired-set snapshot
- **THEN** a `kill`-kind care action SHALL be emitted for that target lane

#### Scenario: Self-departed agent is swept silently

- **WHEN** an agent leaves on its own (its lane emits a terminal `final` event, or it is simply absent from the next desired-set snapshot)
- **THEN** the monster SHALL be despawned with its exit animation and **no** care action or kill callback SHALL be emitted

### Requirement: Graded proximity reveal is non-committal

Approaching an agent-monster SHALL progressively **reveal more** information across proxemic zones (public = automap blip; social = identity + presence color + task one-liner; personal = "press Action to inspect"; intimate = action key opens the live terminal) and SHALL **never commit** a care action by approach alone. Care actions SHALL be **opt-in and reversible** wherever the underlying operation is reversible; only the explicit, confirmed `retire` is a non-reversible last resort taken after redirection. Approaching, focusing, or revealing SHALL have no side effect on the target's lifecycle.

#### Scenario: Approaching reveals but does not commit

- **WHEN** the operator moves from a farther proxemic zone into a nearer one around an agent-monster without pressing the action key
- **THEN** progressively more detail SHALL be revealed (blip → identity/task → inspect affordance → terminal) and **no** care action SHALL be emitted

#### Scenario: Retire requires explicit confirmation

- **WHEN** the operator selects the `retire` (forced, non-graceful) verb
- **THEN** the action SHALL require an explicit confirmation before any `kill`-kind care action is emitted, distinguishing it from the reversible/opt-in care verbs
