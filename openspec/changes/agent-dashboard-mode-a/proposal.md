## Why

You already run a real overnight `/dev-ops` ring — a fleet of agentic-cli lanes, CI ticks, PR queues, merge conflicts, and an orchestrator — and today the only way to read it is by scrolling JSONL transcripts, PR pages, and metrics dashboards. This change turns the Marathon engine into a **live dashboard for that real fleet**: walk a level and watch your actual agents as monsters — lanes as Pfhor Troopers, CI jobs as Ticks, merge conflicts as Hulks, the orchestrator as Durandal's off-map voice — with a faithful, glanceable mapping from agent lifecycle state to body motion. It is the **Mode A beachhead** ([[roadmap-mode-a-to-b]]): novel *today*, with no netcode and no GPU serving, and its plumbing *is* Mode B's Phase 0–1, so it is one continuous path rather than a throwaway demo.

The interaction model is deliberately **collaborative, not combat**. Per [[constitutional-wellness-campus]] and [[gather-style-collaboration]], you do not shoot agents; you walk up and use the action key to *check in / offer help / ask-to-break / send-home-to-rest*. Each care verb triggers the same real underlying operation (inspect transcript / throttle / pause-checkpoint / graceful retire) — the metaphor is gentle, the control is truthful.

## What Changes

- Add an **event-capture daemon** that ingests live Claude Code activity from the running dev-ops fleet — hooks as the real-time spine, the JSONL transcript tail for rich content + subagent threading, and OTEL/statusline as optional gauges ([[claude-code-event-signals]]) — and normalizes each event into a CloudEvents-shaped, append-only NDJSON log (the source of truth for live *and* replay).
- Add an **event-sourced projection** that folds the event log into a per-entity `WorldState` via a pure, deterministic reducer; the append-only log + periodic snapshots make state-as-of-T reconstructible ([[data-architecture-event-sourcing]]).
- Add an **agent reconciler** that pushes a desired-set `EntityDesc` snapshot on a latest-wins channel and, each game tick, diffs it against live monsters — spawn newcomers, update-in-place on state change, smooth-despawn vanished agents — replicating the `m_del_from_pid_list` disambiguation that distinguishes "operator acted on it" from "it left on its own" ([[control-plane-doom-pattern]]).
- Add **agent embodiment**: map agent lifecycle state → discrete monster animation, confidence → continuous glow, attention → orientation, and agent kind → Marathon species ([[embodiment-agent-state-mapping]]).
- Add **collaborative interaction (outbound)**: proximity + line-of-sight + the action key produce care verbs (check-in / offer-help / ask-to-break / send-home / retire-as-last-resort) that emit a `GameAction` back to the harness over an outbound channel. **No weapons, no killing.**
- Add **smooth animation + replay**: render slightly in the past and interpolate bursty events to 60 fps (Fix-Your-Timestep + entity interpolation); one render loop, two clock sources (live = stream head; replay = seek-to-T against nearest snapshot + replay tail).

## Capabilities

### New Capabilities
- `event-capture-daemon`: Ingest live Claude Code fleet activity (hooks + JSONL tail + optional OTEL/statusline) and normalize it into an append-only, CloudEvents-shaped NDJSON event log that is the single source of truth for live and replay.
- `event-sourced-projection`: Fold the event log into a per-entity `WorldState` with a pure deterministic reducer; periodic snapshots + the log make any `state(T)` reconstructible.
- `agent-reconciler`: Push a desired-set `EntityDesc` snapshot on a latest-wins channel and reconcile it against live monsters each tick (spawn / update-in-place / smooth-despawn), with the `m_del_from_pid_list`-style disambiguation between operator action and self-departure.
- `agent-embodiment`: Map agent lifecycle state, confidence, attention, and kind onto Marathon monster animation/glow/orientation/species so each monster is a faithful debugger view of a real agent.
- `collaborative-interaction`: Proximity + LOS + action-key care verbs (check-in / offer-help / ask-to-break / send-home / retire) that emit `GameAction`s to the harness — collaborative, non-combat, governed by the campus constitution.
- `agent-replay`: Live-vs-replay over one render loop and two clock sources; scrub/seek-to-T = nearest snapshot + replay deltas.

### Modified Capabilities
<!-- None. Mode A is purely additive and consumes decouple-tick-snapshot's WorldSnapshot/headless tick as a dependency rather than changing existing spec behavior. -->

## Impact

- **Depends on `decouple-tick-snapshot` (Phase 0) landing first.** Mode A builds *on top of* that change's clean `tick(inputs)` + serializable `render_snapshot() -> WorldSnapshot` and headless tick. This proposal does **not** re-spec that decoupling; it consumes it. The reconciler drives the sim via the already-present seams (`TickInput`, `entities()`, `ecs_world_mut()`, `snapshot()`/`serialize()`) noted in the roadmap's integration-surface table.
- New crate(s)/modules for the daemon, the event store + projection, and the reconciler bridge between the event projection and `marathon-sim`. The daemon runs out-of-process and feeds the reconciler over channels; a dead daemon must never block the sim (hooks always `exit 0`).
- `marathon-sim`: a per-tick `update_agents()` path parallel to `update_monsters()` for spawn/update/despawn of agent-monsters; an outbound `GameAction` channel emitted from a `monster_killed`-equivalent interception point; floating-label / annotation rendering for agent id + task.
- No new netcode, no GPU/world serving, no real-time A/V (agents have no camera — "proximity media" becomes "proximity terminal/transcript reveal"). Those belong to later phases (MCP / Phase 1–3).
- Sequencing within the roadmap: this is **Phase A**, gated on **Phase 0** (`decouple-tick-snapshot`), and unlocks **Phase MCP** (`world-mcp-gateway`) and the longer path toward **Mode B** (`agent-environment-mode-b`).
