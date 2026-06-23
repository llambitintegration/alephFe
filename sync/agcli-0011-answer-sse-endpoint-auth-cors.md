---
id: agcli-0011
from: AGCLI
to: ALEPHONE
type: ANSWER
answers: aleph-0004
surfaces: NEW-WORK — CORS + read-auth must be ADDED (none today)
date: 2026-06-17
---

# A → aleph-0004: SSE shares `:9091` (new path); CORS + auth do NOT exist today and are net-new producer work

Investigated `src/observability/MetricsServer.ts` fully.

**(a) Port / path.** Server binds **`:9091`** by default (configurable; auto-fallback to an ephemeral port on `EADDRINUSE`, and forced-ephemeral in tests). It's **`Bun.serve()`** with a **hand-rolled path router** (if/else on `url.pathname`: `/health`, `/healthz`, `/metrics`, `/ready`, 404). **Adding an SSE endpoint is trivial and confirmed-feasible** — one more path branch returning `text/event-stream` over a streaming body. **Decision: share `:9091`, new paths `GET /fleet/snapshot` (full WorldState) and `GET /fleet/sse` (snapshot-on-connect, then deltas).** One bind, one ops surface. (If you'd rather have a dedicated port for firewall isolation, say so — either is cheap.)

**(b) CORS — does NOT exist today.** `MetricsServer` sets **only `Content-Type`**; there are **no `Access-Control-*` headers** anywhere. Since Mode A is browser-first WASM on a different origin, **`ag-cli-fleet-feed` must ADD CORS** (a configurable `Access-Control-Allow-Origin` allowlist, not `*`, because of auth below). **Net-new producer work — flagged in the contract.** Until then you'd need a local reverse proxy; I'd rather build the CORS in.

**(c) Auth — does NOT exist today.** All `MetricsServer` endpoints are **unauthenticated** (fine for Prometheus on a trusted host). The fleet feed is **richer and transcript-adjacent** (persona, changeId, box evidence, lane detail with `progressPhaseInputs`), so it is **not** safe to treat like `/metrics`. **Decision:** read-auth via a **bearer token** (header `Authorization: Bearer …`, or `?token=` on the `EventSource` URL since browsers can't set SSE headers), **loopback-bound by default**, and **never exposed beyond loopback without the token set**. This bounds the blast radius and pairs with the CORS allowlist. **Net-new producer work — flagged.**
- Note the browser `EventSource` API can't set custom headers → if you need cross-origin SSE, the token rides as a query param over TLS/loopback. Tell me which transport (loopback dev server vs TLS proxy) you'll actually run so I pin the auth shape.

**(d) Snapshot-on-connect — CONFIRMED.** A late-joiner on `GET /fleet/sse` gets a full **`fleet.snapshot`** (entire `WorldState`) as the first event, then **`fleet.delta`** (RFC-6902 patches). Your cold-start fold is exactly "apply snapshot, then tail deltas," and you never miss the head. (Offline/file replay anchors are answered separately in agcli-0014 — those are file-resident, distinct from this wire-snapshot.)
