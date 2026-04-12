## ADDED Requirements

### Requirement: CI Rust test environment includes Marathon 2 data
The CI pipeline SHALL ensure that Marathon 2 scenario data is available when running `cargo test` for crates that have real-data tests (`marathon-formats`, `marathon-viewer`, `marathon-game`).

#### Scenario: Rust test Docker image fetches Marathon 2 data
- **WHEN** the CI Rust test stage builds or runs
- **THEN** the Marathon 2 data files (Map, Shapes, Sounds, Physics Model) SHALL be present in `marathon-formats/tests/fixtures/` before `cargo test` executes

#### Scenario: Data fetch uses the same pinned commit as Dockerfile.e2e
- **WHEN** the CI fetches Marathon 2 data for Rust tests
- **THEN** it SHALL use the same pinned commit hash (`eaf21a7e9f72706c4c2ff9a2960c4367f739f04d`) as `Dockerfile.e2e` to ensure consistent test data
