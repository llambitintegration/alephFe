## Context

The `marathon-formats` crate parses six binary/XML formats: WAD, Map, Shapes, Sounds, Physics, MML, and Plugin metadata. The test suite has strong unit coverage (182 tests) using synthetic data built via `BinaryWriter`/`WadBuilder`/`MapDataBuilder`, plus 12 real-data tests in `real_data_tests.rs` that silently skip when fixture files are absent.

Two data sources are available from the sibling `../alephone` project:

1. **Aleph One engine files** (GPL-3.0): MML configs and Plugin.xml files that ship with the engine. Small (~15 KB total), freely redistributable, and exercise MML sections (`<console>`, `<opengl>`, `<interface>`, `<scenario>`) and Plugin.xml attributes (`hud_lua`, `stats_lua`, `theme_dir`, multi-`<scenario>`) that our synthetic fixtures never touch.

2. **Marathon 2 scenario data** (Bungie limited license): Map.sceA (20 MB), Shapes.shpA (10 MB), Sounds.sndA (14 MB), Physics (11 KB). Hosted publicly at `github.com/Aleph-One-Marathon/data-marathon-2.git`. Cannot be committed but can be fetched in CI.

The existing Docker-based CI (`docker build --target test .`) runs all tests. The Dockerfile has a simple `FROM base AS test → cargo test` pipeline.

## Goals / Non-Goals

**Goals:**
- Committed GPL-3.0 fixtures that test MML/Plugin.xml sections with zero CI changes
- CI pipeline that fetches real Marathon 2 binary data and runs existing `real_data_tests.rs` against it
- Snapshot assertions on real data to catch parsing regressions (e.g., known endpoint/polygon counts for level 0)
- Clear separation between committed fixtures (GPL) and fetched data (Bungie license)

**Non-Goals:**
- Marathon 1 or Marathon Infinity data (future phase — this change targets Marathon 2 only)
- Fuzz testing or property-based testing (separate change)
- Modifying the parsers themselves — this is purely additive test infrastructure
- Setting a coverage threshold (separate concern)

## Decisions

### 1. Committed GPL fixtures go in `tests/fixtures/alephone/`

**Decision**: Copy Aleph One engine MML and Plugin.xml files into a new `tests/fixtures/alephone/` subdirectory, committed to the repo.

**Rationale**: These are GPL-3.0, tiny (~15 KB), and stable (they change rarely in the Aleph One repo). Committing them means tests run immediately with no network dependency. The `alephone/` subdirectory makes the provenance clear and keeps them separate from our hand-crafted `sample.mml`.

**Alternative considered**: Symlink to `../alephone/data/` — rejected because it couples our repo layout to the sibling checkout and breaks in CI where the alephone repo isn't present.

### 2. CI fetches Marathon 2 data via shallow git clone in a Docker stage

**Decision**: Add a new Dockerfile stage `fetch-data` that does a `git clone --depth 1` of the Marathon 2 data repo and copies the relevant files into `tests/fixtures/`.

```
FROM base AS fetch-data
RUN apt-get update && apt-get install -y --no-install-recommends git ca-certificates
RUN git clone --depth 1 https://github.com/Aleph-One-Marathon/data-marathon-2.git /tmp/m2-data
RUN cp /tmp/m2-data/Map.sceA ./marathon-formats/tests/fixtures/Map.sceA \
 && cp /tmp/m2-data/Shapes.shpA ./marathon-formats/tests/fixtures/Shapes \
 && cp /tmp/m2-data/Sounds.sndA ./marathon-formats/tests/fixtures/Sounds \
 && cp "/tmp/m2-data/Physics Models/Standard.phyA" "./marathon-formats/tests/fixtures/Physics Model"

FROM fetch-data AS test
RUN cargo test 2>&1
```

**Rationale**: Shallow clone minimizes download (~50 MB) and avoids committing copyrighted binary data. The Dockerfile stays self-contained — no external scripts or CI workflow changes needed. The existing `FROM base AS test` stage just gets rebased onto `fetch-data`.

**Alternative considered**: Download a tarball or use GitHub API — rejected because the data repo uses Git LFS for binaries and a proper git clone handles that correctly. Also considered fetching in CI workflow YAML before Docker build — rejected because it would require mounting data into the Docker context, complicating the build.

### 3. Real-data tests use the existing skip-if-absent pattern

**Decision**: Keep the existing `fixture()` helper that returns `None` when files are absent. Tests skip gracefully when run locally without data. No `#[ignore]` attributes or feature flags.

**Rationale**: This pattern is already established in `real_data_tests.rs` and works well. Local developers without data see `SKIP:` messages. CI (with the `fetch-data` stage) runs everything. No configuration needed.

### 4. Snapshot assertions use hardcoded expected values

**Decision**: Add assertions like `assert_eq!(map.endpoints.len(), 347)` for known Marathon 2 level data, with comments noting which level the values come from.

**Rationale**: These are simple, readable, and catch regressions immediately. If a parser change silently reinterprets a field, the count changes and the test fails. The values are derived from a one-time parsing pass against the real data.

**Alternative considered**: Golden file comparison (serialize parsed output → compare to committed JSON) — overkill for now, and requires choosing a serialization format. Hardcoded assertions are sufficient for this phase.

### 5. New GPL fixture tests go in `real_data_tests.rs` alongside existing tests

**Decision**: Add new test functions for the Aleph One MML and Plugin.xml files in the existing `real_data_tests.rs` file, since they share the same `fixtures_dir()` helper and testing pattern.

**Rationale**: Keeps all fixture-based tests in one place. The GPL fixtures always exist (committed), so these tests never skip — but they conceptually belong with the other "real file" tests rather than the synthetic `integration_tests.rs`.

## Risks / Trade-offs

**[Network dependency in CI] → Mitigation**: The `git clone` could fail if GitHub is down or the repo is moved. Mitigation: shallow clone is fast (~30s), and the Aleph One data repos have been stable since 2011. If needed, we can cache the clone in a Docker layer or CI cache.

**[CI build time increase] → Mitigation**: The ~50 MB clone adds ~30-60s to CI. Acceptable for the coverage gain. Docker layer caching means subsequent builds skip the clone if the Dockerfile stage hasn't changed.

**[Bungie licensing ambiguity] → Mitigation**: We never commit the binary data. It's fetched at CI time from the same public repo that Aleph One's own CI uses, under the same Bungie distribution license. The README in the data repo documents the licensing situation.

**[Marathon 2 data could change upstream] → Mitigation**: We pin to a specific commit hash in the git clone command rather than using HEAD. This ensures snapshot assertions remain stable. We can bump the pin intentionally when needed.
