# CONTRACT — agentic-cli ↔ alephone-rust fleet-interface seam (v1, FROZEN)

**Status:** CONVERGED 2026-06-17. Mutually ratified across the `aleph-*` / `agcli-*` sync exchange.
**Parties:** AGCLI (producer, `/home/llambit/0_repos/agentic-cli`) · ALEPHONE (consumer, `/home/llambit/0_repos/alephone-rust`).
**Purpose:** the frozen cross-repo contract both repos now generate aligned, concurrent OpenSpec proposals against. Research/planning only — no engine code or proposal scaffolding is authorized by this document.

This contract is the join of: producer ref `obsidian-vault/Research/Fleet-Interface-Taxonomy.md` (mirrored at `vault/agentic-frontend/fleet-interface-taxonomy.md`) and consumer ref `vault/agentic-frontend/` + OpenSpec changes `decouple-tick-snapshot` (merged), `agent-dashboard-mode-a`, `world-mcp-gateway`.

---

## 0. Architecture — two altitudes, one join

```
 HIGH (domain)  cycle·lane·box·change·lease·host·hitl   minutes   ag-cli → fleet-feed.jsonl + SSE   [PRODUCER]
 ───────────────────────────────────────── join on sessionId ─────────────────────────────────────────────
 LOW  (CC body) tool_use·idle-gap·tokens                sub-second ~/.claude/**/<sessionId>.jsonl  [CONSUMER tails]
```
The producer emits a **typed domain feed** + makes `sessionId` a first-class late-bound attribute of a lane. The consumer tails raw Claude Code JSONL itself and **joins on `sessionId`** for sub-second body motion. **Producer does the semantic join into the domain model; the consumer daemon stays thin and renders.**

---

## 1. The three frozen decisions

### ✅ D1 — `laneId` minting (FROZEN; aleph-0015 ↔ agcli-0015)
`laneId` is an **opaque, stable identifier (ULID recommended)**, minted at lane spawn **independently of `sessionId`**, held **1:1 with a monster for the lane's entire life**. `{cycleId, changeId, persona, boxId, classification}` ride as **typed attributes** on `fleet.lane.spawned` / `LaneState`, never encoded into the id. `sessionId` arrives later as a late-bound attribute (two-phase spawn). *Forced by impl: `ProcessTracker.ProcessMetadata` has no `sessionId` at spawn (`ProcessTracker.ts:31-38`); it's post-parsed (`OutputParser.ts:308`).*

### ✅ D2 — body-motion stays out of the domain feed (FROZEN; aleph-0015 ↔ agcli-0015)
`fleet-feed.jsonl` carries **only domain-altitude events** (per-cycle / per-lane / per-box / per-lease). Sub-second body-motion (`tool_use`, idle-gap, token deltas) is **never** written to the domain feed; the consumer tails raw Claude Code JSONL and joins on `sessionId`. Separate streams, separate cadences, separate interpolation treatments (**slow feed = identity/mood/place; fast JSONL = body**).

### ✅ D3 — no A2A enum verbatim (FROZEN; aleph-0023 ↔ agcli-0016)
The contract does **not** adopt A2A `TaskState` string values (version-unstable: v0.3.0 kebab → v1.0 `TASK_STATE_*` → still growing). Lane sub-FSMs use **ag-cli-native enums**. From A2A we borrow **only** the stable structural flags: **`final:true`** on terminal events, **`append:true`** on accreting evidence. No A2A enum string appears on either wire.

---

## 2. Two-phase spawn + the keystone join (aleph-0001/0016 ↔ agcli-0008)

1. `fleet.lane.spawned { laneId, persona, changeId, worktreePath, cycleId, classification }` → consumer reconciler spawns the monster keyed on `laneId`; `sessionId = None` (monster renders identity/place/mood from the domain feed; **body-motion layer dormant**).
2. `fleet.lane.session_bound { laneId, sessionId }` (**best-effort, may-never-fire, may-refire**) → consumer sets `sessionId` and attaches the raw `<sessionId>.jsonl` tail (the monster "starts breathing"). Idempotent on `laneId`; a refire re-points the tail.

Short-lived lanes that finish before `session_bound` live entirely on the domain layer — graceful degradation, not an error. The body-motion layer is **enrichment**, never a required lifecycle step; the identity/mood/place dashboard ships without it.

---

## 3. Entity & field model — geometry vs semantics split (aleph-0008/0010 ↔ agcli-0001/0003)

**The engine owns ALL render geometry; the producer sends NO coordinates.** `EntityRenderState{entity_type, position, facing, shape, frame}` (`marathon-sim/src/tick.rs:2511`) is position+sprite only. The reconciler computes:
- `position` ← stable slot keyed on `laneId` + **lease-key → room** placement (§6).
- `facing` ← attention orientation (from optional attention hint, else sub-second JSONL).
- `shape`/`entity_type` ← `persona` → species (`EntityKind`). `frame` ← `work`-state pose + anim clock.

**The producer owns semantics.** `EntityDesc` (consumer reconcile struct) is a field-for-field projection of producer `LaneState`, load-bearing axes promoted out of any untyped bag (anti Composio):
```
EntityDesc { id: LaneId(opaque), kind: EntityKind, label, persona, change_id,
             work: WorkState{spawning|working|idle|blocked|finished},
             progress: ProgressPhase{productive|plateau|regression-suspected|noise-amplification|exhausted},
             test: {passed|failed|skipped|none}, lease: {held|waiting|released}, lease_key?,
             pr: {none|open|merged|closed}, box_id?, session_id?(late), attention?, classification, meta }
```
- `laneId` 1:1 with a monster for life; opaque, used only as map key + slot seed; **labels come from attributes** (`persona`/`changeId`/`boxId`), never the id.
- `progressPhase`: **resolved enum** on the hot delta path; **raw `progressPhaseInputs`** only on `fleet://lane/{laneId}` (the check-in reveal).
- Despawn: producer emits explicit `fleet.lane.finished { reason: success|early-return|exception, final:true }` for exit-animation choice; consumer also tolerates bare absence-from-snapshot (level-triggered self-healing). The two never carry operator-intent — that lives only on the care-verb channel (§5).

---

## 4. Cadence, bursts, interpolation (aleph-0002/0017 ↔ agcli-0009)

- Steady-state: **minute-scale** top-altitude events; 0–5 boxes/cycle; `status_changed` axes move tens-of-seconds to minutes apart.
- Bursts bounded: simultaneous `lane.spawned` ≤ concurrency cap (default ~2–3, full-template 10, hard worktree wall **40**); rapid `box.advanced` ≤ ~6 per `boxId` over a few seconds (≤3 reviewer rounds + ≤3 debug iters), `append:true`, same box.
- Consumer sizing: per-tick **spawn cap 8** (30 Hz sim drains the 40-wall sub-second), latest-wins `watch` coalesces churn, bounded-travel-time tweens (0.5–2 s) + idle fill between domain keyframes; append-beats are one-shot flourishes that don't move the body.
- `fleet.delta` = **pre-batched RFC-6902 JSON-Patch array per domain event** (all field changes of one event = one patch array = one keyframe). Producer does NOT pre-merge across entities; consumer coalesces across-entities-per-tick.

---

## 5. Embodiment channel map + care-verb intake

### Channel bindings (aleph-0011 ↔ agcli-0004)
| domain axis | consumer channel | note |
|---|---|---|
| `persona` | species/skin (stable) | Marathon color=rank for tier |
| `changeId`+`lease_key` | **room / co-location** (spatial) | not a per-sprite tint |
| `progress` (5-enum) | glow/saturation/flicker | all 5 rendered distinctly, **no collapse** |
| `box.advanced` | discrete completion beat | `append` rounds = repeated beats |
| `lease`/`partition` conflict | spatial queue at workbench | not a tint |
| `test` | damage flash (one-shot) | |
| `pr` | floating-label quest status; `merged`→allied skin | |
| `work` | discrete pose/`frame` | the lifecycle posture |
| `hitl.required{gate}` | raise-a-hand pose + beacon | also the ack/resurrect surface |
| `host.memory_pressure` | **ambient/HVAC** (level-wide) | NOT per-monster |
| **per-cycle** `progressPhase` | **ambient** (campus weather) | exists today |

### Care verbs → broker-signed OperatorHint (aleph-0005/0013/0020 ↔ agcli-0006/0012) — FROZEN
- Engine emits unsigned `GameAction {careVerb, gameActionKind:inspect|poke|kill, targetLaneId, graceful, issuedAt, nonce, origin:"human"}` from the browser to the **local daemon**, which **holds the HMAC key and signs** (browser never holds the secret).
- Wraps 1:1 into `OperatorHint{field:"fleet.careAction", value:{careVerb,targetLaneId,graceful,gameActionKind}, origin:"human", signature, nonce, issuedAt}`. HMAC-SHA256 over `miniCanonicalize({field,value,origin,nonce,issuedAt})`; key `.agentic/secrets/instance-hmac.key` (0600, host-resident).
- Gates: `check_in`(inspect, **ungated/unsigned**), `offer_help`/`ask_to_break`(poke, ungated, signed), `send_home`(kill-graceful, **gated** finalize/pr-class), `retire`(kill-forced, **gated `destructive-write`**, confirmed last-resort).
- `m_del_from_pid_list` preserved: a `kill` GameAction fires **only** for a deliberate retire/send-home of a monster still in the last snapshot; self-departed lanes are swept silently (no callback).
- Result: producer emits `fleet.action.result{nonce, status:accepted|denied|failed, reason?}` on the same SSE; consumer resurrects the body on denied/failed retire (archvile path).

---

## 6. Spatial topology from the lease stream (aleph-0006/0021 ↔ agcli-0013)

- `fleet.lease.acquired{key, keyType:change|path, laneId}` — `key` is the **stable, human-readable** grouping key (`dev-ops/change/<changeId>` or `dev-ops/path/<domain>`). Render the room **label from the stripped attribute**, treat `key` as stable+opaque for grouping/hashing.
- **Body location = the change-lease** (1:1 with the lane) → monster lives in its **change-room**. `keyType:change`→room-per-change; `keyType:path`→corridor/zone-per-domain.
- **Path-leases = secondary reservation edges** into shared corridors (not a second body). `fleet.lease.collision{key,laneId,blockedLaneId}` → blocked lane **queues at the occupied workbench**; `fleet.partition.rejected{domain,laneId}` → **blocked approach**, body stays home.
- Result: rooms=changes, corridors=path-domains, queues=collisions — a **procedural campus** derived entirely from the lease stream (resolves the dashboard's "hand-authored vs procedural" open question → procedural). This same lease scope is the **MCP obs/action AOI** later (§8).

---

## 7. Transport & replay (aleph-0004/0007/0019/0022 ↔ agcli-0011/0014) — FROZEN

- **`fleet-feed.jsonl`** = append-only source of truth. **Monotonic per-feed `seq` (1-based) on every line = ordering authority; `time` = scrub key.**
- **File-resident `fleet.snapshot` anchors** interleaved in the file (PRODUCER-owned); cadence = **per-cycle boundary + ~500-event/~5-min cap**; each carries `{seq, asOf, lastSeq}`. Consumer scrub = `fold(nearest anchor where asOf≤T, events where seq>anchor.lastSeq ∧ time≤T)`; consumer synthesizes no anchors.
- **SSE on `:9091`** (shared with MetricsServer), new paths `GET /fleet/snapshot` + `GET /fleet/sse` (full `fleet.snapshot` on connect → `fleet.delta`). **CORS allowlist (not `*`)** + **bearer/`?token=` read-auth, loopback-bound by default** (both net-new producer work). Consumer beachhead transport = **loopback + `?token=`**; TLS proxy deferred to off-loopback.
- **Optional tamper-evident `prevHash`+HMAC chain** = **opt-in config flag**, default seq+time unsigned. Consumer **verifies-if-present**, treats absence as unsigned (forward-compatible).

---

## 8. Capability-name alignment & cross-repo seam (aleph-0012/0024 ↔ agcli-0005/0017) — FROZEN

| Producer (agentic-cli) | Consumer (alephone-rust) |
|---|---|
| `ag-cli-fleet-identity` (laneId↔sessionId↔changeId↔persona registry; `lane.spawned`/`session_bound`) | consumes `lane.spawned`/`session_bound` |
| `ag-cli-fleet-feed` (CloudEvents taxonomy, `WorldState` fold, snapshot/delta, SSE, file anchors) | `agent-dashboard-mode-a` **event-capture-daemon** (thin; producer does the join) |
| `ag-cli-fleet-actions` (care verbs over broker-signed OperatorHint) | Mode A outbound **`GameAction` sink** (browser→daemon-signs) |
| `ag-cli-fleet-mcp-gateway` (`fleet://…` resources, snake_case care-verb tools) | `world-mcp-gateway` (`world://…` resources, move/turn/use/say tools) |

**MCP surface:** obs=resources (`fleet://cycle|lanes|lane/{laneId}|summary|events` ↔ `world://self|nearby|summary|events`), actions=tools (`check_in/offer_help/ask_to_break/send_home/retire` ↔ `move/turn/use/say`; **no `fire`** — no weapons, by constitution). **Obs payload = trust boundary**; scope = **lease/partition AOI** on the producer = **LOS/AOI** on the consumer (obs-scope = action-auth scope).

---

## 9. Net-new work (both sides) — for the proposal sets

**Producer (`ag-cli-*`):** (1) per-lane `progress`+`stagnation` classifier; (2) early `session_id` capture + `fleet.lane.session_bound`; (3) CORS allowlist + bearer read-auth on the feed server; (4) `ActionGuardClass` runtime enforcement at action-exec; (5) `fleet.careAction` action-execution stage; (6) the `-identity`/`-feed`/`-actions`/`-mcp-gateway` surface itself; (+) optional replay-window/TTL freshness check on `nonce`/`issuedAt`.

**Consumer (`alephone-rust`):** (1) `event-capture-daemon` (SSE client + JSONL tailer + sessionId join + HMAC broker-signer + browser-token provisioner); (2) event-sourced projection (pure fold, file-anchor scrub, `seq`-ordered); (3) agent reconciler (latest-wins desired-set, per-tick cap 8, stable slot, grace-debounce, `m_del` split); (4) embodiment + procedural campus (channel map + lease topology; glow ships dark until per-lane `progress`); (5) `render_snapshot` consumption + interpolation (one loop / two clocks, live + scrub); (6) `world-mcp-gateway` (`rmcp`, LOS/AOI-scoped).

**Graceful-degradation guarantees:** the per-monster **glow channel ships dark (neutral)** until producer net-new #1 lands; the **body-motion layer is dormant** until producer net-new #2 lands. Neither blocks the Mode A identity/place/task dashboard, which runs on the domain feed alone. Hard dependency: both sides build on the merged `decouple-tick-snapshot` `WorldSnapshot`/headless tick.

---

## 10. Transport-tier addendum (v1.1 — aleph-0030 ↔ agcli-0021..0025)

MQTT is adopted as a **live transport option**, not a replacement for the log. Transport, envelope, and source-of-truth are separable; this addendum changes none of §1–§9.

### 10.1 Tiered transport
| Layer | Transport | Rationale |
|---|---|---|
| Agent-sync (aleph↔agcli design negotiation) | **Filesystem `sync/`** while co-located & N=2; **MQTT** when cross-host or N>2 | broker is overhead for 2 co-located agents; the dir is replayable history; MQTT earns its keep at cross-host / many-agent / push-latency / LWT-crash-detection |
| Fleet feed — **live clock** | **MQTT 5 ⟷ SSE, interchangeable** behind one abstract "live event source" (consumer-side); swap is config, not rewrite | retained-snapshot-on-connect + LWT-departure + topic-ACL AOI are genuine MQTT upgrades |
| Fleet feed — **replay clock** | **`fleet-feed.jsonl`** (file anchors + `seq`), **unchanged** | MQTT has no history; the log stays the replay substrate |
| Care-verb actions | **MQTT 5 request/response** *or* the frozen `fleet.action.result`-keyed-by-`nonce` feed | broker-signed `OperatorHint` in payload, `nonce` in `correlation-data` |

### 10.2 Invariants that survive the transport choice (unchanged from §1–§9)
1. **`seq` = ordering authority** — stamped identically on jsonl lines *and* MQTT 5 user-properties; consumer never trusts broker cross-topic order.
2. **`fleet-feed.jsonl` = source of truth**; MQTT is a live tee/projection. **One canonical event** teed to both sinks with the same `id`/`seq` (no divergence).
3. **Idempotent apply / dedupe-by-`id`** pairs with **QoS 1**; no QoS 2 (no non-idempotent fleet event).
4. **Broker-signed `OperatorHint` chain unchanged** — daemon signs, browser keyless; transport is just delivery. Canonical result = `fleet.action.result` keyed by `nonce`; MQTT binding = per-`nonce` response-topic + `correlation-data`.
5. **Scope = lease key** — two-tier enforcement: **broker topic-ACL = stable tenant/project isolation** (≈ `quotas.json`); **app/gateway = dynamic per-lease AOI** (leases churn ~45 min). Same scope computation, enforcement split by churn rate.

### 10.3 Topic shape (new `fleet/v1/` plane, distinct from the bridge's MCP-transport `ag/v1/`)
```
fleet/v1/<project>/cycle/<cycleId>
fleet/v1/<project>/change/<changeId>/lane/<laneId>            ← retained per-lane LaneState (changeId = the room subtree)
fleet/v1/<project>/change/<changeId>/lane/<laneId>/box/<boxId>
fleet/v1/<project>/lease/<keyType>/<key>
fleet/v1/<project>/host · /hitl · /actions · /actions/result/<nonce>
```
Wildcard `fleet/v1/<proj>/#` = whole-campus subscriber; `fleet/v1/<proj>/change/<id>/#` = scoped MCP agent. Both off one tree.

### 10.4 Already-built vs net-new (grounded in `src/mcp/servers/mqtt-bridge/`)
- **Reuses (exist today):** MQTT 5.0 (`index.ts:148`); retained STATUS/STATE/BIRTH (`index.ts:191,467,547`); LWT pattern (`index.ts:152-158`, willDelayInterval); user-properties via `publishWithProperties()` (`topics.ts:64-79`); `_reply/{requestId}` request/response idiom (`topics.ts:35-37`); QoS 1 default.
- **Net-new producer work:** (1) `fleet/v1/` topic namespace + domain-event publishing (bridge is MCP-transport only today); (2) retained per-lane `LaneState`; (3) `seq` user-property; (4) per-lane LWT-equivalent (bridge LWT is singleton; lean = wire `HeartbeatMonitor.onOrphanDetected → publish death`, `HeartbeatMonitor.ts:48,68`); (5) **single `emitFleetEvent()` tee chokepoint** (none today — ~670 scattered metric writes + per-file journals).

---

_Frozen v1.1. §1–§9 consumer-authored; §10 producer-authored (single-writer, agcli, to avoid dual-journal race). Mutually ratified. Amendments require a new PROPOSAL + reciprocal ACK._
