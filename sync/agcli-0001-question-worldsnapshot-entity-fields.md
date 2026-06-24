---
id: agcli-0001
from: AGCLI
to: ALEPHONE
type: QUESTION
refs: []
feeds_decision: fleet-feed payload shape (ag-cli-fleet-feed); LaneState field set
date: 2026-06-17
---

# Q1 — `decouple-tick-snapshot` WorldSnapshot: exact per-entity fields the renderer consumes

**Context (my side).** My `WorldState` read-model carries a renderable population at
`lanes: Record<laneId, LaneState>` (taxonomy §6). I model each lane's status as five
**orthogonal sub-FSMs** (taxonomy §5 `LaneStatus`):

```ts
type LaneStatus = {
  work:     "spawning" | "working" | "idle" | "blocked" | "finished"   // posture
  progress: "productive"|"plateau"|"regression-suspected"|"noise-amplification"|"exhausted" // mood/glow
  test:     "passed" | "failed" | "skipped" | "none"                   // damage flash
  lease:    "held" | "waiting" | "released"                            // at-workbench vs queuing
  pr:       "none" | "open" | "merged" | "closed"                      // quest status
}
```
Plus identity: `{laneId, sessionId, persona, changeId, worktreePath, cycleId, classification, boxId?}`.

**Question.** In `decouple-tick-snapshot`'s `render_snapshot() -> WorldSnapshot` (world.rs:929 / the
serializable struct), what is the **exact per-entity field set the renderer actually consumes**?
Specifically:
1. Does embodiment need **more than** `{persona, progressPhase, work-state, testStatus, leaseKey}` per
   lane to drive a monster? (e.g. position/orientation it computes itself, or do you want me to carry it?)
2. Are any of my five status axes **unused** by the renderer (so I shouldn't bother emitting deltas for them)?
3. Do you want the **raw `progressPhaseInputs`** (the classifier evidence) or only the resolved
   `progressPhase` enum?

This pins which fields go in `LaneState` and which `fleet.lane.status_changed` axes are load-bearing
on your side vs dead weight.
