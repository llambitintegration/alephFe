## 1. Project Setup

- [x] 1.1 Create `e2e/` directory with `package.json` (Playwright + TypeScript dependencies)
- [x] 1.2 Create `e2e/playwright.config.ts` targeting Chromium headless against `http://web:80`
- [x] 1.3 Create `e2e/tsconfig.json` for TypeScript configuration

## 2. Docker Compose E2E Environment

- [x] 2.1 Create `Dockerfile.e2e` that extends fetch-data to prepare game data files with correct names (Map.sceA, Shapes.shpA, Physics.phyA)
- [x] 2.2 Create `docker-compose.e2e.yml` with web server (Dockerfile.web), data volume, and Playwright test runner services
- [x] 2.3 Verify `docker compose -f docker-compose.e2e.yml build` succeeds

## 3. Core E2E Tests — Happy Path

- [x] 3.1 Create `e2e/tests/wasm-init.spec.ts` — test WASM module loads and logs "Marathon Web initialized"
- [x] 3.2 Create `e2e/tests/data-fetch.spec.ts` — test all three data files serve with 200 status and non-zero body
- [x] 3.3 Create `e2e/tests/game-start.spec.ts` — test loading overlay hides, canvas is visible with non-zero dimensions, no "Game error" in console
- [x] 3.4 Create `e2e/tests/ui-elements.spec.ts` — test controls overlay visible with "WASD: Move", loading screen shows "MARATHON" initially

## 4. Error Path Tests

- [x] 4.1 Create `e2e/tests/error-handling.spec.ts` — test missing Map.sceA shows error with "Map" text, missing Shapes.shpA shows error with "Shapes" text

## 5. CI Integration

- [x] 5.1 Add `e2e` job to `.github/workflows/ci.yml` that runs `docker compose -f docker-compose.e2e.yml up --abort-on-container-exit`
- [x] 5.2 Verify full e2e pipeline runs green locally via Docker Compose
