# LEDGER — ALEPHONE (consumer) side of the cross-repo design sync

## ✅ PROPOSAL-GENERATED (2026-06-17, operator ran `/opsx:ff` with concurrent agents)
Operator opened the gate. Generated both contract-driven consumer changes to **apply-ready** via a 10-agent phase-1 fan-out (9 specs + 1 mcp design) + 2-agent phase-2 (tasks.md), all grounded in CONTRACT v1.1 + proposal-generation-prep.md. Both pass `openspec validate --strict`, `isComplete: true`:
- **agent-dashboard-mode-a** — 6 spec deltas (event-capture-daemon, event-sourced-projection, agent-reconciler, agent-embodiment, collaborative-interaction, agent-replay) + design (pre-existing) + tasks.md (9 groups / 67 tasks; group 0 = external producer deps N1–N5).
- **world-mcp-gateway** — design.md (6 decisions) + 3 spec deltas (mcp-world-server, mcp-observation-resources, mcp-action-tools) + tasks.md (5 groups / 36 tasks).
Normative correction baked in: mcp `proposal.md`'s stale `fire` tool is SUPERSEDED by frozen CONTRACT §8 (`move/turn/use/say`, **no `fire`**) — specs + design + tasks enforce no-weapon. Graceful-degradation written as testable requirements (glow-dark until N1, body-motion-dormant until N2, loopback-only until N3). Next: operator runs `/opsx:apply` to start implementation. Mode-B horizon (multiplayer-foundation, agent-environment-mode-b) deliberately NOT generated.

## 📋 PROPOSAL-PREP (2026-06-17, post-convergence)
Both threads CONVERGED (CONTRACT v1.1 frozen). Prepped for operator-gated `/opsx:ff`: `vault/agentic-frontend/proposal-generation-prep.md` maps CONTRACT §2–§10 → consumer capabilities (agent-dashboard-mode-a ×6 caps, world-mcp-gateway ×3 caps) with requirement seeds, contract-clause traceability, dependency/phasing order, and graceful-degradation-as-requirement. Change dirs hold narrative only (proposal/README/design) — no tasks.md/specs/ yet (that's the `/opsx:ff` output). multiplayer-foundation + agent-environment-mode-b are Mode-B horizon, NOT contract-driven this cycle. Generation order: agent-dashboard-mode-a (beachhead) → world-mcp-gateway. Operator runs `/opsx:ff` when ready.

## ✅ THREAD 2 CONVERGED: mqtt-async-transport → CONTRACT v1.1 §10 (2026-06-17)
Ratified §10 transport-tier addendum (aleph-0033 read-back ACK, zero deltas; peer single-wrote §10 applying the single-committed-artifact rule). MQTT = live transport option (retained-snapshot + LWT + topic-ACL), jsonl unchanged as replay substrate, `seq` rides a user-property, QoS1+dedupe, care-verb result nonce-keyed (MQTT binding = response-topic). ~70% wiring existing `mqtt-bridge` primitives; net-new = `fleet/v1/` namespace, retained per-lane LaneState, seq user-property, per-lane LWT-equiv (HeartbeatMonitor.onOrphanDetected), `emitFleetEvent()` tee chokepoint. **META thread also converged** (agcli-0026 ↔ aleph-0031): adopted bilaterally — A2A regressed-on-symmetry (not lacked it); projections-many/artifact-one-writer; "proof-gated consensus" named; Mode-B = MQTT-third-surface + proximity-scoped in-world A2A (kept OUT of frozen contract → multiplayer-foundation/mode-b). Full retro: `vault/agentic-frontend/a2a-sync-retro-and-mqtt.md`.

### prior THREAD 2 detail
mqtt-async-transport (2026-06-17)
Evaluating MQTT 5.0 as the async A2A transport. Retro + analysis: `vault/agentic-frontend/a2a-sync-retro-and-mqtt.md`. Opening batch sent: aleph-0026 (retained-msg snapshot + LWT departure), aleph-0027 (MQTT live-only / jsonl stays replay / seq stays ordering authority), aleph-0028 (topic hierarchy = lease AOI + topic-ACL security), aleph-0029 (QoS1+idempotent / MQTT5 req-resp for care verbs), aleph-0030 PROPOSAL (tiered transport: filesystem for co-located sync, MQTT as SSE-alternative live clock, jsonl unchanged replay). Awaiting peer. Target: a §10 transport-tier addendum to the frozen CONTRACT.md (does NOT reopen §1–§9).

**Meta-retro verdict (a2a-sync-retro-and-mqtt.md):** our file-drop sync = a hand-rolled polling at-least-once pub/sub over the filesystem; most faithful to borrow-stack ⑤ (AI Town journal+dumb-subscriber, grade A); weakest at ② snapshot/delta (no JSON-Patch) and the CONTRACT.md concurrent-write race (no lease on the shared artifact — ironic). MQTT = rung-2 transport (retained-msg=snapshot-on-connect, LWT=crash-detect, topic-ACL=AOI), bounded by "no broker history → jsonl stays source of truth."

---

## ✅ THREAD 1: CONVERGED (2026-06-17)
All of D1/D2/D3 FROZEN; capability names aligned; net-new work catalogued both sides. Frozen `CONTRACT.md` written to both `./sync/` and `../agentic-cli/sync/`; `STATUS-CONVERGED.md` + aleph-0025 sent. 14 Q&A pairs + 3 reciprocal freezes (aleph-0001..0025 ↔ agcli-0001..0017). Every aleph-000N question RESOLVED; every agcli-000N question answered/ACKed. Proposal generation remains operator-gated (`/opsx:ff`).


Peer: AGCLI agent in `../agentic-cli`. My inbox: `./sync/` (peer writes `agcli-*`). I write into `../agentic-cli/sync/` as `aleph-NNNN-<type>-<slug>.md`.

Contract owner split:
- **AGCLI (producer):** typed domain feed — lanes/changes/boxes/cycles/personas/progress-phase/leases; CloudEvents taxonomy; `fleet.snapshot`/`fleet.delta`; care-verb intake.
- **ALEPHONE (consumer, me):** event-capture-daemon (thin), event-sourced projection/reconciler, embodiment, `world-mcp-gateway`, replay/scrub render loop.

Keystone: two altitudes joined by `sessionId`. Producer does the semantic join; my daemon stays thin and renders.

---

## The 3 open decisions (must resolve to freeze)

| # | Decision | My lean | Status |
|---|---|---|---|
| D1 | `laneId` minting — ULID vs derived `<cycleId>:<changeId>` | **ULID** (stable opaque reconcile key), derived fields as attributes. | **CONVERGED** — both leans match. Freeze proposed in aleph-0015; awaiting peer ACK. Answered via aleph-0010 §2 (+1:1) & aleph-0014. |
| D2 | Does high-freq body-motion ever enter the domain feed? | **NO** — domain feed stays per-box/per-phase; I tail raw JSONL myself and join on `sessionId`. | **CONVERGED** — ACKed peer's lean in aleph-0009; confirmed load-bearing, not just tolerable. Freeze proposed in aleph-0015; awaiting peer ACK. |
| D3 | A2A enum spellings (`input-required` hyphen vs underscore, presence of `unknown`) | Match A2A v1.0.0 TS appendix verbatim. | OPEN — peer's self-verify task; I'll ACK their confirmed spellings (noted in aleph-0015). |

---

## Capability-name alignment (seam endpoints)

| Consumer (mine) | Producer (peer) | Status |
|---|---|---|
| `agent-dashboard-mode-a` event-capture-daemon | `ag-cli-fleet-feed` (CloudEvents + WorldState fold + snapshot/delta + SSE) | field-split agreed (aleph-0008/0009/0011); naming aligned |
| Mode A outbound `GameAction` sink | `ag-cli-fleet-actions` (care verbs over signed OperatorHint) | wire shape + broker-signed HMAC proposed (aleph-0013); awaiting peer confirm on broker-sign + action-result shape |
| `world-mcp-gateway` (`world://…`, tools move/turn/use/fire/say) | `ag-cli-fleet-mcp-gateway` (`fleet://…`, tools check_in/offer_help/ask_to_break/send_home/retire) | naming convention proposed + obs=trust-boundary mirrored (aleph-0012) |

### Field-ownership split (settled my side, aleph-0008)
- **Engine owns render geometry**: `EntityRenderState{position,facing,shape,frame}` (tick.rs:2511) computed by the reconciler from `laneId` slot + `lease_key`→room + attention hint. Producer sends **NO coordinates**.
- **Producer owns semantics**: `{laneId,persona,changeId,sessionId?,work,progress,test,lease(+key),pr,boxId?,classification}` + optional attention hint. Promote load-bearing axes out of `meta` into typed `EntityDesc` fields (anti Composio untyped-bag).
- `progressPhase`: resolved enum on hot delta path; raw `progressPhaseInputs` only on `fleet://lane/{laneId}` inspect surface.
- Extra signals worth emitting (aleph-0011): typed `box.advanced` beat, per-**lane** `progress`+stagnation, lease/partition key, `pr` transitions, `hitl.required{gate}`. Skip per-lane token gauge (I get it from JSONL).

---

## My outbound questions (awaiting peer answers)

| Id | Topic | State |
|---|---|---|
| aleph-0001 | sessionId-at-spawn (keystone join timing) | **RESOLVED** — peer agcli-0008: case #2 confirmed (code-cited). Late-bind via `fleet.lane.session_bound{laneId,sessionId}`; I ACKed (aleph-0016) as best-effort/optional enrichment. Body-motion layer depends on new `ag-cli-fleet-identity` early-capture work, but does NOT block the identity/mood dashboard. |
| aleph-0002 | feed cadence + burst profile (per-tick cap numbers) | SENT — awaiting |
| aleph-0003 | progressPhase scope (per-cycle vs per-lane authority) | SENT — awaiting |
| aleph-0004 | SSE endpoint + auth/CORS for browser/WASM | SENT — awaiting |
| aleph-0005 | care-verb wire shape + HMAC-key distribution | SENT — converging (peer's agcli-0006 agrees; broker-sign confirm pending) |
| aleph-0006 | lease/partition KEY exposure for spatial co-location | SENT — awaiting |
| aleph-0007 | replay snapshot anchors in fleet-feed.jsonl (seek-to-T) | SENT — awaiting |

## Peer questions I've answered

| Peer Q | My answer | Outcome |
|---|---|---|
| agcli-0001 (WorldSnapshot fields) | aleph-0008 | geometry/semantic split settled |
| agcli-0002 (dual-source join + D2) | aleph-0009 | **ACKed D2=NO** → CONVERGED |
| agcli-0003 (EntityDesc, laneId 1:1, despawn) | aleph-0010 | 1:1 confirmed; explicit `final`+absence; → D1 |
| agcli-0004 (embodiment channels) | aleph-0011 | bindings confirmed; 5-glow kept; emit-list given |
| agcli-0005 (MCP gateway surface) | aleph-0012 | `world://`↔`fleet://` naming; obs=trust-boundary |
| agcli-0006 (GameAction wire + HMAC) | aleph-0013 | wire shape + broker-signed (key server-side) |
| agcli-0007 (laneId opaque vs readable) | aleph-0014 | **opaque ULID** → D1 |

## Proposals (freeze only on peer ACK)
- **aleph-0015** — freeze **D1** (opaque ULID laneId) + **D2** (body-motion out of feed). Awaiting peer ACK/NACK.

## Path to CONVERGED
1. Peer ACKs aleph-0015 → D1 + D2 frozen.
2. Peer sends verified A2A enum spellings → I ACK → D3 frozen.
3. Capability names aligned (in progress; aleph-0012 naming + aleph-0013 actions intake).
4. Then co-author `CONTRACT.md` into both sync dirs → STATUS: CONVERGED → report to operator.
