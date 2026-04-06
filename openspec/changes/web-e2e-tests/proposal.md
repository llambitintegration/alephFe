## Why

The `marathon-web` crate is the only deployed artifact (marathon.llambit.io) yet has zero e2e or integration test coverage. A live init error ("Game error: director... file size 20478336") was discovered during manual inspection — the kind of regression that basic browser e2e tests would catch. Every other crate has 12+ integration/e2e tests; the web build is the critical gap.

## What Changes

- Add a Playwright-based e2e test suite that exercises the full browser pipeline: WASM load, data fetch, game init, canvas rendering, and error paths
- Add a Docker Compose configuration to run the web build + test data server locally for reproducible e2e runs
- Add a CI job that builds the WASM target and runs Playwright tests against it
- Add npm/package infrastructure for Playwright test runner in the project

## Capabilities

### New Capabilities
- `web-e2e-testing`: Playwright browser tests covering WASM initialization, data loading pipeline, game startup, canvas rendering, UI overlay, and error handling for the marathon-web crate

### Modified Capabilities

## Impact

- **New files**: Playwright test files, package.json, playwright config, Docker Compose for e2e
- **CI**: New GitHub Actions job for web e2e tests (depends on WASM build)
- **Dependencies**: Node.js + Playwright added as dev/test dependencies (not shipped)
- **Crates affected**: marathon-web (test coverage only, no source changes)
