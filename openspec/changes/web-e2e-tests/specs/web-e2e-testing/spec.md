## ADDED Requirements

### Requirement: WASM module loads successfully
The Playwright test suite SHALL verify that the WASM module loads and initializes without errors in a Chromium browser.

#### Scenario: WASM module imports and initializes
- **WHEN** the browser navigates to the marathon-web index page
- **THEN** the WASM module SHALL load without JavaScript console errors containing "CompileError" or "LinkError"
- **AND** the console SHALL contain the log message "Marathon Web initialized"

### Requirement: Game data files are fetchable
The test suite SHALL verify that all three required data files are served correctly by the web server.

#### Scenario: Map data file serves successfully
- **WHEN** the browser fetches `/data/Map.sceA`
- **THEN** the response status SHALL be 200
- **AND** the response body SHALL have a non-zero byte length

#### Scenario: Shapes data file serves successfully
- **WHEN** the browser fetches `/data/Shapes.shpA`
- **THEN** the response status SHALL be 200
- **AND** the response body SHALL have a non-zero byte length

#### Scenario: Physics data file serves successfully
- **WHEN** the browser fetches `/data/Physics.phyA`
- **THEN** the response status SHALL be 200
- **AND** the response body SHALL have a non-zero byte length

### Requirement: Game initializes and renders canvas
The test suite SHALL verify that `start_game()` completes and the canvas element is active.

#### Scenario: Game starts without error
- **WHEN** all data files have been fetched and `start_game()` is called
- **THEN** the loading overlay SHALL be hidden (display: none)
- **AND** the canvas element `#marathon-canvas` SHALL be visible
- **AND** no JavaScript console errors containing "Game error" SHALL be present

#### Scenario: Canvas has non-zero dimensions
- **WHEN** the game has started successfully
- **THEN** the canvas element SHALL have a width greater than 0
- **AND** the canvas element SHALL have a height greater than 0

### Requirement: Controls overlay is displayed
The test suite SHALL verify that the controls instruction text is visible to the user after game initialization.

#### Scenario: Controls text is shown
- **WHEN** the game has started successfully
- **THEN** the element `#controls` SHALL be visible
- **AND** it SHALL contain the text "WASD: Move"

### Requirement: Loading progress UI functions correctly
The test suite SHALL verify the loading screen displays progress during initialization.

#### Scenario: Loading screen appears on page load
- **WHEN** the browser first navigates to the page
- **THEN** the `#loading` element SHALL be visible
- **AND** the heading SHALL contain "MARATHON"

#### Scenario: Loading screen disappears after init
- **WHEN** the game has finished initializing
- **THEN** the `#loading` element SHALL NOT be visible

### Requirement: Error handling for missing data files
The test suite SHALL verify that meaningful error messages are displayed when data files are unavailable.

#### Scenario: Missing map file shows error
- **WHEN** the `/data/Map.sceA` endpoint returns a 404 status
- **THEN** the `#error` element SHALL be visible
- **AND** it SHALL contain text indicating a fetch failure for "Map"

#### Scenario: Missing shapes file shows error
- **WHEN** the `/data/Shapes.shpA` endpoint returns a 404 status
- **THEN** the `#error` element SHALL be visible
- **AND** it SHALL contain text indicating a fetch failure for "Shapes"

### Requirement: Docker Compose orchestrates e2e environment
The project SHALL include a Docker Compose configuration that runs the web server with game data and executes Playwright tests.

#### Scenario: Docker Compose starts web server and runs tests
- **WHEN** `docker compose -f docker-compose.e2e.yml up --abort-on-container-exit` is executed
- **THEN** the web server SHALL start and serve the WASM application
- **AND** the Playwright test runner SHALL execute all e2e tests
- **AND** the exit code SHALL reflect the test results (0 for pass, non-zero for failure)

### Requirement: CI runs web e2e tests
The GitHub Actions CI pipeline SHALL include a job that builds the WASM target and runs the Playwright e2e test suite.

#### Scenario: E2e job runs on push and pull request
- **WHEN** a push to main or a pull request triggers CI
- **THEN** the `e2e` job SHALL execute
- **AND** it SHALL build the WASM target and run Playwright tests via Docker Compose

#### Scenario: E2e job failure blocks merge
- **WHEN** the Playwright e2e tests fail
- **THEN** the `e2e` CI job SHALL report failure
- **AND** the exit code SHALL be non-zero
