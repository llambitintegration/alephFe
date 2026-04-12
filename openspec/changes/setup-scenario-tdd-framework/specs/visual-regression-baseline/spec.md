## MODIFIED Requirements

### Requirement: Visual regression tests are parameterized over golden levels
The Playwright visual regression suite SHALL extend beyond single-scenario generic checks to per-level screenshot assertions driven by golden values in `tests/scenarios.toml`.

#### Scenario: Per-level screenshot thresholds from manifest
- **WHEN** a golden level with `tier3` values is loaded in the browser
- **THEN** the visual regression test SHALL use the level-specific `min_coverage`, `min_unique_colors`, and `min_quadrants` thresholds from the manifest
- **AND** it SHALL NOT fall back to hardcoded generic thresholds for levels that have manifest entries

#### Scenario: Multiple levels tested in a single Playwright run
- **WHEN** the Playwright e2e suite runs with multiple golden levels having `tier3` values
- **THEN** each level SHALL be tested as a separate Playwright test case
- **AND** test names SHALL include the golden level `id` (e.g., `m2-waterloo-waterpark visual regression`)

### Requirement: Camera position is set per level
For Tier 3 visual regression, the test SHALL position the camera at the specified yaw and pitch before capturing the screenshot.

#### Scenario: Camera positioned at specified yaw and pitch
- **WHEN** a golden level declares `camera_yaw` and `camera_pitch` in its `tier3` values
- **THEN** the Playwright test SHALL set the camera to the specified orientation before capturing the screenshot
- **AND** the screenshot SHALL be taken after a rendering settle period of at least 2 seconds

#### Scenario: Default camera position for levels without explicit values
- **WHEN** a golden level does not declare `camera_yaw` and `camera_pitch`
- **THEN** the test SHALL use the level's spawn-point orientation as the camera position

### Requirement: Existing generic visual regression tests are preserved
The current visual regression tests (non-black coverage, color variety, quadrant coverage) SHALL continue to run as a baseline for the default Marathon 2 level.

#### Scenario: Generic tests still run for Marathon 2 level 0
- **WHEN** the Playwright e2e suite runs
- **THEN** the existing `visual-regression.spec.ts` tests SHALL continue to execute unchanged
- **AND** they SHALL assert >= 20% non-black coverage, >= 50 unique colors, and >= 3/4 quadrants with content

#### Scenario: Per-level tests supplement but do not replace generic tests
- **WHEN** a golden level also has per-level tier3 thresholds
- **THEN** the per-level test SHALL run as an additional test case alongside the generic tests
- **AND** failure of a per-level test SHALL NOT affect the generic test results

### Requirement: Playwright test data includes multi-scenario support
The `Dockerfile.e2e` SHALL provide data for all golden levels with `tier3` values, not just Marathon 2.

#### Scenario: Marathon 1 data available for visual regression
- **WHEN** `Dockerfile.e2e` builds the test data stage
- **THEN** Marathon 1 data files SHALL be available at `/data/marathon-1/` inside the web server container

#### Scenario: Marathon Infinity data available for visual regression
- **WHEN** `Dockerfile.e2e` builds the test data stage
- **THEN** Marathon Infinity data files SHALL be available at `/data/marathon-infinity/` inside the web server container

#### Scenario: Level selection mechanism for browser tests
- **WHEN** a Playwright test wants to load a specific golden level
- **THEN** it SHALL use a URL parameter or WASM API call to specify the scenario and level index
- **AND** the game SHALL load the requested level instead of the default Marathon 2 level 0
