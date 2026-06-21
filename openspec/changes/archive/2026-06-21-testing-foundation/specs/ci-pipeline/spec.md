# Spec: ci-pipeline

GitHub Actions CI workflow for automated build, test, lint, format check, and coverage measurement on every push and pull request. The pipeline runs in a Docker container matching the project's build environment (`rust:1.82-slim`) and enforces quality gates that prevent regressions.

---

## ADDED Requirements

### Requirement: GitHub Actions workflow for build and test

A GitHub Actions workflow file SHALL exist at `.github/workflows/ci.yml`. The workflow MUST run on every push to the `main` branch and on all pull requests targeting any branch. The workflow MUST execute inside a Docker container using the project's `rust:1.82-slim` base image.

The workflow MUST include the following stages, executed in order:

| Stage | Command | Purpose |
|-------|---------|---------|
| Build | `cargo build` | Verify the project compiles without errors |
| Test | `cargo test` | Run all unit and integration tests |
| Clippy | `cargo clippy -- -D warnings` | Run the Rust linter with warnings treated as errors |
| Format | `cargo fmt --check` | Verify code formatting matches rustfmt standards |

#### Scenario: PR with all checks passing

WHEN a pull request is opened and the code compiles, all tests pass, clippy reports no warnings, and formatting is correct
THEN all four stages SHALL complete successfully
AND the workflow SHALL report a green/passing status on the pull request

#### Scenario: PR with a failing test

WHEN a pull request is opened and one or more tests fail during the `cargo test` stage
THEN the workflow SHALL report the test failures in the job output
AND the workflow SHALL report a red/failing status on the pull request
AND subsequent stages (clippy, fmt) MAY still run depending on workflow configuration, but the overall status SHALL be failing

#### Scenario: PR with a clippy warning

WHEN a pull request is opened and `cargo clippy -- -D warnings` produces one or more warnings
THEN the clippy stage SHALL fail because `-D warnings` treats warnings as errors
AND the workflow SHALL report a red/failing status on the pull request

#### Scenario: PR with a formatting violation

WHEN a pull request is opened and `cargo fmt --check` detects formatting differences
THEN the format check stage SHALL fail
AND the workflow SHALL report a red/failing status on the pull request

---

### Requirement: Test result reporting

The CI workflow MUST report test results in a way that is visible from the GitHub pull request interface.

#### Scenario: Test results visible in job output

WHEN the `cargo test` stage runs
THEN the test output (including pass/fail counts, test names, and failure details) SHALL be visible in the GitHub Actions job log
AND developers SHALL be able to identify which specific tests failed without downloading artifacts

#### Scenario: Stage failure identification

WHEN any stage fails
THEN the workflow SHALL clearly indicate which stage failed (build, test, clippy, or fmt)
AND the failure message SHALL be visible in the GitHub Actions check summary

---

### Requirement: Coverage measurement with cargo-tarpaulin

The CI workflow MUST include a coverage measurement step using `cargo-tarpaulin`. Coverage MUST be computed after tests pass and MUST enforce a minimum threshold.

The coverage threshold MUST be stored in a file named `.coverage-threshold` at the project root. This file SHALL contain a single numeric value representing the minimum acceptable coverage percentage (e.g., `60`). The threshold is intended to be ratcheted upward as modules are implemented and test coverage increases.

#### Scenario: Coverage above threshold passes

WHEN `cargo-tarpaulin` measures test coverage and the coverage percentage is greater than or equal to the value in `.coverage-threshold`
THEN the coverage stage SHALL pass
AND the coverage percentage SHALL be output to the GitHub Actions job summary

#### Scenario: Coverage below threshold fails

WHEN `cargo-tarpaulin` measures test coverage and the coverage percentage is below the value in `.coverage-threshold`
THEN the coverage stage SHALL fail
AND the failure message SHALL include the actual coverage percentage and the required threshold
AND the workflow SHALL report a red/failing status on the pull request

#### Scenario: Threshold file missing defaults to 0%

WHEN the `.coverage-threshold` file does not exist in the repository
THEN the coverage stage SHALL default the threshold to 0%
AND any non-negative coverage percentage SHALL pass the check
AND the workflow SHALL log a warning that no threshold file was found

#### Scenario: Coverage percentage in job summary

WHEN the coverage stage completes (pass or fail)
THEN the measured coverage percentage SHALL be written to the GitHub Actions job summary (`$GITHUB_STEP_SUMMARY`)
AND the summary SHALL include both the measured percentage and the configured threshold

#### Scenario: Threshold ratcheting

WHEN a contributor increases test coverage and wants to prevent regressions
THEN they SHALL update the numeric value in `.coverage-threshold` to a higher number
AND subsequent CI runs SHALL enforce the new, higher threshold

---

### Requirement: Build caching for CI performance

The CI workflow MUST cache build artifacts and/or dependency downloads to reduce build times on subsequent runs.

The caching strategy MUST include at minimum:

| Cache target | Key strategy |
|-------------|--------------|
| Cargo registry (`~/.cargo/registry`) | Based on `Cargo.lock` hash |
| Cargo build artifacts (`target/`) | Based on `Cargo.lock` hash and source file hashes |

#### Scenario: Second run is faster than first

WHEN a CI workflow runs for the second time with no changes to `Cargo.lock` or source files
THEN the cached cargo registry and build artifacts SHALL be restored
AND the build and test stages SHALL complete faster than the initial uncached run

#### Scenario: Cache invalidation on dependency change

WHEN `Cargo.lock` changes (e.g., a dependency is added or updated)
THEN the cargo registry cache SHALL be invalidated
AND the workflow SHALL download and rebuild dependencies from scratch

#### Scenario: Cache isolation between branches

WHEN CI runs on different branches
THEN each branch SHALL be able to use its own cache
AND the caching strategy SHALL use branch-aware cache keys or fallback keys to maximize cache hits while preventing cross-branch corruption

---

### Requirement: Workflow trigger configuration

The workflow MUST be configured to run on the correct set of events to provide timely feedback without unnecessary runs.

#### Scenario: Push to main triggers CI

WHEN a commit is pushed directly to the `main` branch
THEN the CI workflow SHALL run all stages (build, test, clippy, fmt, coverage)

#### Scenario: Pull request triggers CI

WHEN a pull request is opened, synchronized (new commits pushed), or reopened against any branch
THEN the CI workflow SHALL run all stages

#### Scenario: Docker container environment

WHEN the CI workflow runs
THEN all build and test commands SHALL execute inside a Docker container based on `rust:1.82-slim`
AND the container SHALL have `cargo`, `rustc`, `clippy`, and `rustfmt` available at the versions provided by the `rust:1.82-slim` image
