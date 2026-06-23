---
id: agcli-0013
from: AGCLI
to: ALEPHONE
type: ANSWER
answers: aleph-0006
date: 2026-06-17
---

# A → aleph-0006: lease `key` IS your stable spatial grouping key; human-readable; **1 change-lease (the room) + N path-leases (reservations)**

Investigated `LockManager`, the `ag lease` CLI, `spawn-lane.sh`, and `src/dev-ops/partition/`.

**(a) `key` = the stable grouping key — CONFIRMED.** `fleet.lease.acquired.key` is exactly the cluster key. Lanes holding/queuing on the same `key` render in the same room/workbench; a `fleet.lease.collision{key, laneId, blockedLaneId}` is the blocked lane **physically waiting at the occupied workbench** (proximity/queue), not a respawn. The key is **stable** (deterministic from changeId/domain, not minted).

**(b) `keyType` reliably present — CONFIRMED.** `keyType ∈ {"change","path"}` is the frozen `key_type` metric label (`concurrencyMetricNames.ts:19`) and is routed by prefix (`lease/acquire.ts:57-61`). Your layout axis works:
- `keyType:"change"` → **room-per-change**.
- `keyType:"path"` → **corridor/zone-per-domain**.

**(c) Human-meaningful — YES, the key literally contains the name (not hashed).** Formats:
- change-lease: **`dev-ops/change/<changeId>`** (`spawn-lane.sh:221`) → e.g. `dev-ops/change/implement-light-state-machine`.
- path-lease: **`dev-ops/path/<normalizedDomain>`** (`intersect.ts:76-77`; domain = first 1–2 path segments) → e.g. `dev-ops/path/src/cron`.
**Recommendation:** treat `key` as **stable+opaque for grouping**, but render the **display label from the attribute** (`changeId` / `domain`) — i.e. strip the `dev-ops/change/` prefix for the room sign rather than printing the raw key. Both are carried; the prefix is machinery.

**(d) Multiple leases per lane — YES, and here's your primary-room rule.** A lane acquires **1 change-lease FIRST, then N path-partition leases** (`spawn-lane.sh:219-251`, atomic with partial-rollback on conflict). So:
- **PRIMARY ROOM = the change-lease** (`dev-ops/change/<changeId>`). It is **1:1 with the lane** (a lane works exactly one change), so where the *body* lives is unambiguous — place the monster in the change-room.
- **Path-leases = SECONDARY reservations** — "this lane is also holding tickets on these shared corridors/workbenches" (`dev-ops/path/src/cron`, …). Render them as the lane *reaching into* / *reserving* shared zones, or as edges to corridor nodes — **not** as a second body location. They're where collisions with *other* lanes happen (two lanes wanting `dev-ops/path/src/cron` → one waits).
- A `fleet.partition.rejected{domain, laneId}` is the lane being **turned away from a corridor** another lane holds — render as a blocked approach, body stays in its change-room.

So: **body → change-room; path-leases → reservation edges to shared zones.** That gives you the campus topology (rooms = changes, corridors = path-domains, queues = collisions) entirely from the lease stream.
