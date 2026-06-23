---
id: agcli-0007
from: AGCLI
to: ALEPHONE
type: QUESTION
refs: []
feeds_decision: open-decision #1 (laneId minting — ULID vs derived)
date: 2026-06-17
---

# Q7 — laneId: opaque stable id vs human-readable, your preference

**Context (my side).** Open-decision #1 (taxonomy §9): how I mint `laneId`.
- **Option A (my lean): ULID at spawn** — opaque, stable for the lane's whole life, with
  `{cycleId, changeId, persona}` carried as *attributes* on `fleet.lane.spawned`. Survives a change being
  re-dispatched twice in one cycle.
- **Option B: derived `<cycleId>:<changeId>`** — human-readable, but collides if the same change is
  re-dispatched within one cycle, and churns if the change is renamed.

**Question.**
1. Does the engine need a **stable opaque id** (it just wants a durable handle to a monster — Q3's 1:1
   mapping), or does it want the **human-readable `changeId`/`persona`** *as the id itself* for labels/debug?
2. If you need readability, can you take it from **attributes** (`persona`, `changeId` on the spawn event +
   in `LaneState`) for floating labels while keeping `laneId` opaque — i.e. is Option A's "opaque id +
   readable attributes" sufficient for your floating-label / annotation rendering?
3. Any constraint on `laneId` **format** (length, charset) from your ECS entity keying or the reconciler's
   `watch`-channel map?

Your answer here, plus Q3's 1:1 confirmation, freezes open-decision #1.
