## Context

The `marathon-web` crate compiles to WASM via `wasm-pack`, is served by nginx (Dockerfile.web), and deployed behind Authelia at marathon.llambit.io. The browser entry point (`index.html`) loads the WASM module, fetches three data files (`Map.sceA`, `Shapes.shpA`, `Physics.phyA`) from `/data/`, then calls `start_game()`. Currently there are zero automated tests for this pipeline — a live init error was caught only by manual browser inspection.

The existing CI (`ci.yml`) runs four Docker-based jobs: test, clippy, fmt, coverage. All target the Rust crate tests. There is no web/WASM CI job.

## Goals / Non-Goals

**Goals:**
- Automated Playwright e2e tests that verify the full browser pipeline: WASM load → data fetch → game init → canvas render → UI elements
- Reproducible local execution via Docker Compose (web server + test data + Playwright)
- CI integration as a new GitHub Actions job
- Error-path coverage (missing data files, corrupt data)

**Non-Goals:**
- Gameplay testing (input simulation, frame-by-frame rendering validation)
- Performance benchmarking or load testing
- Visual regression testing (screenshot comparison)
- Changes to marathon-web source code (test-only change)

## Decisions

### 1. Playwright over wasm-bindgen-test

**Choice**: Playwright (TypeScript) running against the actual served HTML page.

**Why**: `wasm-bindgen-test` tests WASM functions in isolation but can't test the full browser pipeline (HTML loading, JS fetch calls, canvas rendering, nginx serving). The live bug was in the integration between these layers. Playwright tests the deployed artifact as a user would experience it.

**Alternative considered**: Cypress — heavier, slower, and Playwright has better WASM/WebGL support.

### 2. Docker Compose for test orchestration

**Choice**: `docker-compose.e2e.yml` that spins up the nginx web server (from `Dockerfile.web`) with game data mounted, then runs Playwright in a sibling container.

**Why**: Matches the production deployment model exactly. The existing `Dockerfile.web` already builds the WASM and serves via nginx. Docker Compose coordinates the web server + data + test runner without requiring Node.js or wasm-pack on the host.

**Alternative considered**: Running nginx directly on the CI host — less reproducible, diverges from prod config.

### 3. Test data sourced from existing fetch-data stage

**Choice**: Reuse the same Marathon 2 data clone (pinned commit `eaf21a7`) from the main Dockerfile's `fetch-data` stage, mounted into the web container at `/data/`.

**Why**: Consistent with existing test infrastructure. Same data used by Rust integration tests.

### 4. Test file location: `e2e/` at project root

**Choice**: `e2e/` directory at the repo root containing `package.json`, `playwright.config.ts`, and test files.

**Why**: Playwright tests are TypeScript/Node.js, not Rust. Placing them at the root (not inside a Rust crate) keeps the Cargo workspace clean and follows Playwright conventions. The `e2e/` name distinguishes from Rust `tests/` directories.

### 5. Headless Chromium only

**Choice**: Test against Chromium only, headless mode.

**Why**: WebGPU/WebGL support is most mature in Chromium. Marathon-web targets Chrome-family browsers. Multi-browser testing adds CI time with low marginal value for this use case.

## Risks / Trade-offs

- **[WebGPU in headless Chromium]** → Canvas rendering tests may need `--use-gl=angle` or similar flags. Mitigate by testing for canvas element presence and absence of JS errors rather than pixel-level validation.
- **[CI resource usage]** → Docker Compose + Playwright adds ~3-5 min to CI. Mitigate by running in parallel with existing jobs (not blocking them).
- **[Data file size in CI]** → Marathon 2 data (~30MB) must be fetched. Mitigate by reusing the same git clone approach and Docker layer caching.
- **[Flaky network-dependent tests]** → Data fetch tests hit localhost nginx, not external URLs, so network flakiness is minimal.
