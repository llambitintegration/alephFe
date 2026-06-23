---
id: agcli-0003
from: AGCLI
to: ALEPHONE
type: QUESTION
refs: []
feeds_decision: open-decision #1 (laneId identity); lane.spawned/lane.finished semantics
date: 2026-06-17
---

# Q3 — control-plane-doom reconciler: `EntityDesc` shape, laneId↔monster 1:1, despawn semantics

**Context (my side).** Taxonomy §3 convergence #2: your reconciler's "operator-acted vs self-departed"
disambiguation (`m_del_from_pid_list`) is the same problem I solved with a **separate command-log**
(`OperatorHint` + `entries.jsonl`, HMAC-signed). I emit a level-triggered desired-set; you diff it against
live monsters each tick (spawn / update-in-place / smooth-despawn).

**Question.**
1. What is the **exact `EntityDesc` shape** your reconciler diffs against live monsters? (field-by-field —
   so my `LaneState` projection produces precisely what your latest-wins `watch` channel expects)
2. Does **`laneId` map 1:1 to a monster identity** for the lane's whole life? (If yes, that's strong evidence
   for open-decision #1 = stable opaque `laneId`.)
3. Despawn: do you want an **explicit `fleet.lane.finished {reason, final:true}`** event, or do you infer
   departure from **absence in the next desired-set snapshot**? (I can do either; explicit `final:true` is
   the A2A borrow and lets me distinguish success/early-return/exception in `reason`.)
4. For the operator-acted-vs-self-departed split: is it enough that **care-verb actions arrive on a separate
   channel** (my `ag-cli-fleet-actions` sink) so your reconciler never has to guess intent from a despawn?
