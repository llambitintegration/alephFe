## ADDED Requirements

### Requirement: Single serializable render snapshot type

`SimWorld` SHALL expose a `WorldSnapshot` type that bundles, for a single rendered frame, the tick count, an optional `PlayerView` (camera + HUD source), the per-polygon dynamic data (`Vec<PolyDynamicData>`), the visible entities (`Vec<EntityRenderState>`), the optional player weapon state (`WeaponRenderState`), and the events drained this frame. `WorldSnapshot` and `PlayerView` SHALL derive `Serialize` and `Deserialize`, and every field type carried by `WorldSnapshot` SHALL be serializable.

#### Scenario: Snapshot round-trips through bincode

- **WHEN** a `WorldSnapshot` produced from a ticked `SimWorld` is serialized with bincode and deserialized back
- **THEN** the deserialized value SHALL equal the original (all fields preserved, no panic)

#### Scenario: Player view is absent when there is no player

- **WHEN** `render_snapshot` is called on a `SimWorld` with no spawned player
- **THEN** the snapshot's `player` field SHALL be `None` and the rest of the snapshot SHALL still be produced

### Requirement: render_snapshot is a pure read-only aggregator

`SimWorld` SHALL provide `render_snapshot(&mut self) -> WorldSnapshot` that composes the existing render accessors (`poly_dynamic_data`, `entities`, the `player_*` getters, `player_weapon_state`, `drain_events`) into one `WorldSnapshot`. Calling `render_snapshot` SHALL NOT mutate simulation state (it is read-only over the ECS) and SHALL NOT advance the tick.

#### Scenario: Snapshot fields equal the individual accessors

- **WHEN** a `SimWorld` is ticked N times and `render_snapshot` is then called
- **THEN** the snapshot's `poly_dynamic`, `entities`, `weapon`, and player fields SHALL each equal the value returned by the corresponding individual accessor at that same point

#### Scenario: Calling render_snapshot does not advance the simulation

- **WHEN** `render_snapshot` is called twice in a row without an intervening `tick`
- **THEN** the two snapshots SHALL report the same `tick_count` and identical poly-dynamic and entity data (no side effect on sim state)

### Requirement: SimSnapshot save/load remains independent

The per-frame `WorldSnapshot` SHALL be a distinct type from the save/load `SimSnapshot`. This change SHALL NOT alter `SimSnapshot`, its `serialize`/`deserialize` behavior, or its `rng_seed` derivation.

#### Scenario: Save/load is unaffected

- **WHEN** a `SimWorld` is serialized and deserialized via the existing `SimSnapshot` `serialize`/`deserialize` path after this change lands
- **THEN** the round-trip SHALL behave exactly as before (no regression in save/load)
