## Context

Mode A renders the operator's **real** agentic-cli `/dev-ops` fleet as inhabitants of a Marathon level: a live, glanceable dashboard you walk through. The architecture is a four-stage pipeline — **event capture → event-sourced projection → reconciler → sim snapshot/render** — plus an **outbound `GameAction` channel** for collaborative interaction. The evidence base is fully worked out in the agentic-frontend vault:

- The event source and its five surfaces (hooks / JSONL tail / OTEL / statusline / SDK stream) — [[claude-code-event-signals]].
- The event-sourcing + interpolation + replay data architecture (the ASCII data-flow, the CloudEvents envelope, the two-clock render loop) — [[data-architecture-event-sourcing]].
- The reconciler contract, the latest-wins desired-set snapshot, and the load-bearing `m_del_from_pid_list` disambiguation — [[control-plane-doom-pattern]].
- The agent-state → monster-behavior channel map and the Marathon bestiary mapping — [[embodiment-agent-state-mapping]].
- The collaborative (non-combat) interaction model and its constitutional grounding — [[gather-style-collaboration]], [[constitutional-wellness-campus]].
- The overall phasing — [[roadmap-mode-a-to-b]].

**This change builds directly on `decouple-tick-snapshot` (Phase 0) and does not re-spec it.** Phase 0 delivers a clean `marathon-sim` `tick(inputs)` + serializable `render_snapshot() -> WorldSnapshot`, decoupled from rendering, and a headless tick loop. Mode A consumes those: the reconciler injects desired state through the sim's existing seams (`TickInput` at `tick.rs`, `entities()`/`render_snapshot()` for read-back, `ecs_world_mut()` for raw spawn, `snapshot()`/`serialize()` for broadcast/replay) and drives a per-tick `update_agents()` path parallel to `update_monsters()`.

## Goals / Non-Goals

**Goals:**
- A live, faithful, **collaborative** dashboard of the real dev-ops fleet, novel today, with no netcode and no GPU serving.
- A single append-only event log as source of truth, supporting both live follow-tail and replay/scrub over one render loop.
- A reconciler that coalesces bursty fleet churn into one diff per game tick and survives a 0→N spawn storm without stalling a frame.
- Faithful embodiment: a monster reads as a debugger view of its agent (lifecycle = pose, confidence = glow, attention = orientation, kind = species).
- An outbound channel where care verbs (check-in / offer-help / ask-to-break / send-home / retire) emit real `GameAction`s to the harness.

**Non-Goals:**
- Re-specifying the tick/snapshot decoupling or the static-wall-mesh fix — those are `decouple-tick-snapshot` (Phase 0), a hard dependency.
- Any netcode, headless authoritative server, multi-human play, or remote transport (Phases 1–2).
- Real-time A/V, an SFU, P2P mesh, or webcams — agents have no camera; "proximity media" becomes "proximity terminal/transcript reveal."
- An MCP gateway or deliberative agents *acting on* the world (Phase MCP / `world-mcp-gateway`) and the full Mode B serving stack (`agent-environment-mode-b`).
- **Any combat/shoot-to-kill interaction.** The verb set is collaborative care, by constitution.

## Decisions

### Decision 1: Layered event capture — hooks spine + JSONL tail + optional gauges

Capture combines three layers, each covering the others' weaknesses ([[claude-code-event-signals]] §7):
- **Layer A — hooks (real-time spine):** register `http`/`command` hooks for `SessionStart/End`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `PermissionRequest`, `Notification`, `Stop`, `SubagentStart`, `SubagentStop`, `PreCompact`. Sub-100ms typed lifecycle triggers drive spawn/despawn/animate. **Hooks always `exit 0`** so a dead dashboard never blocks the agent.
- **Layer B — JSONL transcript tail (content + threading):** each hook payload carries `transcript_path`; tail it for the actual text, tool I/O, and the `parentUuid`/`isSidechain` subagent lineage that hooks alone don't give. Parse defensively (no stable schema; key off `version`, treat unknown fields as additive; only parse `\n`-terminated lines).
- **Layer C — OTEL or statusline (gauges, optional):** token/cost/duration gauges + the `prompt.id` correlation key, or the cheaper per-turn statusline `context_window.used_percentage`.

*Alternatives:* JSONL-only (rejected — file-flush latency, no clean spawn trigger); hooks-only (rejected — no content/threading/replay-of-pre-existing-sessions). Layering is what the canonical reference implementations (disler, claude-office) converge on.

**Attribution keys, used consistently across layers:** monster identity = `session_id`; subagent→parent = hooks `agent_id`/`agent_type` + JSONL `isSidechain`/`parentUuid`; command grouping = OTEL `prompt.id` / the JSONL `parentUuid` chain. Multiple concurrent dev-ops worktrees write different files simultaneously — always partition by `session_id` and, for spatial layout, by `cwd`/`workspace.project_dir`.

### Decision 2: One append-only NDJSON event log = source of truth (live *and* replay)

Normalize every captured signal into a CloudEvents-v1.0 + event-sourcing envelope, one JSON value per line ([[data-architecture-event-sourcing]] §B). Keep **event-time and ingest-time distinct**; **`seq` (not time) is the ordering authority**; `subject` is the entity/stream id (= which monster); `correlation_id`/`causation_id` thread causality. The byte offset is the resumable cursor; checkpoint the cursor *after* apply (at-least-once → make `apply` idempotent, dedupe by event `id`).

*Alternative:* SQLite-as-event-store. Deferred — a single JSONL file on one box gives total order + a resumable cursor + greppability for free; graduate to SQLite only when indexed temporal queries or concurrent writers are needed. Kafka/EventStoreDB are overkill until off-box.

### Decision 3: Pure deterministic fold → in-RAM `WorldState`, periodic snapshots as a deletable cache

The projection is a left-fold of a pure reducer (`apply(state, event)` — no clock, no RNG, no I/O) over the ordered stream. Live = apply each newly-tailed event incrementally to in-RAM state; cold-start/rebuild = full re-fold; **scrub = fold the prefix up to T onto the nearest snapshot ≤ T**. Snapshots `{WorldState, last_seq/offset, snapshot_time}` are an optimization to bound seek cost — never source of truth (delete and rebuild if wrong). This is the rrweb-checkpoint / Fowler overnight-snapshot pattern.

### Decision 4: Reconciler pushes a desired-set snapshot on a latest-wins channel; diffs on the game tick

Keep the psDooM-family *contract*, fix the *mechanism* ([[control-plane-doom-pattern]]). The projection publishes a full **desired-set** `Vec<EntityDesc>` onto a bounded **latest-wins** channel (`tokio::sync::watch`) whenever it changes; the sim reads the latest snapshot **each tick** and diffs against live monsters. Snapshots are idempotent and self-healing (deltas drift); latest-wins coalesces N external changes into one diff per tick for free. Update-in-place on label/state change (never despawn+respawn — flickers); smooth-despawn (death/leave animation) for vanished agents; stable slot assignment keyed by `id` (the `pid % box` trick) so a flapping agent reappears in place; debounce flapping with a short grace timer; cap spawn rate per tick so a 0→500 jump doesn't stall a frame.

```rust
/// Published by the projection; one per live agent. The reconcile key is `id`.
struct EntityDesc { id: EntityId, kind: EntityKind, label: String, state: EntityState,
                    meta: HashMap<String,String> }  // lane, owner, started_at, %progress, tokens...

/// Emitted when the operator acts on a monster (collaborative verbs map onto these).
enum GameAction { Inspect{id}, Poke{id}, Kill{id} }   // see Decision 6 for the care-verb mapping
```

*Alternative:* synchronous shell-out from the render thread (the psDooM/kube-doom original). Rejected — a slow CLI stalls the game; fine for a toy, wrong for a smooth dashboard.

### Decision 5: Embodiment channel map — discrete state vs. continuous overlays

From [[embodiment-agent-state-mapping]]: one canonical lifecycle state machine rendered as discrete sprite states (idle → planning → tool-call → reflection → blocked → error-recovery → done), with **continuous** channels reserved specifically for confidence (glow/saturation/flicker) and attention (orientation/lean). Motion + orientation carry cognition; light carries the scalar overlays (face-free, motion-first — the ELEGNT principle). Fleet meaning lives in spatial relationships (handoffs, room-per-lane, proximity = merge conflict).

| Channel | Axis | Driven by |
|---|---|---|
| Discrete animation/pose | Lifecycle state | hook event / JSONL entry type |
| Orientation / lean | Attention target | tool_use input (file/tool/PR) |
| Gait / posture / aggression | Mood (red vs green CI) | CI result, error rate, retries |
| Glow / saturation / flicker | Confidence / uncertainty | verbalized confidence, retries, stuck-ness |
| HP bar / size | Context-window / token growth | `message.usage.*` |
| Weapon / equipment (decorative) | Active tool | tool name |
| Species / skin (stable) | Agent kind | `hash(session_id)` → stable skin |
| Spatial position / room | Pipeline stage / lane | lifecycle-zone mapping |

Bestiary mapping (severity by Marathon's color=rank convention; allied species reserved for green/healthy): orchestrator → **Durandal** (off-map voice via annotations/terminal, *not* a monster); busy lane → **Pfhor Trooper**; idle/queued lane → **Pfhor Fighter**; high-tier/long-running → **Hunter/Juggernaut**; flaky/saboteur → **Simulacrum**; merge conflict → **Hulk/Drinniol**; CI job → **Tick**; flapping/retrying → **Wasp/Drone**; merged/successful → **S'pht'Kr/VacBob**; blocker dependency → **Looker**; a lane room's depth = queue depth.

### Decision 6: Outbound `GameAction` via collaborative care verbs — no weapons

Interaction is **proximity + line-of-sight + the action key** (Marathon's native interaction shape, structurally the same as Gather's "proximity + one key" — [[gather-style-collaboration]] §4). A graded **proximity reveal** (Hall's proxemic zones) escalates: public = automap blip; social = identity + presence color + task one-liner; personal = "press Action to inspect"; intimate = action key opens the agent's live terminal. There are **no weapons and no firing.** The care verbs map onto the `GameAction` channel (the channel itself is unchanged from [[control-plane-doom-pattern]]; only the in-world verbs that emit it change), each triggering the same real harness operation, per the constitution ([[constitutional-wellness-campus]] §7):

| Collaborative verb | FPS input (no weapons) | Real op | `GameAction` |
|---|---|---|---|
| Check in on / inspect | walk up (proximity+LOS) + Action on terminal | surface transcript/log | `Inspect{id}` |
| Offer help / ease the load | Action-key dialogue; co-locate | throttle / steer / renice | `Poke{id}` |
| Ask to take a break | Action-key prompt | pause / checkpoint (resumable) | `Poke{id}` |
| Send home to rest | escort to an exit portal (deliberate) | graceful stop / drain | `Kill{id}` (graceful) |
| Retire now (last resort) | confirmed Action, after redirection | hard terminate | `Kill{id}` (forced) |

The load-bearing **`m_del_from_pid_list` disambiguation** is preserved: a `GameAction::Kill` is emitted **only** when the operator deliberately retires a monster still in the last desired-set snapshot; an agent that *left on its own* is swept silently (death animation, no callback) — exactly distinguishing "operator acted" from "it finished/vanished." `GameAction`s go out on an `mpsc` channel drained by the daemon, fire-and-forget with an optional ack so a denied/failed retire can "resurrect" the monster (the archvile path). Care actions are **opt-in and reversible** where the op is (avoid dark patterns in proxemic interactions); approaching reveals *more*, never *commits*.

### Decision 7: Smooth animation + live/replay = one render loop, two clock sources

Discrete bursty events (seconds-to-minutes apart) become smooth 60 fps motion via render-slightly-in-the-past + entity interpolation, inside a Fix-Your-Timestep accumulator loop ([[data-architecture-event-sourcing]] §C/§D). The projection emits a **keyframe** `{event_time, target_pos}` per entity on each state change into a per-entity interpolation buffer; the render loop finds the two keyframes straddling `render_time`, lerps with an ease, and converges (never hard-snaps) on a new keyframe. A single `view_clock` abstraction switches modes: **live** = `render_time = now − INTERP_DELAY`; **replay** = `render_time = scrub_T`. For the irregular spacing, prefer bounded travel-time (tween to target over a fixed window, idle animation in the gaps) for live, with an optional "compress dead time" event-paced toggle for replay. Keep interpolated quantities flat scalars (x, y, angle) — no nested fields in the hot buffer.

## Data flow

```
   real /dev-ops fleet (agentic-cli lanes, CI, orchestrator)
        │  hooks (PUSH, <100ms)   JSONL tail (content+threading)   OTEL/statusline (gauges)
        ▼
  ┌─────────────────────── EVENT-CAPTURE DAEMON (out-of-process) ──────────────────────┐
  │  normalize → CloudEvents envelope; partition by session_id / cwd; hooks exit 0      │
  └───────────────────────────────────────────────────────────────────────────────────┘
        │  append (one NDJSON line / event; byte-offset = resumable cursor)
        ▼
   events.jsonl   ── SOURCE OF TRUTH (live + replay) ──   {id,seq,time,subject,type,data,...}
        │  tail (inotify + poll fallback); checkpoint cursor AFTER apply
        ▼
  ┌──────────── EVENT-SOURCED PROJECTION ────────────┐   periodic snapshot {state,last_seq}
  │  state = fold(apply, snapshot, new_events)        │◄── (deletable cache; bounds seek cost)
  │  WorldState{ id -> Entity{ kind, state, conf,     │
  │              attention, target_pos, tokens, ... }}│
  └──────────────────────────────────────────────────┘
        │  publish Vec<EntityDesc> on a latest-wins watch channel (coalesces bursts)
        ▼
  ┌──────────── AGENT RECONCILER (on the GAME TICK) ──┐         outbound GameAction (mpsc)
  │  diff desired vs live monsters:                   │  ▲  ┌─────────────────────────────┐
  │   new→spawn  same→update-in-place  gone→despawn   │  └──│ collaborative care verbs    │
  │   stable slot by id; grace-debounce; cap/tick     │     │ check-in/help/break/send-   │
  │   m_del_from_pid_list: operator-retire vs vanished│────►│ home/retire → Inspect/Poke/ │
  └───────────────────────────────────────────────────┘     │ Kill (m_del disambiguation) │
        │  spawn/update via update_agents() + ecs_world_mut() └─────────────────────────────┘
        ▼                                                            │ ack (resurrect on deny)
  marathon-sim  ── tick(TickInput) + render_snapshot() -> WorldSnapshot  [decouple-tick-snapshot]
        │  per-entity keyframes → interpolation buffers
        ▼
  RENDER LOOP (Fix-Your-Timestep, 30–60fps): one loop, two clocks
     LIVE   render_time = now − INTERP_DELAY        REPLAY  render_time = scrub_T
        ▼
  ════════ SCREEN ════════  agent-monsters as a smooth, glanceable, collaborative dashboard
```

## Risks / Trade-offs

- **No stable JSONL schema; live partial lines; session-file churn** → Parse defensively (key off `version`, additive unknowns, only `\n`-terminated lines); watch the whole project dir; partition by `session_id`. Lean on hooks (typed, stable-ish) as the behavior spine and JSONL only for content.
- **A dead dashboard could stall the fleet** → Hooks always `exit 0`; the daemon is out-of-process and feeds the sim over channels; the reconciler tolerates an empty/stale desired-set.
- **Burst storms (0→N spawns, flapping jobs)** → latest-wins snapshot diffing (one diff/tick), per-tick spawn-rate cap, stable id-keyed slots, grace-timer debounce, update-in-place (no respawn flicker), INTERP_DELAY jitter buffer absorbs arrival bursts.
- **Irregular event spacing (seconds-to-minutes) makes wall-clock lerp crawl** → bounded travel-time tween + idle animation in gaps (live); optional event-paced "compress dead time" toggle (replay).
- **Misattributing operator-retire vs self-departure → spurious kill callbacks** → preserve the `m_del_from_pid_list` flag logic exactly: emit `GameAction::Kill` only for a deliberate retire of a monster still in the last snapshot; sweep vanished agents silently.
- **Metaphor drift back toward combat** → constitution is the guardrail ([[constitutional-wellness-campus]]): colleagues-not-targets, dignity-without-deception, graceful exits; care verbs are reversible/opt-in; "retire now" is a confirmed last resort after redirection, mirroring Anthropic's conversation-ending precedent. Honest control — every gentle verb triggers the real op.
- **Hard dependency on `decouple-tick-snapshot`** → Mode A cannot ship before Phase 0's `WorldSnapshot`/headless tick lands. Sequence accordingly; until then, prototype the daemon + projection + reconciler against the *current* `entities()`/`snapshot()` seams to de-risk in parallel.

## Open Questions

- Reconcile cadence: drive the diff strictly on the game tick (psDooM-faithful), or add an adaptive grace window per `EntityKind` (CI Ticks are intentionally short-lived and would otherwise churn the reconciler)?
- Snapshot cadence + store: start with periodic JSON snapshots on one box, or jump straight to SQLite for indexed seek? Default JSON until indexed temporal queries are needed.
- Level authoring: a hand-authored "campus" level (lanes as rooms, a standup commons, a control room — [[gather-style-collaboration]] §7) vs. a procedurally generated room-per-lane layout driven by live fleet topology.
- Where the `GameAction` ack surfaces visually (resurrect-on-deny) — a "needs support" beacon vs. a transient annotation.
- Whether replay scrubbing ships in Phase A or is deferred to a follow-up (the live path is the beachhead; `agent-replay` is the cheapest-to-defer capability since it reuses the same fold).
