# Spec: test-conventions

Project-wide testing standards for the alephone-rust project. Defines test naming conventions, module organization patterns, coverage expectations, and documentation requirements. These conventions ensure consistency across all current and future crates (`marathon-formats`, `marathon-sim`, `marathon-audio`, `marathon-viewer`, `marathon-integration`).

---

## ADDED Requirements

### Requirement: TESTING.md documentation at project root

A file named `TESTING.md` SHALL exist at the project root. This file serves as the canonical reference for how testing works in the project and MUST be kept up to date as testing infrastructure evolves.

The `TESTING.md` file MUST cover the following topics:

| Topic | Description |
|-------|-------------|
| Test naming conventions | The `test_<unit>_<scenario>` naming pattern with examples |
| Test structure pattern | The arrange/act/assert pattern and how to organize test functions |
| Unit vs integration tests | When to use `#[cfg(test)] mod tests` vs `tests/` directory |
| How to use test builders | Examples of `BinaryWriter`, `WadBuilder`, and `MapDataBuilder` usage |
| Coverage expectations | Minimum coverage requirements and how the threshold ratchet works |
| How to run tests locally | Commands for running tests via Docker matching the CI environment |

#### Scenario: TESTING.md exists and is readable

WHEN a contributor clones the repository
THEN a file named `TESTING.md` SHALL exist at the repository root
AND the file SHALL contain sections covering all six topics listed above

#### Scenario: TESTING.md includes naming convention examples

WHEN a contributor reads the naming conventions section of `TESTING.md`
THEN the document SHALL explain the `test_<unit>_<scenario>` pattern
AND the document SHALL include at least three concrete examples from the codebase (e.g., `test_wad_header_too_short`, `test_shape_descriptor_roundtrip`, `test_tag_roundtrip`)

#### Scenario: TESTING.md includes builder usage examples

WHEN a contributor reads the test builders section of `TESTING.md`
THEN the document SHALL include at least one code example showing `BinaryWriter` usage
AND at least one code example showing `WadBuilder` usage
AND explanatory text describing when to use each builder

#### Scenario: TESTING.md includes local execution instructions

WHEN a contributor reads the local execution section of `TESTING.md`
THEN the document SHALL include the exact Docker command for running tests in the same environment as CI
AND the document SHALL include the plain `cargo test` command for running tests without Docker

---

### Requirement: Test naming convention

All test functions across the project MUST follow the naming pattern `test_<unit>_<scenario>`, where `<unit>` identifies the module, type, or function under test and `<scenario>` describes the specific behavior or condition being verified. Names MUST use snake_case.

#### Scenario: Correctly named test from the WAD module

WHEN a test verifies that `WadFile::from_bytes` rejects input shorter than 128 bytes
THEN the test function MUST be named `test_wad_header_too_short` or a similarly descriptive name following the `test_<unit>_<scenario>` pattern

#### Scenario: Correctly named test from the types module

WHEN a test verifies that `ShapeDescriptor::from_parts` followed by accessor methods returns the original values
THEN the test function MUST be named `test_shape_descriptor_roundtrip` or a similarly descriptive name following the `test_<unit>_<scenario>` pattern

#### Scenario: Correctly named test from the tags module

WHEN a test verifies that converting a `WadTag` to `u32` and back yields the original tag
THEN the test function MUST be named `test_tag_roundtrip` or a similarly descriptive name following the `test_<unit>_<scenario>` pattern

#### Scenario: Test name clearly identifies the failure condition

WHEN a test covers an error path or edge case
THEN the scenario portion of the name MUST describe the condition, not just "error" (e.g., `test_wad_invalid_version` rather than `test_wad_error`)

---

### Requirement: Module test organization

Each source module in any crate MUST organize its tests according to the following structure:

- **Unit tests**: Each `.rs` source file MUST contain a `#[cfg(test)] mod tests` block at the bottom of the file for unit tests that test the module's internal behavior.
- **Integration tests**: Cross-module tests and tests that exercise the public API from an external perspective MUST live in the crate's `tests/` directory (e.g., `marathon-formats/tests/`).
- **Test helpers**: Shared test utilities (builders, fixtures, assertion helpers) MUST reside in a `test_helpers` module (e.g., `marathon-formats/src/test_helpers.rs`) gated with `#[cfg(test)]`.

#### Scenario: Unit test in module

WHEN a source file `marathon-formats/src/wad.rs` contains functions to test
THEN the file SHALL contain a `#[cfg(test)] mod tests { ... }` block
AND test functions within that block SHALL have access to private items in the parent module via `use super::*`
AND the test module SHALL be compiled only during test builds due to the `#[cfg(test)]` gate

#### Scenario: Integration test accessing public API

WHEN a test needs to verify end-to-end behavior across multiple modules (e.g., parsing a WAD file and then extracting map geometry from its entries)
THEN the test SHALL be placed in a file under `marathon-formats/tests/` (e.g., `marathon-formats/tests/integration_tests.rs`)
AND the test SHALL only use public API surfaces of the `marathon_formats` crate

#### Scenario: Test helpers accessible from unit and integration tests

WHEN test builder structs (e.g., `BinaryWriter`, `WadBuilder`, `MapDataBuilder`) are defined in `marathon-formats/src/test_helpers.rs`
THEN the module SHALL be gated with `#[cfg(test)]`
AND the module SHALL be declared in `lib.rs` with `#[cfg(test)] pub mod test_helpers;`
AND unit tests within the crate SHALL be able to import helpers via `use crate::test_helpers::*`
AND integration tests SHALL be able to import helpers via `use marathon_formats::test_helpers::*`

#### Scenario: No test code in production builds

WHEN the crate is compiled without the `--cfg test` flag (i.e., a normal release or debug build)
THEN no test helper code, test modules, or test-only dependencies SHALL be included in the compiled output

---

### Requirement: Coverage requirements for modules

Every implemented module MUST have test coverage for all public functions. New modules MUST include tests before they can be merged.

#### Scenario: Module with tests passes review

WHEN a pull request adds a new module (e.g., `marathon-formats/src/map.rs`)
THEN the module MUST include a `#[cfg(test)] mod tests` block containing at least one test for each public function
AND the CI pipeline MUST verify that the overall coverage percentage meets or exceeds the `.coverage-threshold` value

#### Scenario: Module without tests fails CI

WHEN a pull request adds a new module that contains public functions but no `#[cfg(test)] mod tests` block or test functions
THEN the CI coverage check SHALL detect the drop in coverage percentage
AND if the measured coverage falls below the `.coverage-threshold` value, the CI pipeline SHALL fail

#### Scenario: Existing modules maintain coverage

WHEN a pull request modifies an existing module by adding new public functions
THEN the contributor MUST add corresponding tests for the new functions
AND the CI coverage threshold SHALL prevent merging if coverage drops below the configured minimum

#### Scenario: Coverage ratchet prevents regression

WHEN the project's test coverage reaches a new high-water mark
THEN the `.coverage-threshold` file SHOULD be updated to the new value (rounded down to the nearest integer)
AND subsequent pull requests that reduce coverage below this threshold SHALL fail the CI coverage check
