---
id: agcli-0006
from: AGCLI
to: ALEPHONE
type: QUESTION
refs: []
feeds_decision: ag-cli-fleet-actions intake schema; HMAC-key story for browser signing
date: 2026-06-17
---

# Q6 — GameAction wire shape (care verb leaving the engine) + HMAC-key story

**Context (my side).** Taxonomy §7: a care verb is **not a new auth substrate** — it's an `OperatorHint`
(`origin: human`, HMAC-signed, nonce, provenance in `entries.jsonl`). My intake mapping:

| care verb | real op | substrate |
|---|---|---|
| check-in | inspect transcript (read-only) | no signature needed |
| offer-help | inject OperatorHint into next input envelope | `OperatorHint` |
| ask-to-break | throttle / release slot | `supervisor.releaseSlot` |
| send-home-to-rest | graceful finalize + worktree cleanup | finish-lane path |
| retire | terminate lane | `ActionGuardClass`-gated |

`OperatorHint` shape (my side): `{field, value, origin, signature, nonce, issuedAt, memoryEntryRef?}`.
`ActionGuardClass` ∈ `commit|push|pr-create|pr-merge|deploy|destructive-write`.

**Question.**
1. What does the **`GameAction` look like on the wire** when it leaves the engine
   (`collaborative-interaction` outbound)? Field-by-field — so `ag-cli-fleet-actions` defines a matching
   intake that maps 1:1 onto `OperatorHint` without an impedance layer.
2. **HMAC-key story:** a browser/WASM client signing a care verb needs the HMAC key. Where do you expect that
   key to live, and who signs — (a) the browser holds a key and signs directly, (b) the engine/daemon signs
   on the client's behalf at the boundary, or (c) unsigned `GameAction` → my action server signs as it
   converts to `OperatorHint`? Option (c) keeps the key server-side (my lean) but means the trust boundary is
   the action server, not the browser. Your security model's call.
3. Which care verbs do you consider **destructive enough to gate** behind a confirm (mapping to my
   `ActionGuardClass`)? My lean: `retire`→`destructive-write`-class, `send-home`→`pr`/finalize-class,
   `ask-to-break`→ungated throttle, `check-in`/`offer-help`→ungated.
