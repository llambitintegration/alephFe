## Why

`marathon-sim` and `marathon-fleet` both depend on `bevy_ecs`, currently pinned
at `0.15`. bevy_ecs 0.16 (released April 2025) introduces three improvements
directly relevant to our codebase:

1. **ECS Relationships** — bidirectional entity-entity links via
   `#[relationship]` components. `marathon-fleet/src/reconciler.rs` currently
   tracks entity-lane linkage in plain `BTreeMap`/`HashMap` structures; ECS
   Relationships would express these more directly and make them queryable
   via the standard ECS query API.

2. **no_std support** — `bevy_ecs` can now be used without the standard
   library, a prerequisite for any future WASM-resident simulation path.

3. **Improved spawn API** — `children!` macro for hierarchy spawning; Observers
   now return `Result` via a unified `BevyError`.

Our usage pattern (direct `world.query_filtered` / `world.resource` /
`world.get` / `world.get_mut` calls — zero `Schedule`/`ScheduleLabel`/system
usage, see `marathon-sim/src/tick.rs:102-514`) is explicitly the
"standalone World" path that bevy_ecs versions preserve across releases.
This minimizes migration risk.

## What Changes

- Bump `bevy_ecs` from `"0.15"` to `"0.16"` in `marathon-sim/Cargo.toml`
  and any other workspace members that transitively pin the version.
- Resolve any compile errors from the bevy_ecs 0.16 API surface (expected:
  minor, primarily around the `BevyError` Observer return type and updated
  derive macro syntax).
- Optionally refactor `marathon-fleet/src/reconciler.rs` entity-lane tracking
  to use ECS Relationships (`#[relationship]` + `RelationshipTarget`) in place
  of the current BTreeMap.

## Capabilities

### Modified Capabilities
- `marathon-sim` simulation loop: unchanged behavior; updated dependency only.
- `marathon-fleet` reconciler (optional): entity-lane linkage modeled as an
  ECS Relationship rather than a manual BTreeMap, enabling standard ECS query
  patterns for lane-entity joins.

### New Capabilities
- `no_std`-compatible bevy_ecs dependency, unblocking any future
  WASM-resident simulation path.

## Impact

- `marathon-sim/Cargo.toml`: `bevy_ecs = "0.15"` → `"0.16"`
- `marathon-fleet/Cargo.toml`: same if directly pinned
- `marathon-sim/src/`: compile-check all `#[derive(Component, Resource)]`
  usages and Observer signatures against 0.16 API; expect O(1) fixes
- `marathon-fleet/src/reconciler.rs`: optional refactor to ECS Relationships
  (scoped to this change or deferred to a follow-on)
- No behavior change to simulation tick, rendering, or format parsing
- All three Docker gates (test/fmt/clippy) must stay green
